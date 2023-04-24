#[cfg(feature = "builder")]
use crate::builder::boxed_dst_tag;
#[cfg(feature = "builder")]
use crate::builder::traits::StructAsBytes;
use crate::tag_type::{TagType, TagTypeId};

use core::convert::TryInto;
use core::fmt::Debug;

#[cfg(feature = "builder")]
use alloc::boxed::Box;

#[cfg(test)]
const METADATA_SIZE: usize = core::mem::size_of::<TagTypeId>()
    + core::mem::size_of::<u32>()
    + core::mem::size_of::<u8>() * 8;

/// This tag contains a copy of SMBIOS tables as well as their version.
#[repr(C, packed)]
pub struct SmbiosTag {
    typ: TagTypeId,
    size: u32,
    pub major: u8,
    pub minor: u8,
    _reserved: [u8; 6],
    pub tables: [u8],
}

impl SmbiosTag {
    #[cfg(feature = "builder")]
    pub fn new(major: u8, minor: u8, tables: &[u8]) -> Box<Self> {
        let mut bytes = [major, minor].to_vec();
        bytes.extend(tables);
        let tag = boxed_dst_tag(TagType::Smbios.into(), bytes.as_slice());
        unsafe { Box::from_raw(Box::into_raw(tag) as *mut Self) }
    }
}

#[cfg(feature = "builder")]
impl StructAsBytes for SmbiosTag {
    fn byte_size(&self) -> usize {
        self.size.try_into().unwrap()
    }
}

impl Debug for SmbiosTag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BootLoaderNameTag")
            .field("typ", &{ self.typ })
            .field("size", &{ self.size })
            .field("major", &{ self.major })
            .field("minor", &{ self.minor })
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use core::ptr::slice_from_raw_parts;

    use crate::smbios::METADATA_SIZE;
    use crate::tag_type::TagType;

    /// Returns the tag structure in bytes in native endian format.
    fn get_bytes() -> std::vec::Vec<u8> {
        let tables = [0xabu8; 24];
        // size is: 4 bytes for tag + 4 bytes for size + 1 byte for major and minor
        // + 6 bytes reserved + the actual tables
        let size = (4 + 4 + 1 + 1 + 6 + tables.len()) as u32;
        let typ: u32 = TagType::Smbios.into();
        let mut bytes = [typ.to_ne_bytes(), size.to_ne_bytes()].concat();
        bytes.push(3);
        bytes.push(0);
        bytes.extend([0; 6]);
        bytes.extend(tables);
        bytes
    }

    /// Tests to parse a string with a terminating null byte from the tag (as the spec defines).
    #[test]
    fn test_parse() {
        let tag = get_bytes();
        let tag = unsafe {
            let ptr = tag.as_ptr() as *const ();
            (slice_from_raw_parts(ptr, tag.len() - METADATA_SIZE) as *const super::SmbiosTag)
                .as_ref()
                .unwrap()
        };
        assert_eq!({ tag.typ }, TagType::Smbios);
        assert_eq!(tag.major, 3);
        assert_eq!(tag.minor, 0);
        assert_eq!(tag.tables, [0xabu8; 24]);
    }
}
