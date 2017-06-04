//! **bitstream** is a crate for dealing with single bit input and output
//!
//! This crate provides a writer that can write single bits to an
//! underlying Write implementation, and read them back using a reader
//! implementation.

use std::io::{Write, Read};
use std::io::Result as IOResult;

pub mod padding;
pub use padding::{Padding, NoPadding, LengthPadding};

/// **BitWriter** is a writer for single bit values
///
/// Bits will be grouped to a single byte before writing to the inner writer.
/// The first Bit will be the most significant bit of the byte.
///
/// When dropping this writer, any remaining bits will be written according to the padding used.
/// The default padding is [NoPadding](struct.NoPadding.html)
///
/// # Examples
///
/// ```
/// extern crate bitstream;
///
/// let vec = Vec::new();
/// let mut bit_writer = bitstream::BitWriter::new(vec);
///
/// assert!(bit_writer.write_bit(true).is_ok());
/// assert!(bit_writer.write_bit(false).is_ok());
/// ```
pub struct BitWriter<W, P> where W: Write, P: Padding {
    inner: W,
    padder: P,
    last_byte: u8,
    last_fill: u8,
}


/// **BitReader** is a reader for single bit values
///
/// This reader expects the last byte in the input to contain the number of significant bits in the
/// second to last byte. This is the same format produced by [BitWriter]
///
/// # Examples
/// ```
/// extern crate bitstream;
/// use std::io::Cursor;
///
/// let vec = vec![192, 2];
/// let mut bit_reader = bitstream::BitReader::new(Cursor::new(vec));
/// let first_read = bit_reader.read_bit();
/// assert!(first_read.is_ok());
/// let option = first_read.unwrap();
/// assert!(option.is_some());
/// assert!(option.unwrap());
/// ```
pub struct BitReader<R, P> where R: Read, P: Padding {
    padder: P,
    inner: R,
    ended: bool,
    fill: usize,
    current: u8,
    buffer: Box<[u8]>,
    bits_left: usize,
}


impl<W> BitWriter<W, NoPadding> where W: Write {
    /// Create a new BitWriter with no padding, writing to the inner writer.
    pub fn new(write: W) -> Self {
        BitWriter::with_padding(write, NoPadding::new())
    }
}

impl<W, P> BitWriter<W, P> where W: Write, P: Padding {
    /// Create a new BitWriter with the given padding
    pub fn with_padding(write: W, padder: P) -> Self {
        BitWriter {
            inner: write,
            padder: padder,
            last_byte: 0,
            last_fill: 0,
        }
    }

    /// Write a single bit to the inner writer.
    ///
    /// # Failures
    /// Returns an error if the inner writer returns an error
    pub fn write_bit(&mut self, bit: bool) -> IOResult<()> {
        if bit {
            let data = 128u8 >> self.last_fill;
            self.last_byte |= data;
        }

        self.last_fill += 1;
        if self.last_fill == 8 {
            self.inner.write_all(&[self.last_byte])?;
            self.last_byte = 0;
            self.last_fill = 0
        }
        Ok(())
    }
}

impl<W, P> Drop for BitWriter<W, P> where W: Write, P: Padding {
    fn drop(&mut self) {
        let _ = self.padder.pad(self.last_byte, self.last_fill, &mut self.inner);
    }
}


impl<R> BitReader<R, NoPadding> where R: Read {
    /// Create a new BitReader, with no padding, reading from the inner reader.
    pub fn new(reader: R) -> Self {
        BitReader::with_padding(reader, NoPadding::new())
    }
}

impl<R, P> BitReader<R, P> where R: Read, P: Padding {

    /// Create a new BitReader, using the supplied padding.
    ///
    /// This can be used to supply a custom padding to the bit reader.
    ///
    /// # Examples
    /// ```
    /// extern crate bitstream;
    /// use std::io::Cursor;
    ///
    /// let vec = vec![192, 2];
    /// let mut bit_reader = bitstream::BitReader::with_padding(Cursor::new(vec),
    ///                                                         bitstream::LengthPadding::new());
    /// let _ = bit_reader.read_bit();
    /// let _ = bit_reader.read_bit();
    /// let last = bit_reader.read_bit();
    /// assert!(last.is_ok());
    /// /// None indicates there is nothing left to read
    /// assert!(last.unwrap().is_none());
    /// ```
    pub fn with_padding(reader: R, padder: P) -> Self {
        let buf_size = padder.max_size() + 1;
        let buffer = vec![0; buf_size];

        BitReader {
            inner: reader,
            padder: padder,
            fill: 0,
            ended: false,
            buffer: buffer.into_boxed_slice(),
            current: 0,
            bits_left: 0,
        }
    }

    fn fill_buffer(&mut self) -> IOResult<()> {
        while !self.ended && self.fill != self.buffer.len() {
            match self.inner.read(&mut self.buffer[self.fill..]) {
                Ok(0) => {
                    self.ended = true;
                    let buf_pad_start = if self.fill < self.buffer.len() {
                        0
                    } else {
                        1
                    };
                    self.bits_left = self.padder.bits_left(&self.buffer[buf_pad_start..self.fill])?;
                }
                Ok(n) => {
                    self.fill += n;
                    self.bits_left = 8;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Read a single bit.
    ///
    /// End of stream is signaled by returning  `Ok(None)`
    ///
    /// # Failures
    /// Will return an error if the inner reader returns one
    pub fn read_bit(&mut self) -> IOResult<Option<bool>> {
        self.fill_buffer()?;
        if self.bits_left == 0 {
            Ok(None)
        } else {
            let res = (self.buffer[0] & (128u8 >> self.current)) == (128u8 >> self.current);
            self.current += 1;
            self.bits_left -= 1;

            if self.current == 8 {
                self.current = 0;
                self.fill -= 1;
                unsafe {
                    std::ptr::copy(self.buffer[1..].as_ptr(), self.buffer[..].as_mut_ptr(), self.buffer.len() - 1);
                }
            }
            Ok(Some(res))
        }
    }
}

impl<R, P> Iterator for BitReader<R, P> where R: Read, P: Padding {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_bit() {
            Ok(opt) => opt,
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_writer_no_pad() {
        let mut vec = Vec::new();
        {
            let mut bit_writer = BitWriter::new(&mut vec);
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(false).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(false).is_ok());
            assert!(bit_writer.write_bit(false).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
        }
        assert_eq!(vec.len(), 2);
        assert_eq!(vec[0], 217);
        assert_eq!(vec[1], 192);
    }

    #[test]
    fn test_writer_no_pad_empty() {
        let mut vec = Vec::new();
        {
            BitWriter::new(&mut vec);
        }
        assert_eq!(vec.len(), 0);
    }

    #[test]
    fn test_reader_no_pad() {
        let mut vec = Cursor::new(vec![200, 192]);
        let mut bit_reader = BitReader::new(&mut vec);
        assert_eq!(bit_reader.read_bit().unwrap().unwrap(), true);
        assert_eq!(bit_reader.read_bit().unwrap().unwrap(), true);
        assert_eq!(bit_reader.read_bit().unwrap().unwrap(), false);
        assert_eq!(bit_reader.read_bit().unwrap().unwrap(), false);
        assert_eq!(bit_reader.read_bit().unwrap().unwrap(), true);
        assert_eq!(bit_reader.read_bit().unwrap().unwrap(), false);
        assert_eq!(bit_reader.read_bit().unwrap().unwrap(), false);
        assert_eq!(bit_reader.read_bit().unwrap().unwrap(), false);
        assert_eq!(bit_reader.read_bit().unwrap().unwrap(), true);
        assert_eq!(bit_reader.read_bit().unwrap().unwrap(), true);
        assert_eq!(bit_reader.read_bit().unwrap().unwrap(), false);
        assert_eq!(bit_reader.read_bit().unwrap().unwrap(), false);
        assert_eq!(bit_reader.read_bit().unwrap().unwrap(), false);
        assert_eq!(bit_reader.read_bit().unwrap().unwrap(), false);
        assert_eq!(bit_reader.read_bit().unwrap().unwrap(), false);
        assert_eq!(bit_reader.read_bit().unwrap().unwrap(), false);
        assert!(bit_reader.read_bit().unwrap().is_none());
    }

    #[test]
    fn test_reader_no_pad_empty() {
        let mut vec = Cursor::new(&[]);
        let mut bit_reader = BitReader::new(&mut vec);
        assert!(bit_reader.read_bit().unwrap().is_none());
    }

    #[test]
    fn test_writer_length_pad() {
        let mut vec = Vec::new();
        {
            let mut bit_writer = BitWriter::with_padding(&mut vec, LengthPadding::new());
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(false).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(false).is_ok());
            assert!(bit_writer.write_bit(false).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
        }
        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0], 217);
        assert_eq!(vec[1], 192);
        assert_eq!(vec[2], 2);
    }

    #[test]
    fn test_writer_length_pad_empty() {
        let mut vec = Vec::new();
        {
            BitWriter::with_padding(&mut vec, LengthPadding::new());
        }
        assert_eq!(vec.len(), 1);
        assert_eq!(vec[0], 8);
    }

    #[test]
    fn test_write_read_length_pad_empty() {
        let mut vec = Vec::new();
        {
            BitWriter::with_padding(&mut vec, LengthPadding::new());
        }
        {
            let mut cur = Cursor::new(&vec);
            let mut bit_reader = BitReader::with_padding(&mut cur, LengthPadding::new());
            assert!(bit_reader.read_bit().unwrap().is_none());
        }
    }

    #[test]
    fn test_write_read_length_pad() {
        let mut vec = Vec::new();
        {
            let mut bit_writer = BitWriter::with_padding(&mut vec, LengthPadding::new());
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(false).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(false).is_ok());
            assert!(bit_writer.write_bit(false).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
            assert!(bit_writer.write_bit(true).is_ok());
        }
        {
            let mut cur = Cursor::new(&vec);
            let mut bit_reader = BitReader::with_padding(&mut cur, LengthPadding::new());
            assert_eq!(bit_reader.read_bit().unwrap().unwrap(), true);
            assert_eq!(bit_reader.read_bit().unwrap().unwrap(), true);
            assert_eq!(bit_reader.read_bit().unwrap().unwrap(), false);
            assert_eq!(bit_reader.read_bit().unwrap().unwrap(), true);
            assert_eq!(bit_reader.read_bit().unwrap().unwrap(), true);
            assert_eq!(bit_reader.read_bit().unwrap().unwrap(), false);
            assert_eq!(bit_reader.read_bit().unwrap().unwrap(), false);
            assert_eq!(bit_reader.read_bit().unwrap().unwrap(), true);
            assert_eq!(bit_reader.read_bit().unwrap().unwrap(), true);
            assert_eq!(bit_reader.read_bit().unwrap().unwrap(), true);
            assert!(bit_reader.read_bit().unwrap().is_none());
        }
    }
}
