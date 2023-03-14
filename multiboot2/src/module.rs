#[cfg(feature = "builder")]
use crate::builder::boxed_dst_tag;
#[cfg(feature = "builder")]
use crate::builder::traits::StructAsBytes;
use crate::tag_type::{Tag, TagIter, TagType, TagTypeId};

use core::convert::TryInto;
use core::fmt::{Debug, Formatter};
use core::mem;
use core::str::Utf8Error;

#[cfg(feature = "builder")]
use alloc::ffi::CString;

#[cfg(feature = "builder")]
use alloc::boxed::Box;

const METADATA_SIZE: usize = mem::size_of::<TagTypeId>() + 3 * mem::size_of::<u32>();

/// This tag indicates to the kernel what boot module was loaded along with
/// the kernel image, and where it can be found.
#[repr(C, packed)] // only repr(C) would add unwanted padding near name_byte.
pub struct ModuleTag {
    typ: TagTypeId,
    size: u32,
    mod_start: u32,
    mod_end: u32,
    /// Null-terminated UTF-8 string
    cmdline_str: [u8],
}

impl ModuleTag {
    #[cfg(feature = "builder")]
    pub fn new(start: u32, end: u32, cmdline: &str) -> Box<Self> {
        // allocate a C string

        let cstr = CString::new(cmdline).expect("failed to create CString");
        let start_bytes = start.to_le_bytes();
        let end_bytes = end.to_le_bytes();
        let mut content_bytes = [start_bytes, end_bytes].concat();
        content_bytes.extend_from_slice(cstr.as_bytes_with_nul());
        let tag = boxed_dst_tag(TagType::Module.into(), content_bytes.as_slice());
        unsafe { Box::from_raw(Box::into_raw(tag) as *mut Self) }
    }

    /// Returns the cmdline of the module.
    /// This is an null-terminated UTF-8 string. If this returns `Err` then perhaps the memory
    /// is invalid or the bootloader doesn't follow the spec.
    ///
    /// For example: If the GRUB configuration contains
    /// `module2 /foobar/some_boot_module --test cmdline-option` then this method
    /// will return `--test cmdline-option`.
    pub fn cmdline(&self) -> Result<&str, Utf8Error> {
        use core::{slice, str};
        // strlen without null byte
        let strlen = self.size as usize - METADATA_SIZE - 1;
        let bytes = unsafe { slice::from_raw_parts((&self.cmdline_str[0]) as *const u8, strlen) };
        str::from_utf8(bytes)
    }

    /// Start address of the module.
    pub fn start_address(&self) -> u32 {
        self.mod_start
    }

    /// End address of the module
    pub fn end_address(&self) -> u32 {
        self.mod_end
    }

    /// The size of the module/the BLOB in memory.
    pub fn module_size(&self) -> u32 {
        self.mod_end - self.mod_start
    }
}

#[cfg(feature = "builder")]
impl StructAsBytes for ModuleTag {
    fn byte_size(&self) -> usize {
        self.size.try_into().unwrap()
    }
}

impl Debug for ModuleTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ModuleTag")
            .field("type", &{ self.typ })
            .field("size (tag)", &{ self.size })
            .field("size (module)", &self.module_size())
            .field("mod_start", &(self.mod_start as *const usize))
            .field("mod_end", &(self.mod_end as *const usize))
            .field("cmdline", &self.cmdline())
            .finish()
    }
}

pub fn module_iter(iter: TagIter) -> ModuleIter {
    ModuleIter { iter }
}

/// An iterator over all module tags.
#[derive(Clone)]
pub struct ModuleIter<'a> {
    iter: TagIter<'a>,
}

impl<'a> Iterator for ModuleIter<'a> {
    type Item = &'a ModuleTag;

    fn next(&mut self) -> Option<&'a ModuleTag> {
        self.iter
            .find(|tag| tag.typ == TagType::Module)
            .map(|tag| unsafe {
                let (ptr, _) = (tag as *const Tag).to_raw_parts();
                &*(core::ptr::from_raw_parts(ptr, tag.size as usize - METADATA_SIZE)
                    as *const ModuleTag)
            })
    }
}

impl<'a> Debug for ModuleIter<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut list = f.debug_list();
        self.clone().for_each(|tag| {
            list.entry(&tag);
        });
        list.finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::{module::METADATA_SIZE, tag_type::TagType};

    const MSG: &str = "hello";

    /// Returns the tag structure in bytes in native endian format.
    fn get_bytes() -> std::vec::Vec<u8> {
        // size is: 4 bytes for tag + 4 bytes for size + length of null-terminated string
        //          4 bytes mod_start + 4 bytes mod_end
        let size = (4 + 4 + 4 + 4 + MSG.as_bytes().len() + 1) as u32;
        [
            &((TagType::Module.val()).to_ne_bytes()),
            &size.to_ne_bytes(),
            &0_u32.to_ne_bytes(),
            &0_u32.to_ne_bytes(),
            MSG.as_bytes(),
            // Null Byte
            &[0],
        ]
        .iter()
        .flat_map(|bytes| bytes.iter())
        .copied()
        .collect()
    }

    /// Tests to parse a string with a terminating null byte from the tag (as the spec defines).
    #[test]
    fn test_parse_str() {
        let tag = get_bytes();
        let tag = unsafe {
            let (ptr, _) = tag.as_ptr().to_raw_parts();
            &*(core::ptr::from_raw_parts(ptr, tag.len() - METADATA_SIZE) as *const super::ModuleTag)
        };
        assert_eq!({ tag.typ }, TagType::Module.val());
        assert_eq!(tag.cmdline().expect("must be valid UTF-8"), MSG);
    }
}
