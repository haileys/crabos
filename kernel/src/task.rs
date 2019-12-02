use core::future::{self, Future};
use core::marker::PhantomData;
use core::mem;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::{AtomicU64, Ordering};
use core::task::{Poll, Context, Waker, RawWaker, RawWakerVTable};

use alloc_collections::boxed::Box;
use alloc_collections::btree_map::BTreeMap;

use crate::interrupt::TrapFrame;
use crate::mem::MemoryExhausted;
use crate::mem::fault::Flags;
use crate::mem::kalloc::GlobalAlloc;
use crate::page::{self, PageCtx};
use crate::sync::{Arc, Mutex};
use crate::util::EarlyInit;

pub const SEG_UCODE: u16 = 0x1b;
pub const SEG_UDATA: u16 = 0x23;

static TASKS: EarlyInit<Mutex<BTreeMap<TaskId, TaskRef, GlobalAlloc>>> = EarlyInit::new();

static CURRENT_TASK: Mutex<Option<TaskRef>> = Mutex::new(None);

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq, Ord)]
pub struct TaskId(pub u64);

pub type TaskRef = Arc<Mutex<Task>>;

pub struct Tasks {
    current: Option<TaskRef>,
}

#[derive(Debug)]
pub enum TaskState {
    SyscallEntry(TrapFrame),
    Wake,
    Sleep,
    User(TrapFrame),
}

type TaskFuture = Arc<Mutex<Pin<Box<dyn Future<Output = ()>, GlobalAlloc>>>>;

pub struct Task {
    id: TaskId,
    page_ctx: PageCtx,
    state: Arc<Mutex<TaskState>>,
    future: TaskFuture,
}

pub fn init() {
    EarlyInit::set(&TASKS, Mutex::new(BTreeMap::new()));
}

fn alloc_task_id() -> TaskId {
    static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);
    TaskId(NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst))
}

pub fn spawn<F, Fut>(page_ctx: PageCtx, f: F) -> Result<TaskRef, MemoryExhausted>
    where F: FnOnce(TaskEmbryo) -> Fut, Fut: Future<Output = ()> + 'static
{
    let task_state = Arc::new(Mutex::new(TaskState::Wake))?;

    let future = Box::new(f(TaskEmbryo {
        task_state: task_state.clone(),
    })).map_err(|_| MemoryExhausted)?;

    let future_obj = future as Box<dyn Future<Output = ()>, GlobalAlloc>;

    // TODO - why doesn't Pin::new work?
    let future_pin = unsafe { Pin::new_unchecked(future_obj) };

    let id = alloc_task_id();

    let task = Arc::new(Mutex::new(Task {
        id: id,
        page_ctx: PageCtx::new()?,
        state: task_state,
        future: Arc::new(Mutex::new(future_pin))?,
    }))?;

    TASKS
        .lock()
        .insert(id, task.clone())
        .map_err(|_| MemoryExhausted)?;

    Ok(task)
}

pub unsafe fn start() -> ! {
    let mut frame = TrapFrame::new(0, 0);
    switch(&mut frame);
    asm!("
        movq $0, %rsp
        jmp interrupt_return
    " :: "r"(&mut frame as *mut TrapFrame) :: "volatile");

    unreachable!()
}

pub unsafe fn switch(frame: &mut TrapFrame) {
    fn save_current_task(frame: &mut TrapFrame) -> TaskId {
        let mut tasks = TASKS.lock();

        let current = CURRENT_TASK.lock();

        // save old context
        match *current {
            Some(ref task) => {
                let task = task.lock();
                let mut state = task.state.lock();
                match *state {
                    TaskState::User(ref mut task_frame) => {
                        *task_frame = frame.clone()
                    }
                    _ => {}
                }
                task.id
            }
            None => {
                TaskId(0)
            }
        }
    }

    enum WorkItem {
        Kernel(TaskFuture),
        User(TrapFrame),
    }

    fn find_next_work_item(previous_task_id: TaskId) -> (TaskId, WorkItem) {
        let mut tasks = TASKS.lock();

        let next_tasks = tasks.range(previous_task_id..)
            .skip(1) // skip first task, it will always be `current_id`
            .chain(tasks.range(..=previous_task_id));

        for (id, task) in next_tasks {
            let task_locked = task.lock();
            let state = task_locked.state.lock();

            match *state {
                TaskState::Sleep => {
                    continue;
                }
                TaskState::SyscallEntry(_) | TaskState::Wake => {
                    *CURRENT_TASK.lock() = Some(task.clone());

                    return (*id, WorkItem::Kernel(Arc::clone(&task_locked.future)));
                }
                TaskState::User(ref task_frame) => {
                    *CURRENT_TASK.lock() = Some(task.clone());

                    return (*id, WorkItem::User(task_frame.clone()));
                }
            }
        }

        panic!("there should always be a task ready to run!");
    }

    let mut previous_task_id = save_current_task(frame);

    loop {
        match find_next_work_item(previous_task_id) {
            (new_task_id, WorkItem::Kernel(future)) => {
                let waker = Waker::from_raw(RawWaker::new(ptr::null(), &RAW_WAKER_VTABLE));
                let mut cx = Context::from_waker(&waker);
                let mut fut = future.lock();

                match fut.as_mut().poll(&mut cx) {
                    Poll::Ready(()) => panic!("task finished!"),
                    Poll::Pending => {}
                }

                previous_task_id = new_task_id;
            }
            (_, WorkItem::User(task_frame)) => {
                *frame = task_frame;
                return;
            }
        }
    }
}

pub unsafe fn dispatch_syscall(frame: &mut TrapFrame) {
    {
        let mut current_task = CURRENT_TASK.lock();

        let mut current_task = current_task
            .as_mut().expect("CURRENT_TASK is Some")
            .lock();

        let previous_state = mem::replace(
            &mut *current_task.state.lock(),
            TaskState::SyscallEntry(frame.clone()));

        match previous_state {
            TaskState::User(_) => { /* ok */ }
            _ => {
                panic!("syscall arrived from kernel context! task state: {:?}", previous_state);
            }
        }
    }

    // TODO don't switch immediately but process syscall on this task first:
    switch(frame)
}

static RAW_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    waker_clone,
    waker_wake,
    waker_wake_by_ref,
    waker_drop,
);

unsafe fn waker_clone(_waker: *const ()) -> RawWaker {
    panic!("waker_clone");
}

unsafe fn waker_wake(_waker: *const ()) {
    panic!("waker_wake");
}

unsafe fn waker_wake_by_ref(_waker: *const ()) {
    panic!("waker_wake_by_ref");
}

unsafe fn waker_drop(_waker: *const ()) {}

pub struct TaskEmbryo {
    task_state: Arc<Mutex<TaskState>>,
}

impl TaskEmbryo {
    pub fn setup(self, trap_frame: TrapFrame) -> TaskRun {
        TaskRun {
            task_state: self.task_state,
            trap_frame: trap_frame,
        }
    }
}

pub struct TaskRun {
    task_state: Arc<Mutex<TaskState>>,
    trap_frame: TrapFrame,
}

impl TaskRun {
    pub fn run(&mut self) -> TaskResume {
        *self.task_state.lock() = TaskState::User(self.trap_frame.clone());

        TaskResume { task_run: self }
    }

    pub fn trap_frame(&mut self) -> &mut TrapFrame {
        &mut self.trap_frame
    }
}

pub enum Trap {
    Syscall,
}

pub struct TaskResume<'a> {
    task_run: &'a mut TaskRun,
}

impl<'a> Future for TaskResume<'a> {
    type Output = Trap;

    fn poll(mut self: Pin<&mut Self>, cx: &mut core::task::Context) -> Poll<Self::Output> {
        let (trap, frame) = match *self.task_run.task_state.lock() {
            TaskState::SyscallEntry(ref frame) => (Trap::Syscall, frame.clone()),
            TaskState::Wake => return Poll::Pending,
            TaskState::User(_) => return Poll::Pending,
            TaskState::Sleep => panic!("task state should not be Sleep"),
        };

        self.task_run.trap_frame = frame;
        Poll::Ready(trap)
    }
}
