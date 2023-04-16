use crate::{Reader, TagType};
#[cfg(feature = "builder")]
use crate::builder::boxed_dst_tag;
#[cfg(feature = "builder")]
use crate::builder::traits::StructAsBytes;

use core::convert::TryInto;
use core::fmt::Debug;
use core::mem;
use core::slice;

#[cfg(feature = "builder")]
use alloc::boxed::Box;
#[cfg(feature = "builder")]
use alloc::vec::Vec;

/// The VBE Framebuffer information Tag.
#[derive(Eq)]
#[repr(C, packed)]
pub struct FramebufferTag {
    typ: TagType,
    size: u32,

    /// Contains framebuffer physical address.
    ///
    /// This field is 64-bit wide but bootloader should set it under 4GiB if
    /// possible for compatibility with payloads which aren’t aware of PAE or
    /// amd64.
    address: u64,

    /// Contains the pitch in bytes.
    pitch: u32,

    /// Contains framebuffer width in pixels.
    width: u32,

    /// Contains framebuffer height in pixels.
    height: u32,

    /// Contains number of bits per pixel.
    bpp: u8,

    /// The type of framebuffer, one of: `Indexed`, `RGB` or `Text`.
    type_no: u8,

    // In the multiboot spec, it has this listed as a u8 _NOT_ a u16.
    // Reading the GRUB2 source code reveals it is in fact a u16.
    _reserved: u16,

    buffer: [u8],
}

impl FramebufferTag {
    #[cfg(feature = "builder")]
    pub fn new(
        address: u64, pitch: u32, width: u32, height: u32, bpp: u8,
        buffer_type: FramebufferType,
    ) -> Box<Self> {
        let mut bytes: Vec<u8> = address.to_le_bytes().into();
        bytes.extend(pitch.to_le_bytes());
        bytes.extend(width.to_le_bytes());
        bytes.extend(height.to_le_bytes());
        bytes.extend(bpp.to_le_bytes());
        bytes.extend(buffer_type.to_bytes());

        let tag = boxed_dst_tag(TagType::Framebuffer, &bytes);
        unsafe { Box::from_raw(Box::into_raw(tag) as *mut Self) }
    }

    /// Contains framebuffer physical address.
    ///
    /// This field is 64-bit wide but bootloader should set it under 4GiB if
    /// possible for compatibility with payloads which aren’t aware of PAE or
    /// amd64.
    pub fn address(&self) -> u64 {
        self.address
    }

    /// Contains the pitch in bytes.
    pub fn pitch(&self) -> u32 {
        self.pitch
    }

    /// Contains framebuffer width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Contains framebuffer height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Contains number of bits per pixel.
    pub fn bpp(&self) -> u8 {
        self.bpp
    }


    /// The type of framebuffer, one of: `Indexed`, `RGB` or `Text`.
    pub fn buffer_type(&self) -> FramebufferType {
        let mut reader = Reader::new(&self.buffer);
        match self.type_no {
            0 => {
                let num_colors = reader.read_u32();
                let palette = unsafe {
                    slice::from_raw_parts(
                        reader.current_address() as *const FramebufferColor,
                        num_colors as usize,
                    )
                } as &'static [FramebufferColor];
                FramebufferType::Indexed { palette }
            }
            1 => {
                let red_pos = reader.read_u8(); // These refer to the bit positions of the LSB of each field
                let red_mask = reader.read_u8(); // And then the length of the field from LSB to MSB
                let green_pos = reader.read_u8();
                let green_mask = reader.read_u8();
                let blue_pos = reader.read_u8();
                let blue_mask = reader.read_u8();
                FramebufferType::RGB {
                    red: FramebufferField {
                        position: red_pos,
                        size: red_mask,
                    },
                    green: FramebufferField {
                        position: green_pos,
                        size: green_mask,
                    },
                    blue: FramebufferField {
                        position: blue_pos,
                        size: blue_mask,
                    },
                }
            }
            2 => FramebufferType::Text,
            no => panic!("Unknown framebuffer type: {}", no),
        }
    }
}

#[cfg(feature = "builder")]
impl StructAsBytes for FramebufferTag {
    fn byte_size(&self) -> usize {
        self.size.try_into().unwrap()
    }
}

impl Debug for FramebufferTag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FramebufferTag")
            .field("typ", &{self.typ})
            .field("size", &{self.size})
            .field("buffer_type", &self.buffer_type())
            .field("address", &{self.address})
            .field("pitch", &{self.pitch})
            .field("width", &{self.width})
            .field("height", &{self.height})
            .field("bpp", &self.bpp)
            .finish()
    }
}

impl PartialEq for FramebufferTag {
    fn eq(&self, other: &Self) -> bool {
        (
            {self.typ} == {other.typ}
            && {self.size} == {other.size}
            && {self.address} == {other.address}
            && {self.pitch} == {other.pitch}
            && {self.width} == {other.width}
            && {self.height} == {other.height}
            && {self.bpp} == {other.bpp}
            && {self.type_no} == {other.type_no}
            && self.buffer == other.buffer
        )
    }
}

/// The type of framebuffer.
#[derive(Debug, PartialEq, Eq)]
pub enum FramebufferType<'a> {
    /// Indexed color.
    Indexed {
        #[allow(missing_docs)]
        palette: &'a [FramebufferColor],
    },

    /// Direct RGB color.
    #[allow(missing_docs)]
    RGB {
        red: FramebufferField,
        green: FramebufferField,
        blue: FramebufferField,
    },

    /// EGA Text.
    ///
    /// In this case the framebuffer width and height are expressed in
    /// characters and not in pixels.
    ///
    /// The bpp is equal 16 (16 bits per character) and pitch is expressed in bytes per text line.
    Text,
}

impl<'a> FramebufferType<'a> {
    #[cfg(feature = "builder")]
    fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::new();
        match self {
            FramebufferType::Indexed { palette } => {
                v.extend(0u8.to_le_bytes()); // type
                v.extend(0u16.to_le_bytes()); // reserved
                v.extend((palette.len() as u32).to_le_bytes());
                for color in palette.iter() {
                    v.extend(color.struct_as_bytes());
                }
            },
            FramebufferType::RGB {red, green, blue } => {
                v.extend(1u8.to_le_bytes()); // type
                v.extend(0u16.to_le_bytes()); // reserved
                v.extend(red.struct_as_bytes());
                v.extend(green.struct_as_bytes());
                v.extend(blue.struct_as_bytes());
            },
            FramebufferType::Text => {
                v.extend(2u8.to_le_bytes()); // type
                v.extend(0u16.to_le_bytes()); // reserved
            },
        }
        v
    }
}

/// An RGB color type field.
#[derive(Debug, PartialEq, Eq)]
pub struct FramebufferField {
    /// Color field position.
    pub position: u8,

    /// Color mask size.
    pub size: u8,
}

#[cfg(feature = "builder")]
impl StructAsBytes for FramebufferField {
    fn byte_size(&self) -> usize {
        mem::size_of::<Self>()
    }
}

/// A framebuffer color descriptor in the palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C, packed)] // only repr(C) would add unwanted padding at the end
pub struct FramebufferColor {
    /// The Red component of the color.
    pub red: u8,

    /// The Green component of the color.
    pub green: u8,

    /// The Blue component of the color.
    pub blue: u8,
}

#[cfg(feature = "builder")]
impl StructAsBytes for FramebufferColor {
    fn byte_size(&self) -> usize {
        mem::size_of::<Self>()
    }
}
