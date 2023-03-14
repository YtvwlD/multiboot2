use crate::TagTypeId;
use crate::TagType;
#[cfg(feature = "builder")]
use crate::builder::boxed_dst_tag;
#[cfg(feature = "builder")]
use crate::builder::traits::StructAsBytes;

use core::convert::TryInto;
use core::fmt::Debug;
use core::mem;
use core::str::Utf8Error;

#[cfg(feature = "builder")]
use alloc::ffi::CString;

#[cfg(feature = "builder")]
use alloc::boxed::Box;

const METADATA_SIZE: usize = mem::size_of::<TagType>() + mem::size_of::<u32>();


/// This tag contains the name of the bootloader that is booting the kernel.
///
/// The name is a normal C-style UTF-8 zero-terminated string that can be
/// obtained via the `name` method.

#[repr(C, packed)] // only repr(C) would add unwanted padding before first_section
pub struct BootLoaderNameTag {
    typ: TagTypeId,
    size: u32,
    /// Null-terminated UTF-8 string
    string: [u8],
}

impl BootLoaderNameTag {
    #[cfg(feature = "builder")]
    pub fn new(name: &str) -> Box<Self> {
        // allocate a C string
        let cstr = CString::new(name)
            .expect("failed to create CString");
        let tag = boxed_dst_tag(
            TagType::BootLoaderName, cstr.as_bytes_with_nul(),
        );
        unsafe { Box::from_raw(Box::into_raw(tag) as *mut Self) }
    }

    /// Read the name of the bootloader that is booting the kernel.
    /// This is an null-terminated UTF-8 string. If this returns `Err` then perhaps the memory
    /// is invalid or the bootloader doesn't follow the spec.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # let boot_info = unsafe { multiboot2::load(0xdeadbeef).unwrap() };
    /// if let Some(tag) = boot_info.boot_loader_name_tag() {
    ///     assert_eq!(Ok("GRUB 2.02~beta3-5"), tag.name());
    /// }
    /// ```
    pub fn name(&self) -> Result<&str, Utf8Error> {
        use core::{slice, str};
        // strlen without null byte
        let strlen = self.size as usize - METADATA_SIZE - 1;
        let bytes = unsafe { slice::from_raw_parts((&self.string[0]) as *const u8, strlen) };
        str::from_utf8(bytes)
    }
}

#[cfg(feature = "builder")]
impl StructAsBytes for BootLoaderNameTag {
    fn byte_size(&self) -> usize {
        self.size.try_into().unwrap()
    }
}

impl Debug for BootLoaderNameTag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BootLoaderNameTag")
            .field("typ", &{self.typ})
            .field("size", &{self.size})
            .field("name", &self.name())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use core::ptr::slice_from_raw_parts;

    use crate::{TagType, boot_loader_name::METADATA_SIZE};

    const MSG: &str = "hello";

    /// Returns the tag structure in bytes in native endian format.
    fn get_bytes() -> std::vec::Vec<u8> {
        // size is: 4 bytes for tag + 4 bytes for size + length of null-terminated string
        let size = (4 + 4 + MSG.as_bytes().len() + 1) as u32;
        [
            &((TagType::BootLoaderName.val()).to_ne_bytes()),
            &size.to_ne_bytes(),
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
            let ptr = tag.as_ptr() as *const ();
            (slice_from_raw_parts(
                ptr, tag.len() - METADATA_SIZE
            ) as *const super::BootLoaderNameTag)
                .as_ref()
                .unwrap()
        };
        assert_eq!({ tag.typ }, TagType::BootLoaderName);
        assert_eq!(tag.name().expect("must be valid UTF-8"), MSG);
    }
}
