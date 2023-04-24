#[cfg(feature = "builder")]
use crate::builder::boxed_dst_tag;
#[cfg(feature = "builder")]
use crate::builder::traits::StructAsBytes;
use crate::tag_type::{TagType, TagTypeId};

use core::convert::TryInto;
use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem;

#[cfg(feature = "builder")]
use alloc::boxed::Box;

/// This tag provides an initial host memory map.
///
/// The map provided is guaranteed to list all standard RAM that should be
/// available for normal use. This type however includes the regions occupied
/// by kernel, mbi, segments and modules. Kernel must take care not to
/// overwrite these regions.
///
/// This tag may not be provided by some boot loaders on EFI platforms if EFI
/// boot services are enabled and available for the loaded image (The EFI boot
/// services tag may exist in the Multiboot2 boot information structure).
#[derive(Debug)]
#[repr(C)]
pub struct MemoryMapTag {
    typ: TagTypeId,
    size: u32,
    entry_size: u32,
    entry_version: u32,
    areas: [MemoryArea],
}

impl MemoryMapTag {
    #[cfg(feature = "builder")]
    pub fn new(areas: &[MemoryArea]) -> Box<Self> {
        let entry_size: u32 = mem::size_of::<MemoryArea>().try_into().unwrap();
        let entry_version: u32 = 0;
        let mut bytes = [entry_size.to_le_bytes(), entry_version.to_le_bytes()].concat();
        for area in areas {
            bytes.extend(area.struct_as_bytes());
        }
        let tag = boxed_dst_tag(TagType::Mmap.into(), bytes.as_slice());
        unsafe { Box::from_raw(Box::into_raw(tag) as *mut Self) }
    }

    /// Return an iterator over all memory areas that have the type
    /// [`MemoryAreaType::Available`].
    pub fn available_memory_areas(&self) -> impl Iterator<Item = &MemoryArea> {
        self.memory_areas()
            .filter(|entry| matches!(entry.typ, MemoryAreaType::Available))
    }

    /// Return an iterator over all memory areas.
    pub fn memory_areas(&self) -> MemoryAreaIter {
        let self_ptr = self as *const MemoryMapTag;
        let start_area = (&self.areas[0]) as *const MemoryArea;
        MemoryAreaIter {
            current_area: start_area as u64,
            // NOTE: `last_area` is only a bound, it doesn't necessarily point exactly to the last element
            last_area: (self_ptr as *const () as u64 + (self.size - self.entry_size) as u64),
            entry_size: self.entry_size,
            phantom: PhantomData,
        }
    }

    /// Return a mutable iterator over all marked memory areas.
    pub fn all_memory_areas_mut(&mut self) -> impl Iterator<Item = &mut MemoryArea> {
        let self_ptr = self as *mut MemoryMapTag;
        let start_area = (&mut self.areas[0]) as *mut MemoryArea;
        MemoryAreaIterMut {
            current_area: start_area as u64,
            last_area: (self_ptr as *const () as u64 + (self.size - self.entry_size) as u64),
            entry_size: self.entry_size,
            phantom: PhantomData,
        }
    }
}

#[cfg(feature = "builder")]
impl StructAsBytes for MemoryMapTag {
    fn byte_size(&self) -> usize {
        self.size.try_into().unwrap()
    }
}

/// A memory area entry descriptor.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct MemoryArea {
    base_addr: u64,
    length: u64,
    typ: MemoryAreaType,
    _reserved: u32,
}

impl MemoryArea {
    /// Create a new MemoryArea.
    pub fn new(base_addr: u64, length: u64, typ: MemoryAreaType) -> Self {
        Self {
            base_addr,
            length,
            typ,
            _reserved: 0,
        }
    }

    /// The start address of the memory region.
    pub fn start_address(&self) -> u64 {
        self.base_addr
    }

    /// The end address of the memory region.
    pub fn end_address(&self) -> u64 {
        self.base_addr + self.length
    }

    /// The size, in bytes, of the memory region.
    pub fn size(&self) -> u64 {
        self.length
    }

    /// The type of the memory region.
    pub fn typ(&self) -> MemoryAreaType {
        self.typ
    }
}

#[cfg(feature = "builder")]
impl StructAsBytes for MemoryArea {
    fn byte_size(&self) -> usize {
        mem::size_of::<Self>()
    }
}

/// An enum of possible reported region types.
/// Inside the Multiboot2 spec this is kind of hidden
/// inside the implementation of `struct multiboot_mmap_entry`.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(u32)]
pub enum MemoryAreaType {
    /// Available memory free to be used by the OS.
    Available = 1,

    /// A reserved area that must not be used.
    Reserved = 2,

    /// Usable memory holding ACPI information.
    AcpiAvailable = 3,

    /// Reserved memory which needs to be preserved on hibernation.
    /// Also called NVS in spec, which stands for "Non-Volatile Sleep/Storage",
    /// which is part of ACPI specification.
    ReservedHibernate = 4,

    /// Memory which is occupied by defective RAM modules.
    Defective = 5,
}

/// An iterator over all memory areas
#[derive(Clone, Debug)]
pub struct MemoryAreaIter<'a> {
    current_area: u64,
    last_area: u64,
    entry_size: u32,
    phantom: PhantomData<&'a MemoryArea>,
}

impl<'a> Iterator for MemoryAreaIter<'a> {
    type Item = &'a MemoryArea;
    fn next(&mut self) -> Option<&'a MemoryArea> {
        if self.current_area > self.last_area {
            None
        } else {
            let area = unsafe { &*(self.current_area as *const MemoryArea) };
            self.current_area += self.entry_size as u64;
            Some(area)
        }
    }
}

/// A mutable iterator over all memory areas
#[derive(Clone, Debug)]
pub struct MemoryAreaIterMut<'a> {
    current_area: u64,
    last_area: u64,
    entry_size: u32,
    phantom: PhantomData<&'a MemoryArea>,
}

impl<'a> Iterator for MemoryAreaIterMut<'a> {
    type Item = &'a mut MemoryArea;
    fn next(&mut self) -> Option<&'a mut MemoryArea> {
        if self.current_area > self.last_area {
            None
        } else {
            let area = unsafe { &mut *(self.current_area as *mut MemoryArea) };
            self.current_area += self.entry_size as u64;
            Some(area)
        }
    }
}

/// Basic memory info
///
/// This tag includes "basic memory information".
/// This means (legacy) lower and upper memory:
/// In Real Mode (modeled after the 8086),
/// only the first 1MB of memory is accessible.
/// Typically, the region between 640KB and 1MB is not freely usable,
/// because it is used for memory-mapped IO, for instance.
/// The term “lower memory” refers to those first 640KB of memory that are
/// freely usable for an application in Real Mode.
/// “Upper memory” then refers to the next freely usable chunk of memory,
/// starting at 1MB up to about 10MB, in practice.
/// This is the memory an application running on a 286
/// (which had a 24-bit address bus) could use, historically.
/// Nowadays, much bigger chunks of continuous memory are available at higher
/// addresses, but the Multiboot standard still references those two terms.
#[repr(C, packed)]
pub struct BasicMemoryInfoTag {
    typ: TagTypeId,
    size: u32,
    memory_lower: u32,
    memory_upper: u32,
}

impl BasicMemoryInfoTag {
    pub fn new(memory_lower: u32, memory_upper: u32) -> Self {
        Self {
            typ: TagType::BasicMeminfo.into(),
            size: mem::size_of::<BasicMemoryInfoTag>().try_into().unwrap(),
            memory_lower,
            memory_upper,
        }
    }

    pub fn memory_lower(&self) -> u32 {
        self.memory_lower
    }

    pub fn memory_upper(&self) -> u32 {
        self.memory_upper
    }
}

#[cfg(feature = "builder")]
impl StructAsBytes for BasicMemoryInfoTag {
    fn byte_size(&self) -> usize {
        mem::size_of::<Self>()
    }
}

impl Debug for BasicMemoryInfoTag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BasicMemoryInfoTag")
            .field("typ", &{ self.typ })
            .field("size", &{ self.size })
            .field("memory_lower", &{ self.memory_lower })
            .field("memory_upper", &{ self.memory_upper })
            .finish()
    }
}

/// EFI memory map as per EFI specification.
#[derive(Debug)]
#[repr(C)]
pub struct EFIMemoryMapTag {
    typ: TagTypeId,
    size: u32,
    desc_size: u32,
    desc_version: u32,
    descs: [EFIMemoryDesc],
}

impl EFIMemoryMapTag {
    #[cfg(feature = "builder")]
    /// Create a new EFI memory map tag with the given memory descriptors.
    /// Version and size can't be set because you're passing a slice of
    /// EFIMemoryDescs, not the ones you might have gotten from the firmware.
    pub fn new(descs: &[EFIMemoryDesc]) -> Box<Self> {
        // update this when updating EFIMemoryDesc
        const MEMORY_DESCRIPTOR_VERSION: u32 = 1;
        let mut bytes = [
            (mem::size_of::<EFIMemoryDesc>() as u32).to_le_bytes(),
            MEMORY_DESCRIPTOR_VERSION.to_le_bytes(),
        ]
        .concat();
        for desc in descs {
            bytes.extend(desc.struct_as_bytes());
        }
        let tag = boxed_dst_tag(TagType::EfiMmap.into(), bytes.as_slice());
        unsafe { Box::from_raw(Box::into_raw(tag) as *mut Self) }
    }

    /// Return an iterator over ALL marked memory areas.
    ///
    /// This differs from `MemoryMapTag` as for UEFI, the OS needs some non-
    /// available memory areas for tables and such.
    pub fn memory_areas(&self) -> EFIMemoryAreaIter {
        let self_ptr = self as *const EFIMemoryMapTag;
        let start_area = (&self.descs[0]) as *const EFIMemoryDesc;
        EFIMemoryAreaIter {
            current_area: start_area as u64,
            // NOTE: `last_area` is only a bound, it doesn't necessarily point exactly to the last element
            last_area: (self_ptr as *const () as u64 + self.size as u64),
            entry_size: self.desc_size,
            phantom: PhantomData,
        }
    }

    #[cfg(feature = "builder")]
    /// Return an iterator over ALL marked memory areas, mutably.
    ///
    /// This differs from `MemoryMapTag` as for UEFI, the OS needs some non-
    /// available memory areas for tables and such.
    pub fn memory_areas_mut(&mut self) -> EFIMemoryAreaIterMut {
        let self_ptr = self as *const EFIMemoryMapTag;
        let start_area = (&self.descs[0]) as *const EFIMemoryDesc;
        EFIMemoryAreaIterMut {
            current_area: start_area as u64,
            // NOTE: `last_area` is only a bound, it doesn't necessarily point exactly to the last element
            last_area: (self_ptr as *const () as u64 + self.size as u64),
            entry_size: self.desc_size,
            phantom: PhantomData,
        }
    }
}

#[cfg(feature = "builder")]
impl StructAsBytes for EFIMemoryMapTag {
    fn byte_size(&self) -> usize {
        self.size.try_into().unwrap()
    }
}

/// EFI Boot Memory Map Descriptor
#[derive(Debug, Clone)]
#[repr(C)]
pub struct EFIMemoryDesc {
    typ: u32,
    _padding: u32,
    phys_addr: u64,
    virt_addr: u64,
    num_pages: u64,
    attr: u64,
}

#[cfg(feature = "builder")]
impl StructAsBytes for EFIMemoryDesc {
    fn byte_size(&self) -> usize {
        mem::size_of::<Self>()
    }
}

/// An enum of possible reported region types.
#[derive(Debug, PartialEq, Eq)]
pub enum EFIMemoryAreaType {
    /// Unusable.
    EfiReservedMemoryType,
    /// Code area of a UEFI application.
    EfiLoaderCode,
    /// Data area of a UEFI application.
    EfiLoaderData,
    /// Code area of a UEFI Boot Service Driver.
    EfiBootServicesCode,
    /// Data area of a UEFI Boot Service Driver.
    EfiBootServicesData,
    /// Code area of a UEFI Runtime Driver.
    ///
    /// Must be preserved in working and ACPI S1-S3 states.
    EfiRuntimeServicesCode,
    /// Data area of a UEFI Runtime Driver.
    ///
    /// Must be preserved in working and ACPI S1-S3 states.
    EfiRuntimeServicesData,
    /// Available memory.
    EfiConventionalMemory,
    /// Memory with errors, treat as unusable.
    EfiUnusableMemory,
    /// Memory containing the ACPI tables.
    ///
    /// Must be preserved in working and ACPI S1-S3 states.
    EfiACPIReclaimMemory,
    /// Memory reserved by firmware.
    ///
    /// Must be preserved in working and ACPI S1-S3 states.
    EfiACPIMemoryNVS,
    /// Memory used by firmware for requesting memory mapping of IO.
    ///
    /// Should not be used by the OS. Use the ACPI tables for memory mapped IO
    /// information.
    EfiMemoryMappedIO,
    /// Memory used to translate memory cycles to IO cycles.
    ///
    /// Should not be used by the OS. Use the ACPI tables for memory mapped IO
    /// information.
    EfiMemoryMappedIOPortSpace,
    /// Memory used by the processor.
    ///
    /// Must be preserved in working and ACPI S1-S4 states. Processor defined
    /// otherwise.
    EfiPalCode,
    /// Available memory supporting byte-addressable non-volatility.
    EfiPersistentMemory,
    /// Unknown region type, treat as unusable.
    EfiUnknown,
}

impl From<EFIMemoryAreaType> for u32 {
    fn from(area: EFIMemoryAreaType) -> Self {
        match area {
            EFIMemoryAreaType::EfiReservedMemoryType => 0,
            EFIMemoryAreaType::EfiLoaderCode => 1,
            EFIMemoryAreaType::EfiLoaderData => 2,
            EFIMemoryAreaType::EfiBootServicesCode => 3,
            EFIMemoryAreaType::EfiBootServicesData => 4,
            EFIMemoryAreaType::EfiRuntimeServicesCode => 5,
            EFIMemoryAreaType::EfiRuntimeServicesData => 6,
            EFIMemoryAreaType::EfiConventionalMemory => 7,
            EFIMemoryAreaType::EfiUnusableMemory => 8,
            EFIMemoryAreaType::EfiACPIReclaimMemory => 9,
            EFIMemoryAreaType::EfiACPIMemoryNVS => 10,
            EFIMemoryAreaType::EfiMemoryMappedIO => 11,
            EFIMemoryAreaType::EfiMemoryMappedIOPortSpace => 12,
            EFIMemoryAreaType::EfiPalCode => 13,
            EFIMemoryAreaType::EfiPersistentMemory => 14,
            EFIMemoryAreaType::EfiUnknown => panic!("unknown type"),
        }
    }
}

impl EFIMemoryDesc {
    /// The physical address of the memory region.
    pub fn physical_address(&self) -> u64 {
        self.phys_addr
    }

    /// The virtual address of the memory region.
    pub fn virtual_address(&self) -> u64 {
        self.virt_addr
    }

    /// The size in bytes of the memory region.
    pub fn size(&self) -> u64 {
        // Spec says this is number of 4KiB pages.
        self.num_pages * 4096
    }

    /// The type of the memory region.
    pub fn typ(&self) -> EFIMemoryAreaType {
        match self.typ {
            0 => EFIMemoryAreaType::EfiReservedMemoryType,
            1 => EFIMemoryAreaType::EfiLoaderCode,
            2 => EFIMemoryAreaType::EfiLoaderData,
            3 => EFIMemoryAreaType::EfiBootServicesCode,
            4 => EFIMemoryAreaType::EfiBootServicesData,
            5 => EFIMemoryAreaType::EfiRuntimeServicesCode,
            6 => EFIMemoryAreaType::EfiRuntimeServicesData,
            7 => EFIMemoryAreaType::EfiConventionalMemory,
            8 => EFIMemoryAreaType::EfiUnusableMemory,
            9 => EFIMemoryAreaType::EfiACPIReclaimMemory,
            10 => EFIMemoryAreaType::EfiACPIMemoryNVS,
            11 => EFIMemoryAreaType::EfiMemoryMappedIO,
            12 => EFIMemoryAreaType::EfiMemoryMappedIOPortSpace,
            13 => EFIMemoryAreaType::EfiPalCode,
            14 => EFIMemoryAreaType::EfiPersistentMemory,
            _ => EFIMemoryAreaType::EfiUnknown,
        }
    }
}

impl Default for EFIMemoryDesc {
    fn default() -> Self {
        Self {
            typ: EFIMemoryAreaType::EfiReservedMemoryType.into(),
            _padding: 0,
            phys_addr: 0,
            virt_addr: 0,
            num_pages: 0,
            attr: 0,
        }
    }
}

/// EFI ExitBootServices was not called
#[derive(Debug)]
#[repr(C)]
pub struct EFIBootServicesNotExited {
    typ: TagTypeId,
    size: u32,
}

impl EFIBootServicesNotExited {
    #[cfg(feature = "builder")]
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(feature = "builder")]
impl Default for EFIBootServicesNotExited {
    fn default() -> Self {
        Self {
            typ: TagType::EfiBs.into(),
            size: mem::size_of::<Self>().try_into().unwrap(),
        }
    }
}

#[cfg(feature = "builder")]
impl StructAsBytes for EFIBootServicesNotExited {
    fn byte_size(&self) -> usize {
        mem::size_of::<Self>()
    }
}

/// An iterator over ALL EFI memory areas.
#[derive(Clone, Debug)]
pub struct EFIMemoryAreaIter<'a> {
    current_area: u64,
    last_area: u64,
    entry_size: u32,
    phantom: PhantomData<&'a EFIMemoryDesc>,
}

impl<'a> Iterator for EFIMemoryAreaIter<'a> {
    type Item = &'a EFIMemoryDesc;
    fn next(&mut self) -> Option<&'a EFIMemoryDesc> {
        if self.current_area > self.last_area {
            None
        } else {
            let area = unsafe { &*(self.current_area as *const EFIMemoryDesc) };
            self.current_area += self.entry_size as u64;
            Some(area)
        }
    }
}

/// An iterator over ALL EFI memory areas, mutably.
#[derive(Clone, Debug)]
pub struct EFIMemoryAreaIterMut<'a> {
    current_area: u64,
    last_area: u64,
    entry_size: u32,
    phantom: PhantomData<&'a mut EFIMemoryDesc>,
}

impl<'a> Iterator for EFIMemoryAreaIterMut<'a> {
    type Item = &'a mut EFIMemoryDesc;
    fn next(&mut self) -> Option<&'a mut EFIMemoryDesc> {
        if self.current_area > self.last_area {
            None
        } else {
            let area = unsafe { &mut *(self.current_area as *mut EFIMemoryDesc) };
            self.current_area += self.entry_size as u64;
            Some(area)
        }
    }
}
