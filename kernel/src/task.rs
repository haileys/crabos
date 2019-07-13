use crate::interrupt::TrapFrame;
use crate::page::{self, Pml4};
use crate::sync::Mutex;

pub const SEG_UCODE: u16 = 0x1b;
pub const SEG_UDATA: u16 = 0x23;

pub struct Task {
    frame: TrapFrame,
    pml4: Pml4,
}

static TASKS: Mutex<[Option<Task>; 8]> = Mutex::new([None, None, None, None, None, None, None, None]);

pub unsafe fn start() -> ! {
    let mut frame = TrapFrame::new(0x1_0000_0000, 0x0);

    {
        let mut tasks = TASKS.lock();

        tasks[0] = Some(Task {
            frame: frame.clone(),
            pml4: page::pml4(),
        });
    }

    asm!("
        movq $0, %rsp
        jmp interrupt_return
    " :: "r"(&mut frame as *mut TrapFrame) :: "volatile");

    loop {}
}

// const_assert!(task_size; mem::size_of::<Task>() == 184);
