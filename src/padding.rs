//! A module for `padding` bits into bytes
//!
//! This module contains the trait for defining a padding strategy for `BitReader` and `BitWriter`.

use std::io::{Write};
use std::io::Result as IOResult;

/// **Padding** specifies what sort of padding should be used by
/// [BitReader](../struct.BitReader.html)/[BitWriter](../struct.BitWriter.html)
///
/// This trait can be used to implement custom padding of bits to a whole byte.
pub trait Padding {
    /// Get the maximum size of the padding.
    ///
    /// This is used to determine how many bytes should be passed to
    /// [bits_left](#tymethod.bits_left)
    fn max_size(&self) -> usize;

    /// Pad the last bits of the stream.
    ///
    /// This is called by [BitWriter](../struct.BitWriter.html) on drop to make sure the last bits are
    /// written to the output, using the specified padding. The padding is responsible for writing
    /// the last byte, or else it may lead to unintended loss of data.
    fn pad<W: Write>(&self, last_byte: u8, byte_fill: u8, writer: &mut W) -> IOResult<()>;

    /// Determine how many bits are left to read.
    ///
    /// This is called by [BitReader](../struct.BitReader.html) when encountering the last byte in the
    /// input stream. This will be called with the last `n` bytes of the input stream, where `n`
    /// is [`max_size()`](#tymethod.max_size), or fewer, if there are fewer bytes in the whole input
    /// stream.
    fn bits_left(&self, last_bytes: &[u8]) -> IOResult<usize>;
}

/// **NoPadding** is the default [Padding](trait.Padding.html) used by
/// [BitReader](../struct.BitReader.html)/[BitWriter](../struct.BitWriter.html)
///
/// This does not add any padding to the output stream, apart from filling up the the stream
/// with 0s until the next byte is full.
#[derive(Default, Debug)]
pub struct NoPadding {}

impl NoPadding {
    /// Create a new instance
    pub fn new() -> Self {
        NoPadding {}
    }
}

impl Padding for NoPadding {
    fn max_size(&self) -> usize {
        0
    }

    fn pad<W: Write>(&self, last_byte: u8, byte_fill: u8, writer: &mut W) -> IOResult<()> {
        if byte_fill > 0 {
            writer.write_all(&[last_byte])
        } else {
            Ok(())
        }
    }

    fn bits_left(&self, _: &[u8]) -> IOResult<usize> {
        Ok(0)
    }
}

/// **LengthPadding** can be used encode the number of bits in the bit stream.
///
/// When using this padding, an extra byte is appended at the end of the stream. This byte
/// indicates how many bots in the previous byte are valid.
#[derive(Debug, Default)]
pub struct LengthPadding {}

impl LengthPadding {
    /// Create a new instance
    pub fn new() -> Self {
        LengthPadding {}
    }
}

impl Padding for LengthPadding {
    fn max_size(&self) -> usize {
        2
    }

    fn pad<W: Write>(&self, last_byte: u8, byte_fill: u8, writer: &mut W) -> IOResult<()> {
        if byte_fill > 0 {
            writer.write_all(&[last_byte, byte_fill])
        } else {
            writer.write_all(&[8u8])
        }
    }

    fn bits_left(&self, last_bytes: &[u8]) -> IOResult<usize> {
        if last_bytes.len() == 2 {
            Ok(last_bytes[1] as usize)
        } else {
            Ok(0)
        }
    }
}
