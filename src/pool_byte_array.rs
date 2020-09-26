use gdnative::prelude::*;
use std::io::{Error, ErrorKind, Read, Seek, SeekFrom};
pub struct PoolByteArray {
    byte_array: ByteArray,
    array_seek_pos: u64,
}

/// Wrapper for godot's ByteArray with Seek and Write traits
impl PoolByteArray {
    /// Creates a new PoolByteArray
    ///
    /// # Arguments
    ///
    /// * `byte_array` - ByteArray to wrap around
    pub fn new(byte_array: ByteArray) -> Self {
        PoolByteArray {
            array_seek_pos: 0,
            byte_array: byte_array,
        }
    }
}

impl Seek for PoolByteArray {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, std::io::Error> {
        let array_len = self.byte_array.len() as u64;
        let new_offset = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(offset) => array_len as i64 - 1 - offset,
            SeekFrom::Current(offset) => self.array_seek_pos as i64 + offset,
        };
        if new_offset < 0 {
            return Err(Error::from(ErrorKind::InvalidInput));
        }
        let new_offset = (new_offset % array_len as i64) as u64;
        self.array_seek_pos = new_offset;
        Ok(new_offset)
    }
}

impl Read for PoolByteArray {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let mut bytes_returned: usize = 0;
        let array_len = self.byte_array.len() as usize;
        while bytes_returned < buf.len() && self.array_seek_pos < array_len as u64 {
            buf[bytes_returned] = self.byte_array.get(self.array_seek_pos as i32);
            bytes_returned += 1;
            self.array_seek_pos += 1;
        }
        Ok(bytes_returned)
    }
}
