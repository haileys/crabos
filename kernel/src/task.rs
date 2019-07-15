use core::sync::atomic::AtomicU64;

use alloc_collections::btree_map::BTreeMap;

use crate::interrupt::TrapFrame;
use crate::mem::MemoryExhausted;
use crate::mem::kalloc::GlobalAlloc;
use crate::page::{self, PageCtx};
use crate::sync::{Arc, Mutex};

pub const SEG_UCODE: u16 = 0x1b;
pub const SEG_UDATA: u16 = 0x23;

static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

pub struct Tasks {
    map: BTreeMap<TaskId, TaskRef, GlobalAlloc>,
    current: TaskRef,
}

static TASKS: Mutex<Option<Tasks>> = Mutex::new(None);

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq, Ord)]
pub struct TaskId(pub u64);

pub type TaskRef = Arc<Mutex<Task>>;

pub struct Task {
    id: TaskId,
    frame: TrapFrame,
    page_ctx: PageCtx,
    // parent: Arc<Task>,
}

pub unsafe fn start() -> Result<!, MemoryExhausted> {
    let mut frame = TrapFrame::new(0x1_0000_0000, 0x0);

    let mut task_map = BTreeMap::new();

    let init = Arc::new(Mutex::new(Task {
        id: TaskId(1),
        frame: frame.clone(),
        page_ctx: page::current_ctx(),
    }))?;

    task_map.insert(TaskId(1), init.clone())
        .map_err(|_| MemoryExhausted);

    let second = Arc::new(Mutex::new(Task {
        id: TaskId(2),
        frame: TrapFrame::new(0x1_0000_1000, 0x0),
        page_ctx: page::current_ctx(),
    }))?;

    task_map.insert(TaskId(2), second)
        .map_err(|_| MemoryExhausted);

    *TASKS.lock() = Some(Tasks {
        map: task_map,
        current: init,
    });

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
        let mut current_locked = tasks.current.lock();
        current_locked.frame = frame.clone();
        current_locked.id
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

    tasks.current = new_task.clone();

    // restore new task context
    *frame = new_task.lock()
        .frame
        .clone();
}
