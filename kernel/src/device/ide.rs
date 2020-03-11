use core::sync::atomic::{AtomicBool, Ordering};

use arrayvec::ArrayString;
use x86_64::instructions::port::Port;

use crate::sync::{Mutex, MutexGuard};
use crate::util;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drive {
    A,
    #[allow(unused)]
    B,
}

#[derive(Debug)]
pub struct DriveBusy;

#[derive(Debug)]
pub struct IdeChannel {
    a: AtomicBool,
    b: AtomicBool,
    io: Mutex<IdeIo>,
}

impl IdeChannel {
    const fn new(io: IdeIo) -> Self {
        IdeChannel {
            a: AtomicBool::new(false),
            b: AtomicBool::new(false),
            io: Mutex::new(io),
        }
    }

    fn busyness(&self, drive: Drive) -> &AtomicBool {
        match drive {
            Drive::A => &self.a,
            Drive::B => &self.b,
        }
    }

    pub fn open(&'static self, drive: Drive) -> Result<IdeDrive, DriveBusy> {
        if self.busyness(drive).swap(true, Ordering::SeqCst) {
            return Err(DriveBusy);
        }

        Ok(IdeDrive { channel: self, drive })
    }
}

pub static PRIMARY: IdeChannel = IdeChannel::new(IdeIo {
    base: 0x1f0,
    control_base: 0x3f6,
});

#[derive(Debug)]
pub struct IdeDrive {
    channel: &'static IdeChannel,
    drive: Drive,
}

impl Drop for IdeDrive {
    fn drop(&mut self) {
        self.channel.busyness(self.drive)
            .store(false, Ordering::SeqCst)
    }
}

bitflags::bitflags! {
    pub struct AtaStatus: u8 {
        const BUSY                  = 0x80;
        const DRIVE_READY           = 0x40;
        const WRITE_FAULT           = 0x20;
        const SEEK_COMPLETE         = 0x10;
        const DATA_REQUEST_READY    = 0x08;
        const CORRECTED_DATA        = 0x04;
        const INDEX                 = 0x02;
        const ERROR                 = 0x01;
    }
}

bitflags::bitflags! {
    pub struct AtaError: u8 {
        const BAD_BLOCK             = 0x80;
        const UNCORRECTABLE_DATA    = 0x40;
        const MEDIA_CHANGED         = 0x20;
        const ID_MARK_NOT_FOUND     = 0x10;
        const MEDIA_CHANGE_REQUEST  = 0x08;
        const COMMAND_ABORTED       = 0x04;
        const TRACK_0_NOT_FOUND     = 0x02;
        const NO_ADDRESS_MARK       = 0x01;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AtaCommand {
    ReadPio = 0x20,
    Identify = 0xec,
}

#[derive(Debug)]
struct IdeIo {
    base: u16,
    control_base: u16,
}

impl IdeIo {
    pub fn data(&self) -> Port<u16> {
        Port::new(self.base + 0)
    }

    pub fn error_features(&self) -> Port<u8> {
        Port::new(self.base + 1)
    }

    pub fn seccount0(&self) -> Port<u8> {
        Port::new(self.base + 2)
    }

    pub fn lba0(&self) -> Port<u8> {
        Port::new(self.base + 3)
    }

    pub fn lba1(&self) -> Port<u8> {
        Port::new(self.base + 4)
    }

    pub fn lba2(&self) -> Port<u8> {
        Port::new(self.base + 5)
    }

    pub fn device_select(&self) -> Port<u8> {
        Port::new(self.base + 6)
    }

    pub fn command_status(&self) -> Port<u8> {
        Port::new(self.base + 7)
    }

    pub fn alternate_status_control(&self) -> Port<u8> {
        Port::new(self.control_base + 2)
    }

    pub fn wait(&self) {
        for _ in 0..4 {
            unsafe { self.alternate_status_control().read(); }
        }
    }

    fn wait_command(&self, required: AtaStatus) -> Result<AtaStatus, AtaError> {
        self.wait();

        loop {
            let status = self.status();

            if status.contains(AtaStatus::BUSY) {
                continue;
            }

            if status.contains(AtaStatus::ERROR) {
                return Err(self.error());
            }

            // if status.contains(AtaStatus::DRIVE_FAIL) {
            //     return Err(self.error());
            // }

            if status.contains(required) {
                return Ok(status);
            }
        }
    }

    fn status(&self) -> AtaStatus {
        let status_raw = unsafe { self.command_status().read() };

        // all bits are defined in AtaStatus, so from_bits_truncate is fine:
        let status = AtaStatus::from_bits_truncate(status_raw);

        crate::println!("ata status: {:x?} => {:x?}", status_raw, status);

        status
    }

    fn error(&self) -> AtaError {
        AtaError::from_bits_truncate(unsafe {
            self.error_features().read()
        })
    }

    fn read_pio_data(&self, buff: &mut Sector) {
        for i in 0..256 {
            let w = unsafe { self.data().read() };
            buff[i * 2 + 0] = ((w >> 0) & 0xff) as u8;
            buff[i * 2 + 1] = ((w >> 8) & 0xff) as u8;
        }
    }
}

#[derive(Debug)]
pub struct Detect {
    model: ArrayString<[u8; 40]>,
}

#[derive(Debug)]
pub enum DetectError {
    NoDevice,
    NotAta,
    Ata(AtaError),
}

pub type Sector = [u8; 512];

impl IdeDrive {
    fn select(&self) -> MutexGuard<IdeIo> {
        let ports = self.channel.io.lock();

        unsafe {
            ports.device_select().write(match self.drive {
                Drive::A => 0xe0,
                Drive::B => 0xf0,
            });
        }

        // TODO can we do something other than just busy waiting?
        ports.wait();

        ports
    }

    pub async fn detect(&self) -> Result<Detect, DetectError> {
        let io = self.select();

        unsafe {
            // set addressing ports to 0
            io.seccount0().write(0);
            io.lba0().write(0);
            io.lba1().write(0);
            io.lba2().write(0);
            io.wait();

            // identify
            io.command_status().write(AtaCommand::Identify as u8);

            let status = io.wait_command(AtaStatus::DRIVE_READY)
                .map_err(DetectError::Ata)?;

            if status.is_empty() {
                // drive does not exist
                return Err(DetectError::NoDevice);
            }

            // check lba1 and lba2 to make sure this is an ATA device
            if io.lba1().read() != 0 || io.lba2().read() != 0 {
                return Err(DetectError::NotAta);
            }

            let mut identify_data = [0u8; 512];
            io.read_pio_data(&mut identify_data);

            // ASCII strings in the identify response are big endian
            // https://www.win.tue.nl/~aeb/linux/Large-Disk-10.html
            for idx in (20..96).step_by(2) {
                let a = identify_data[idx + 0];
                let b = identify_data[idx + 1];

                identify_data[idx + 0] = b;
                identify_data[idx + 1] = a;
            }

            let model = {
                let mut model = util::array_string(&identify_data[54..94])
                    .expect("IDE device model not UTF8");

                while let Some(' ') = model.chars().rev().nth(0) {
                    model.pop();
                }

                model
            };

            Ok(Detect {
                model,
            })
        }
    }

    pub async fn read_sectors(&self, lba: usize, buffs: &mut [&mut Sector]) -> Result<(), AtaError> {
        crate::println!("read_sectors({:x})", lba);

        if lba > 0x00fffffe {
            panic!("cannot read lba > 0x00ffffff currently");
        }

        if buffs.len() > 255 {
            panic!("cannot read more than 255 sectors currently");
        }

        let lba = lba.to_le_bytes();

        let io = self.select();
        io.wait_command(AtaStatus::empty())?;

        unsafe {
            io.error_features().write(0);
            io.seccount0().write(buffs.len() as u8);
            crate::println!("  lba: {:x?}", lba);
            io.lba0().write(lba[0]);
            io.lba1().write(lba[1]);
            io.lba2().write(lba[2]);
            io.wait_command(AtaStatus::DRIVE_READY)?;
            io.command_status().write(AtaCommand::ReadPio as u8);
        }

        for buff in buffs {
            io.wait_command(AtaStatus::DATA_REQUEST_READY)?;
            io.read_pio_data(buff);
        }

        Ok(())
    }
}
