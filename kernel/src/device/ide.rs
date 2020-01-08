use arraystring::{ArrayString, typenum::U40};
use x86_64::instructions::port::Port;

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

struct IdePorts {
    base: u16,
    control_base: u16,
}

impl IdePorts {
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
}

pub struct IdeChannel {
    ports: IdePorts,
}

#[derive(Debug, Clone, Copy)]
pub enum Device {
    // I don't like this, TODO figure out better naming here
    Master,
    Slave,
}

#[derive(Debug)]
pub struct Detect {
    model: ArrayString<U40>,
}

#[derive(Debug)]
pub enum DetectError {
    NoDevice,
    NotAta,
    Ata(AtaError),
}

type Sector = [u8; 512];

impl IdeChannel {
    // TODO - use PCI to figure out the right ports
    pub fn primary() -> Self {
        IdeChannel {
            ports: IdePorts {
                base: 0x1f0,
                control_base: 0x3f6,
            },
        }
    }

    fn select(&self, device: Device) {
        unsafe {
            self.ports.device_select().write(match device {
                Device::Master => 0xa0,
                Device::Slave => 0xb0,
            });
        }

        // TODO can we do something other than just busy waiting?
        self.wait_port_io();
    }

    fn wait_port_io(&self) {
        for _ in 0..4 {
            unsafe { self.ports.alternate_status_control().read(); }
        }
    }

    fn wait_command(&self) -> Result<AtaStatus, AtaError> {
        self.wait_port_io();

        loop {
            let status = self.status();

            if status.contains(AtaStatus::BUSY) {
                continue;
            }

            if status.contains(AtaStatus::ERROR) {
                return Err(self.error());
            }

            return Ok(status);
        }
    }

    fn status(&self) -> AtaStatus {
        let status_raw = unsafe { self.ports.command_status().read() };

        // all bits are defined in AtaStatus, so from_bits_truncate is fine:
        let status = AtaStatus::from_bits_truncate(status_raw);

        crate::println!("ata status: {:x?} => {:x?}", status_raw, status);

        status
    }

    fn error(&self) -> AtaError {
        AtaError::from_bits_truncate(unsafe {
            self.ports.error_features().read()
        })
    }

    fn read_pio_data(&self, buff: &mut Sector) {
        for i in 0..256 {
            let w = unsafe { self.ports.data().read() };
            buff[i * 2 + 0] = ((w >> 8) & 0xff) as u8;
            buff[i * 2 + 1] = ((w >> 0) & 0xff) as u8;
        }
    }

    pub fn detect(&self, device: Device) -> Result<Detect, DetectError> {
        self.select(device);

        unsafe {
            // set addressing ports to 0
            self.ports.seccount0().write(0);
            self.ports.lba0().write(0);
            self.ports.lba1().write(0);
            self.ports.lba2().write(0);

            // identify
            self.ports.command_status().write(AtaCommand::Identify as u8);

            let status = self.wait_command()
                .map_err(DetectError::Ata)?;

            if status.is_empty() {
                // drive does not exist
                return Err(DetectError::NoDevice);
            }

            // check lba1 and lba2 to make sure this is an ATA device
            if self.ports.lba1().read() != 0 || self.ports.lba2().read() != 0 {
                return Err(DetectError::NotAta);
            }

            /*
            // poll for request ready to come up
            self.wait_command()
            loop {
                let status = self.status();

                if status.contains(AtaStatus::DATA_REQUEST_READY) {
                    break;
                }

                if status.contains(AtaStatus::ERROR) {
                    return Err(DetectError::Ata(self.error()));
                }
            }
            */

            let mut identify_data = [0u8; 512];
            self.read_pio_data(&mut identify_data);

            let model = {
                let mut model = ArrayString::from_utf8(&identify_data[54..94])
                    .expect("IDE device model not UTF8");

                model.trim();

                model
            };

            Ok(Detect {
                model,
            })
        }
    }

    pub fn read_sectors(&self, device: Device, lba: usize, buffs: &mut [&mut Sector]) -> Result<(), AtaError> {
        if lba > 0x00ffffff {
            panic!("cannot read lba > 0x00ffffff currently");
        }

        if buffs.len() > 255 {
            panic!("cannot read more than 255 sectors currently");
        }

        let lba = lba.to_le_bytes();

        self.select(device);

        unsafe {
            self.ports.error_features().write(0);
            self.ports.seccount0().write(buffs.len() as u8);
            self.ports.lba0().write(lba[0]);
            self.ports.lba1().write(lba[1]);
            self.ports.lba2().write(lba[2]);
            self.ports.command_status().write(AtaCommand::ReadPio as u8);
        }

        for buff in buffs {
            self.wait_command()?;
            self.read_pio_data(buff);
        }

        Ok(())
    }
}
