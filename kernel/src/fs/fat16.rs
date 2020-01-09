use core::mem;

use arraystring::{ArrayString, typenum::U11};
use arrayvec::ArrayVec;

use crate::device::ide::{AtaError, Sector};
use crate::device::mbr::Partition;
use crate::sync::Arc;

const DIR_ENTRY_SIZE: usize = 32;
const SECTOR_SIZE: usize = 512;

pub struct Fat16 {
    fs: Arc<Filesystem>,
}

struct Filesystem {
    part: Partition,
    bpb: BiosParameterBlock,
}

#[derive(Debug)]
pub enum OpenError {
    MemoryExhausted,
    Ata(AtaError),
}

#[derive(Debug)]
pub enum FatError {
    Ata(AtaError),
}

impl Fat16 {
    pub async fn open(part: Partition) -> Result<Self, OpenError> {
        let bpb = read_bpb(&part).await
            .map_err(OpenError::Ata)?;

        crate::println!("bpb: {:#x?}", bpb);

        let fs = Arc::new(Filesystem { part, bpb })
            .map_err(|_| OpenError::MemoryExhausted)?;

        Ok(Fat16 { fs })
    }

    pub fn root(&self) -> Directory {
        Directory {
            fs: self.fs.clone(),
            kind: DirectoryKind::Root,
        }
    }
}

enum DirectoryKind {
    Root,
    Cluster(usize),
}

pub struct Directory {
    fs: Arc<Filesystem>,
    kind: DirectoryKind,
}

impl Directory {
    /// TODO this only reads the first sector/cluster worth of entries:
    pub async fn read_entries(&self) -> Result<ArrayVec<[DirectoryEntry; 16]>, FatError> {
        let sector = match self.kind {
            DirectoryKind::Root => self.fs.bpb.first_root_dir_sector(),
            DirectoryKind::Cluster(number) => self.fs.bpb.first_cluster_sector(number),
        };

        let mut buff: Sector = [0u8; 512];
        self.fs.part.read_sectors(sector, &mut [&mut buff])
            .await
            .map_err(FatError::Ata)?;

        let entries = unsafe { mem::transmute::<&Sector, &[DirectoryEntry; 16]>(&buff) };

        Ok(entries.iter()
            .take_while(|entry| entry.basename[0] != 0) // end
            .filter(|entry| entry.basename[0] != 0xef) // deleted file
            .copied()
            .collect::<ArrayVec<_>>())
    }
}

#[repr(packed)]
#[derive(Clone, Copy, Debug)]
pub struct DirectoryEntry {
    basename: [u8; 8],
    extension: [u8; 3],
    attributes: u8,
    _reserved: u8,
    create_tenths: u8,
    create_time: PackedTime,
    create_date: PackedDate,
    access_date: PackedDate,
    cluster_hi: u16,
    modify_time: PackedTime,
    modify_date: PackedTime,
    cluster_lo: u16,
    size: u32,
}

impl DirectoryEntry {
    pub fn filename(&self) -> ArrayVec<[u8; 12]> {
        let mut filename = ArrayVec::new();

        filename.extend(self.basename.iter().copied());

        // trim trailing space
        while filename.last() == Some(&b' ') {
            filename.pop();
        }

        if self.extension[0] != b' ' {
            filename.push(b'.');

            filename.extend(self.extension.iter().copied());

            // trim trailing space again
            while filename.last() == Some(&b' ') {
                filename.pop();
            }
        }

        filename
    }
}

#[repr(packed)]
#[derive(Clone, Copy, Debug)]
struct PackedTime {
    hms: u16,
}

#[repr(packed)]
#[derive(Clone, Copy, Debug)]
struct PackedDate {
    ymd: u16,
}

#[repr(packed)]
#[derive(Clone, Copy, Debug)]
struct BiosParameterBlock {
    // 0x0
    jmp: [u8; 3],
    // 0x3
    oem: [u8; 8],
    // 0xb
    bytes_per_sector: u16,
    // 0xd
    sectors_per_cluster: u8,
    // 0xe
    reserved_sector_count: u16,
    fat_count: u8,
    root_directory_entry_count: u16,
    total_sector_count: u16,
    media_descriptor_type: u8,
    sectors_per_fat: u16,
    // more stuff but we don't use it
}

impl BiosParameterBlock {
    pub fn first_fat_sector(&self) -> usize {
        self.reserved_sector_count as usize
    }

    pub fn fat_sector_count(&self) -> usize {
        self.sectors_per_fat as usize
    }

    pub fn all_fats_sector_count(&self) -> usize {
        self.fat_count as usize * self.fat_sector_count()
    }

    pub fn first_root_dir_sector(&self) -> usize {
        self.first_fat_sector() + self.all_fats_sector_count()
    }

    pub fn root_dir_sector_count(&self) -> usize {
        (self.root_directory_entry_count as usize * DIR_ENTRY_SIZE) / SECTOR_SIZE
    }

    pub fn first_cluster_sector(&self, cluster_number: usize) -> usize {
        let clusters_base = self.first_root_dir_sector() + self.root_dir_sector_count();

        // cluster numbers are 2-indexed:
        let cluster_number = cluster_number - 2;

        clusters_base + cluster_number * self.sectors_per_cluster()
    }

    pub fn sectors_per_cluster(&self) -> usize {
        self.sectors_per_cluster as usize
    }
}

async fn read_bpb(part: &Partition) -> Result<BiosParameterBlock, AtaError> {
    let mut buff: Sector = [0; 512];
    part.read_sectors(0, &mut [&mut buff]).await?;

    crate::println!("bpb sector: {:x?}", &buff[..]);

    let bpb = unsafe {
        mem::transmute::<&Sector, &BiosParameterBlock>(&buff).clone()
    };

    Ok(bpb)
}
