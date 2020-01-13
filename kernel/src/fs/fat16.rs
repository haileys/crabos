use core::future::Future;
use core::iter;
use core::mem;

use arraystring::{ArrayString, typenum::U11};
use arrayvec::ArrayVec;
use futures::future;
use futures::stream::{self, Stream, StreamExt, TryStream, TryStreamExt};

use crate::device::ide::{AtaError, Sector};
use crate::device::mbr::Partition;
use crate::mem::kalloc::GlobalAlloc;
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

impl From<AtaError> for FatError {
    fn from(e: AtaError) -> FatError {
        FatError::Ata(e)
    }
}

#[derive(Debug, Copy, Clone)]
struct ClusterNumber(usize);

impl Fat16 {
    pub async fn open(part: Partition) -> Result<Self, OpenError> {
        let bpb = BiosParameterBlock::read(&part).await
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

impl Filesystem {
    async fn next_cluster(&self, cluster: ClusterNumber) -> Result<Option<ClusterNumber>, AtaError> {
        const FAT_ENTRY_SIZE: usize = mem::size_of::<u16>();

        let max_cluster = self.bpb.fat_sector_count() * SECTOR_SIZE / FAT_ENTRY_SIZE;

        if cluster.0 >= max_cluster {
            panic!("cluster out of bounds: {:?}", cluster);
        }

        let fat_entry_offset = cluster.0 * FAT_ENTRY_SIZE;

        let fat_sector = self.bpb.first_fat_sector() +
            fat_entry_offset / SECTOR_SIZE;

        let sector_offset = fat_entry_offset % SECTOR_SIZE;

        let mut buff: Sector = [0u8; 512];
        self.part.read_sectors(fat_sector, &mut [&mut buff]).await?;

        let next_lo = buff[sector_offset + 0];
        let next_hi = buff[sector_offset + 1];
        let next = u16::from_le_bytes([next_lo, next_hi]);

        if next >= 0xfff8 {
            Ok(None)
        } else if next == 0xfff7 {
            panic!("bad cluster in chain! what do here?");
        } else {
            Ok(Some(ClusterNumber(next as usize)))
        }
    }

    fn cluster_chain(&self, start: ClusterNumber) -> impl Stream<Item = Result<ClusterNumber, AtaError>> + '_ {
        stream::unfold(Some(start), move |cluster| async move {
            match cluster {
                Some(cluster) => {
                    match self.next_cluster(cluster).await.transpose()? {
                        Ok(next) => Some((Ok(cluster), Some(next))),
                        Err(e) => Some((Err(e), None)),
                    }
                }
                None => None,
            }
        })
    }

    fn sector_chain(&self, start: ClusterNumber) -> impl Stream<Item = Result<usize, AtaError>> + '_ {
        self.cluster_chain(start)
            .map(move |cluster| {
                cluster.map(|cluster| stream::iter(self.bpb.cluster_sectors(cluster).map(Ok)))
            })
            .try_flatten()
    }
}

enum DirectoryKind {
    Root,
    Cluster(ClusterNumber),
}

pub struct Directory {
    fs: Arc<Filesystem>,
    kind: DirectoryKind,
}

impl Directory {
    fn directory_sectors(&self) -> impl TryStream<Ok = usize, Error = AtaError> + '_ {
        match self.kind {
            DirectoryKind::Root => {
                let first_sector = self.fs.bpb.first_root_dir_sector();
                let sector_count = self.fs.bpb.root_dir_sector_count();
                let sectors = first_sector..(first_sector + sector_count);

                stream::iter(sectors.into_iter().map(Ok))
                    .left_stream()
            }
            DirectoryKind::Cluster(start) => {
                self.fs.sector_chain(start)
                    .right_stream()
            }
        }
    }

    /// TODO this only reads the first sector/cluster worth of entries:
    pub fn read_entries(&self) -> impl TryStream<Ok = DirectoryEntry, Error = FatError> + '_ {
        async fn read_raw_entries_from_sector(fs: &Filesystem, sector: usize)
            -> Result<ArrayVec<[DirectoryEntry; 16]>, FatError>
        {
            let mut buff: Sector = [0u8; 512];
            fs.part.read_sectors(sector, &mut [&mut buff]).await?;

            let entries = unsafe { mem::transmute::<&Sector, &[DirectoryEntry; 16]>(&buff) };

            Ok(entries.iter().cloned().collect())
        }

        let fs = &self.fs;

        self.directory_sectors()
            .map_err(FatError::Ata)
            .and_then(move |sector| async move {
                let raw_entries = read_raw_entries_from_sector(fs, sector).await?;
                Ok(stream::iter(raw_entries.into_iter().map(Ok)))
            })
            .try_flatten()
            .try_filter(|entry| future::ready(entry.basename[0] != 0xef)) // deleted file
            .take_while(|entry| future::ready(entry.as_ref().map(|e| e.basename[0] != 0).unwrap_or(true))) // end
    }
}

pub struct ClusterChain {
    fs: Arc<Filesystem>,
    cluster: usize,
}

pub struct EntriesPage<F> {
    entries: [DirectoryEntry; 16],
    next: F,
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

    pub fn first_cluster_sector(&self, cluster_number: ClusterNumber) -> usize {
        let clusters_base = self.first_root_dir_sector() + self.root_dir_sector_count();

        // cluster numbers are 2-indexed:
        let cluster_number = cluster_number.0 - 2;

        clusters_base + cluster_number * self.sectors_per_cluster()
    }

    pub fn sectors_per_cluster(&self) -> usize {
        self.sectors_per_cluster as usize
    }

    pub fn cluster_sectors(&self, cluster_number: ClusterNumber) -> impl Iterator<Item = usize> {
        let first = self.first_cluster_sector(cluster_number);
        let count = self.sectors_per_cluster();

        (first..(first + count)).into_iter()
    }
}

impl BiosParameterBlock {
    pub async fn read(part: &Partition) -> Result<BiosParameterBlock, AtaError> {
        let mut buff: Sector = [0; 512];
        part.read_sectors(0, &mut [&mut buff]).await?;

        crate::println!("bpb sector: {:x?}", &buff[..]);

        let bpb = unsafe {
            mem::transmute::<&Sector, &BiosParameterBlock>(&buff).clone()
        };

        Ok(bpb)
    }
}
