use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicU64, Ordering};
use core::task::{Poll, Context, Waker, RawWaker, RawWakerVTable};

use alloc_collections::boxed::Box;
use alloc_collections::btree_map::BTreeMap;

use crate::interrupt::TrapFrame;
use crate::mem::kalloc::GlobalAlloc;
use crate::mem::MemoryExhausted;
use crate::object::ObjectRef;
use crate::page::{self, PageCtx};
use crate::sync::{Arc, Mutex};
use crate::syscall;
use crate::util::EarlyInit;

pub const SEG_UCODE: u16 = 0x1b;
pub const SEG_UDATA: u16 = 0x23;

pub type TaskMap<V> = EarlyInit<Mutex<BTreeMap<TaskId, V, GlobalAlloc>>>;

static TASKS: TaskMap<Task> = TaskMap::new();
static TASK_STATES: TaskMap<TaskState> = TaskMap::new();
static TASK_FUTURES: TaskMap<TaskFuture> = TaskMap::new();

pub fn init() {
    EarlyInit::set(&TASKS, Mutex::new(BTreeMap::new()));
    EarlyInit::set(&TASK_STATES, Mutex::new(BTreeMap::new()));
    EarlyInit::set(&TASK_FUTURES, Mutex::new(BTreeMap::new()));
}

static CURRENT_TASK: Mutex<Option<TaskId>> = Mutex::new(None);

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq, Ord)]
pub struct TaskId(pub u64);

#[derive(Debug)]
pub enum TaskState {
    SyscallEntry(TrapFrame),
    Wake,
    Sleep,
    User(TrapFrame),
}

type TaskFuture = Arc<Mutex<Pin<Box<dyn Future<Output = ()>, GlobalAlloc>>>>;

#[derive(Debug)]
pub struct Task {
    id: TaskId,
    page_ctx: ObjectRef<PageCtx>,
}

fn alloc_task_id() -> TaskId {
    static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);
    TaskId(NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst))
}

pub fn spawn<F, Fut>(page_ctx: ObjectRef<PageCtx>, f: F) -> Result<TaskId, MemoryExhausted>
    where F: FnOnce(TaskEmbryo) -> Fut, Fut: Future<Output = ()> + 'static
{
    let id = alloc_task_id();

    let state = TaskState::Wake;

    let future = {
        let future = Box::new(f(TaskEmbryo { task_id: id }))
            .map_err(|_| MemoryExhausted)?;

        let future_obj = future as Box<dyn Future<Output = ()>, GlobalAlloc>;

        // TODO - why doesn't Pin::new work?
        unsafe { Pin::new_unchecked(future_obj) }
    };

    let task = Task { id, page_ctx };

    // try inserting all task related data:
    let result: Result<_, MemoryExhausted> = (|| {
        TASK_STATES.lock().insert(id, state)
            .map_err(|_| MemoryExhausted)?;

        TASK_FUTURES.lock().insert(id, Arc::new(Mutex::new(future))?)
            .map_err(|_| MemoryExhausted)?;

        TASKS.lock().insert(id, task)
            .map_err(|_| MemoryExhausted)?;

        Ok(())
    })();

    // roll back inserts if any error:
    match result {
        Ok(()) => Ok(id),
        Err(_) => {
            TASKS.lock().remove(&id);
            TASK_FUTURES.lock().remove(&id);
            TASK_STATES.lock().remove(&id);
            Err(MemoryExhausted)
        }
    }
}

pub fn current() -> TaskId {
    CURRENT_TASK.lock()
        .expect("task::current called with no current task")
}

pub fn get_page_ctx() -> ObjectRef<PageCtx> {
    TASKS.lock()
        .get(&current())
        .expect("task::get_page_ctx called with no current task")
        .page_ctx
        .clone()
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
    fn save_current_task(frame: &mut TrapFrame) -> Option<TaskId> {
        let current = (*CURRENT_TASK.lock())?;

        let mut task_states = TASK_STATES.lock();

        let state = task_states
            .get_mut(&current)
            .expect("task id not in TASK_STATES");

        match *state {
            TaskState::User(ref mut task_frame) => {
                *task_frame = frame.clone();
            }
            _ => {}
        }

        Some(current)
    }

    enum WorkItem {
        Kernel(TaskFuture),
        User(TrapFrame),
    }

    fn find_next_work_item(previous_task_id: Option<TaskId>) -> (TaskId, WorkItem) {
        let tasks = TASKS.lock();

        let previous_task_id = previous_task_id.unwrap_or(TaskId(0));

        let next_tasks = tasks.range(previous_task_id..)
            .filter(|(id, _)| **id != previous_task_id)
            .chain(tasks.range(..=previous_task_id));

        for (id, _) in next_tasks {
            let mut task_states = TASK_STATES.lock();

            let state = task_states.get_mut(&id)
                .expect("id not in TASK_STATES");

            let work_item = match *state {
                TaskState::Sleep => {
                    continue;
                }
                TaskState::SyscallEntry(_) | TaskState::Wake => {
                    let future = TASK_FUTURES.lock()
                        .get(&id)
                        .cloned()
                        .expect("id not in TASK_FUTURES");

                    WorkItem::Kernel(future)
                }
                TaskState::User(ref task_frame) => {
                    WorkItem::User(task_frame.clone())
                }
            };

            return (*id, work_item);
        }

        panic!("there should always be a task ready to run!");
    }

    let mut previous_task_id = save_current_task(frame);

    loop {
        let (task_id, work_item) = find_next_work_item(previous_task_id);

        *CURRENT_TASK.lock() = Some(task_id);

        let page_ctx = TASKS.lock()
            .get(&task_id)
            .expect("current task in TASKS")
            .page_ctx
            .clone();

        page::set_ctx(page_ctx.object().clone());

        match work_item {
            WorkItem::Kernel(future) => {
                let waker = Waker::from_raw(task_waker_new(task_id));
                let mut cx = Context::from_waker(&waker);
                let mut fut = future.lock();

                match fut.as_mut().poll(&mut cx) {
                    Poll::Ready(()) => panic!("task finished!"),
                    Poll::Pending => {
                        // TODO set task state to sleep
                    }
                }

                previous_task_id = Some(task_id);
            }
            WorkItem::User(task_frame) => {
                *frame = task_frame;
                return;
            }
        }
    }
}

pub unsafe fn dispatch_syscall(frame: &mut TrapFrame) {
    {
        let current_task = CURRENT_TASK.lock()
            .expect("no current task for syscall entry");

        let mut task_states = TASK_STATES.lock();

        let task_state = task_states.get_mut(&current_task)
            .expect("current task in TASK_STATES");

        match task_state {
            TaskState::User(_) => {
                // ok
            }
            _ => {
                panic!("syscall arrived from kernel context! task state: {:?}", task_state);
            }
        }

        *task_state = TaskState::SyscallEntry(frame.clone());
    }

    // TODO don't switch immediately but process syscall on this task first:
    switch(frame)
}

static TASK_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    task_waker_clone,
    task_waker_wake,
    task_waker_wake_by_ref,
    task_waker_drop,
);

fn task_waker_new(task_id: TaskId) -> RawWaker {
    RawWaker::new(task_id.0 as *const (), &TASK_WAKER_VTABLE)
}

unsafe fn task_waker_clone(data: *const ()) -> RawWaker {
    RawWaker::new(data, &TASK_WAKER_VTABLE)
}

unsafe fn task_waker_wake(data: *const ()) {
    let task_id = TaskId(data as u64);

    if let Some(state) = TASK_STATES.lock().get_mut(&task_id) {
        *state = TaskState::Wake;
    }
}

unsafe fn task_waker_wake_by_ref(data: *const ()) {
    task_waker_wake(data);
}

unsafe fn task_waker_drop(_data: *const ()) {}

pub struct TaskEmbryo {
    task_id: TaskId,
}

impl TaskEmbryo {
    pub fn setup(self, trap_frame: TrapFrame) -> TaskRun {
        TaskRun {
            task_id: self.task_id,
            trap_frame: trap_frame,
        }
    }
}

pub struct TaskRun {
    task_id: TaskId,
    trap_frame: TrapFrame,
}

impl TaskRun {
    pub fn run(&mut self) -> TaskResume {
        let mut task_states = TASK_STATES.lock();

        let task_state = task_states.get_mut(&self.task_id)
            .expect("id not in TASK_STATES");

        *task_state = TaskState::User(self.trap_frame.clone());

        TaskResume { task_run: self }
    }

    pub fn trap_frame(&mut self) -> &mut TrapFrame {
        &mut self.trap_frame
    }

    pub async fn run_loop(&mut self) -> ! {
        loop {
            match self.run().await {
                Trap::Syscall => {
                    syscall::dispatch(self.trap_frame()).await;
                }
            }
        }
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

    fn poll(mut self: Pin<&mut Self>, _cx: &mut core::task::Context) -> Poll<Self::Output> {
        let mut task_states = TASK_STATES.lock();

        let task_state = task_states.get_mut(&self.task_run.task_id)
            .expect("id not in TASK_STATES");

        let (trap, frame) = match *task_state {
            TaskState::SyscallEntry(ref frame) => (Trap::Syscall, frame.clone()),
            TaskState::Wake => return Poll::Pending,
            TaskState::User(_) => return Poll::Pending,
            TaskState::Sleep => panic!("task state should not be Sleep"),
        };

        self.task_run.trap_frame = frame;
        Poll::Ready(trap)
    }
}
