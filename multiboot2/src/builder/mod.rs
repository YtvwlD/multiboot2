//! Module for the builder-feature.

mod information;
pub(crate) mod traits;


pub use information::Multiboot2InformationBuilder;

use core::alloc::Layout;
use core::convert::TryInto;
use core::mem::size_of;
use alloc::alloc::alloc;
use alloc::boxed::Box;

use crate::{TagType, Tag};

/// Create a boxed tag with the given content.
pub(super) fn boxed_dst_tag(typ: TagType, content: &[u8]) -> Box<Tag> {
    // based on https://stackoverflow.com/a/64121094/2192464
    let (layout, size_offset) = Layout::new::<TagType>()
        .extend(Layout::new::<u32>()).unwrap();
    let (layout, inner_offset) = layout.extend(
        Layout::array::<usize>(content.len()).unwrap()
    ).unwrap();
    let ptr = unsafe { alloc(layout) };
    assert!(!ptr.is_null());
    unsafe {
        // initialize the content as good as we can
        ptr.cast::<TagType>().write(typ);
        ptr.add(size_offset).cast::<u32>().write((
            content.len() + size_of::<TagType>() + size_of::<u32>()
        ).try_into().unwrap());
        // initialize body
        let content_ptr = ptr.add(inner_offset);
        for (idx, val) in content.iter().enumerate() {
            content_ptr.add(idx).write(*val);
        }
        Box::from_raw(
            core::ptr::from_raw_parts_mut(
                ptr as *mut (), content.unwrap().len()
            )
        )
    }
}
