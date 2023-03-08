//! Module for the helper trait [`StructAsBytes`].

/// Trait for all tags that helps to create a byte array from the tag.
/// Useful in builders to construct a byte vector that
/// represents the Multiboot2 information with all its tags.
pub(crate) trait StructAsBytes {
    /// Returns the size in bytes of the struct.
    /// This can be either the "size" field of tags or the compile-time size
    /// (if known).
    fn byte_size(&self) -> usize;

    /// Returns a byte pointer to the begin of the struct.
    fn as_ptr(&self) -> *const u8 {
        self as *const Self as *const u8
    }

    /// Returns the structure as a vector of its bytes.
    /// The length is determined by [`size`].
    fn struct_as_bytes(&self) -> alloc::vec::Vec<u8> {
        let ptr = self.as_ptr();
        let mut vec = alloc::vec::Vec::with_capacity(self.byte_size());
        for i in 0..self.byte_size() {
            vec.push(unsafe { *ptr.add(i) })
        }
        vec
    }
}
