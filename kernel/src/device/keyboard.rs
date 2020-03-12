use core::task::Poll;

use arraydeque::{ArrayDeque, Saturating};
use futures::future;
use x86_64::instructions::port::Port;

use crate::sync::Mutex;

pub type Scancode = u8;

static BUFF: Mutex<Option<ArrayDeque<[u8; 32], Saturating>>> = Mutex::new(None);

// Safety: must not be called more than once
pub unsafe fn init() {
    *BUFF.lock() = Some(ArrayDeque::new());
}

pub async fn read_scancode() -> Scancode {
    // TODO - send task to sleep while waiting for interrupt
    future::poll_fn(|_ctx| {
        let mut buff = BUFF.lock();

        let buff = buff.as_mut()
            .expect("keyboard to be initialized");

        match buff.pop_front() {
            None => Poll::Pending,
            Some(s) => Poll::Ready(s),
        }
    }).await
}

pub unsafe fn interrupt() {
    let mut keyboard = Port::<u8>::new(0x60);
    let raw_scancode = keyboard.read();

    // TODO - can we do this locklessly?
    let mut buff = BUFF.lock();

    if let Some(ref mut buff) = &mut *buff {
        match buff.push_back(raw_scancode) {
            Ok(()) => {}
            Err(_) => {
                crate::println!("keyboard buffer overflow!");
            }
        }
    }
}
