use crate::interrupt::TrapFrame;
use crate::page::{self, Pml4};
use crate::sync::Mutex;

pub const SEG_UCODE: u16 = 0x1b;
pub const SEG_UDATA: u16 = 0x23;

pub struct Task {
    frame: TrapFrame,
    pml4: Pml4,
}

pub struct Tasks {
    tasks: [Option<Task>; 8],
    current: usize,
}

impl Tasks {
    pub const fn null() -> Self {
        Tasks {
            tasks: [None, None, None, None, None, None, None, None],
            current: 0,
        }
    }

    pub fn current(&self) -> &Task {
        self.tasks[self.current]
            .as_ref()
            .expect("current task to be Some")
    }

    pub fn current_mut(&mut self) -> &mut Task {
        self.tasks[self.current]
            .as_mut()
            .expect("current task to be Some")
    }
}

static TASKS: Mutex<Tasks> = Mutex::new(Tasks::null());

pub unsafe fn start() -> ! {
    let mut frame = TrapFrame::new(0x1_0000_0000, 0x0);

    {
        let mut tasks = TASKS.lock();

        tasks.tasks[0] = Some(Task {
            frame: frame.clone(),
            pml4: page::pml4(),
        });

        tasks.tasks[1] = Some(Task {
            frame: TrapFrame::new(0x1_0000_1000, 0x0),
            pml4: page::pml4(),
        })
    }

    asm!("
        movq $0, %rsp
        jmp interrupt_return
    " :: "r"(&mut frame as *mut TrapFrame) :: "volatile");

    loop {}
}

pub unsafe fn switch(frame: &mut TrapFrame) {
    let mut tasks = TASKS.lock();

    // save old context
    tasks.current_mut().frame = frame.clone();

    // select new task to run
    let mut new_task_id = tasks.current;

    let new_task_id = loop {
        new_task_id = (new_task_id + 1) % tasks.tasks.len();

        if let Some(_) = tasks.tasks[new_task_id] {
            break new_task_id;
        }
    };

    tasks.current = new_task_id;

    // restore new task context
    *frame = tasks.tasks[new_task_id]
        .as_ref()
        .expect("new task to be Some")
        .frame
        .clone();
}
