use core::mem;
use core::ops::Range;
use core::slice;

use arrayvec::ArrayVec;

use crate::device::ide::{IdeDrive, Sector, AtaError};
use crate::mem::MemoryExhausted;
use crate::sync::Arc;

pub struct Mbr {
    drive: Arc<IdeDrive>,
}

impl Mbr {
    pub fn open(drive: IdeDrive) -> Result<Self, MemoryExhausted> {
        Ok(Mbr { drive: Arc::new(drive)? })
    }

    pub fn partitions(&self) -> Result<ArrayVec<[Option<Partition>; 4]>, AtaError> {
        #[repr(packed)]
        struct RawMbr {
            pad: [u8; 0x1be],
            entries: [MbrEntry; 4],
        }

        let mut boot_sector = [0u8; 512];
        self.drive.read_sectors(0, &mut [&mut boot_sector])?;

        let mbr = unsafe { mem::transmute::<&[u8; 512], &RawMbr>(&boot_sector) };

        crate::println!("{:x?}", &mbr.pad[432..440]);

        let mut parts = [None, None, None, None];

        for (idx, part) in mbr.entries.iter().enumerate() {
            crate::println!("{:?}", part);
            if (part.status & 0x80) != 0 {
                parts[idx] = Some(Partition {
                    drive: self.drive.clone(),
                    number: idx,
                    lba: part.lba as usize,
                    sectors: part.sectors as usize,
                });
            }
        }

        Ok(ArrayVec::from(parts))
    }
}

#[repr(packed)]
#[derive(Debug)]
struct MbrEntry {
    status: u8,
    chs_first: (u8, u8, u8),
    type_: u8,
    chs_last: (u8, u8, u8),
    lba: u32,
    sectors: u32,
}

pub struct Partition {
    drive: Arc<IdeDrive>,
    pub number: usize,
    pub lba: usize,
    pub sectors: usize,
}

impl Partition {
    pub fn read_sectors(&self, lba: usize, buffs: &mut [&mut Sector])
        -> Result<(), AtaError>
    {
        if lba + buffs.len() > self.sectors {
            panic!("would read beyond partition");
        }

        self.drive.read_sectors(lba + self.lba, buffs)
    }
}
