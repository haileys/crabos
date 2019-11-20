use alloc_collections::btree_map::BTreeMap;

use crate::interrupt::TrapFrame;
use crate::mem::MemoryExhausted;
use crate::mem::kalloc::GlobalAlloc;
use crate::page::{self, PageCtx};
use crate::sync::{Arc, Mutex};

pub const SEG_UCODE: u16 = 0x1b;
pub const SEG_UDATA: u16 = 0x23;

static TASKS: Mutex<Option<Tasks>> = Mutex::new(None);

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq, Ord)]
pub struct TaskId(pub u64);

pub type TaskRef = Arc<Mutex<Task>>;

pub struct Tasks {
    map: BTreeMap<TaskId, TaskRef, GlobalAlloc>,
    current: Option<TaskRef>,
    next_id: u64,
}

pub struct Task {
    id: TaskId,
    frame: TrapFrame,
    page_ctx: PageCtx,
    // parent: Arc<Task>,
}

impl Tasks {
    pub fn create(&mut self, frame: TrapFrame, page_ctx: PageCtx)
        -> Result<TaskRef, MemoryExhausted>
    {
        let id = TaskId(self.next_id);
        self.next_id += 1;

        let task = Arc::new(Mutex::new(Task {
            id,
            frame,
            page_ctx,
        }))?;

        self.map.insert(id, task.clone())
            .map_err(|_| MemoryExhausted)?;

        Ok(task)
    }
}

pub unsafe fn start() -> Result<!, MemoryExhausted> {
    let mut frame = TrapFrame::new(0x1_0000_0000, 0x0);

    let mut tasks = Tasks {
        map: BTreeMap::new(),
        current: None,
        next_id: 1,
    };

    let init = tasks.create(frame.clone(), page::current_ctx())?;

    let second = tasks.create(TrapFrame::new(0x1_0000_1000, 0x0),
        page::current_ctx())?;

    tasks.current = Some(init);

    *TASKS.lock() = Some(tasks);

    asm!("
        movq $0, %rsp
        jmp interrupt_return
    " :: "r"(&mut frame as *mut TrapFrame) :: "volatile");

    loop {}
}

pub unsafe fn switch(frame: &mut TrapFrame) {
    let mut tasks = TASKS.lock();

    let tasks = tasks
        .as_mut()
        .expect("TASKS is not Some");

    // save old context
    let current_id = {
        match tasks.current {
            Some(ref task) => {
                let mut task = task.lock();
                task.frame = frame.clone();
                task.id
            }
            None => {
                TaskId(0)
            }
        }
    };

    // select new task to run
    let new_task = tasks.map.iter()
        .filter(|(id, _)| **id > current_id)
        .min_by_key(|(id, _)| *id)
        .or_else(|| {
            tasks.map.iter().min_by_key(|(id, _)| *id)
        })
        .map(|(_, task)| task)
        .cloned()
        .expect("no task to run!");

    tasks.current = Some(new_task.clone());

    // restore new task context
    *frame = new_task.lock()
        .frame
        .clone();
}
