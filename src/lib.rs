use std::io::{Write, Read};
use std::io::Result as IOResult;

pub struct BitWriter<W> where W: Write {
    inner: W,
    last_byte: u8,
    last_fill: u8,
}

pub struct BitReader<R> where R: Read {
    inner: R,
    ended: bool,
    fill: u8,
    current: u8,
    buffer: [u8; 3],
    byte_fill: u8,
}

impl<W> BitWriter<W> where W: Write {
    pub fn new(write: W) -> Self {
        BitWriter {
            inner: write,
            last_byte: 0,
            last_fill: 0,
        }
    }

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

impl<W> Drop for BitWriter<W> where W: Write {
    fn drop(&mut self) {
        if self.last_fill > 0 {
            let _ = self.inner.write_all(&[self.last_byte, self.last_fill]);
        } else {
            let _ = self.inner.write_all(&[8u8]);
        }
    }
}

impl<R> BitReader<R> where R: Read {
    pub fn new(reader: R) -> Self {
        BitReader {
            inner: reader,
            fill: 0,
            ended: false,
            buffer: [0, 0, 0],
            current: 0,
            byte_fill: 8,
        }
    }

    fn fill_buffer(&mut self) -> IOResult<()> {
        while !self.ended && self.fill != 3 {
            match self.inner.read(&mut self.buffer[self.fill as usize..]) {
                Ok(0) => {
                    self.ended = true;
                    self.fill -= 1;
                    self.byte_fill = self.buffer[self.fill as usize];
                },
                Ok(n) => self.fill += n as u8,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    pub fn read_bit(&mut self) -> IOResult<Option<bool>> {
        if self.fill > 0 && self.current == self.byte_fill {
            self.buffer = [self.buffer[1], self.buffer[2], 0];
            self.current = 0;
            self.fill -= 1;
        }
        self.fill_buffer()?;
        if self.fill > 0 {
            let res = (self.buffer[0] & (128u8 >> self.current)) == (128u8 >> self.current);
            self.current += 1;
            Ok(Some(res))
        } else {
            Ok(None)
        }
    }
}

impl<R> Iterator for BitReader<R> where R: Read {
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
    fn test_writer() {
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
        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0], 217);
        assert_eq!(vec[1], 192);
        assert_eq!(vec[2], 2);
    }

    #[test]
    fn test_reader() {
        let mut vec = Cursor::new(vec![200, 192, 2]);
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
        assert!(bit_reader.read_bit().unwrap().is_none());
    }

    #[test]
    fn test_write_read() {
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
        {
            let mut cur = Cursor::new(&vec);
            let mut bit_reader = BitReader::new(&mut cur);
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