use std::io::{Read, Result, Seek, SeekFrom};
use std::marker::Sync;
use std::sync::{Mutex, RwLock};

use symphonia::core::io::MediaSource;

pub struct ReadSeekSource<T: Read + Seek + Send + Sync> {
    inner: Mutex<T>,
    byte_len: RwLock<Option<Option<u64>>>, // One option is for lazy calculation
}

// Copied from std Seek::stream_len since its unstable
fn stream_len(stream: &mut impl Seek) -> std::io::Result<u64> {
    let old_pos = stream.stream_position()?;
    let len = stream.seek(SeekFrom::End(0))?;

    // Avoid seeking a third time when we were already at the end of the
    // stream. The branch is usually way cheaper than a seek operation.
    if old_pos != len {
        stream.seek(SeekFrom::Start(old_pos))?;
    }

    Ok(len)
}

impl<T: Read + Seek + Send + Sync> ReadSeekSource<T> {
    /// Instantiates a new `ReadSeekSource<T>` by taking ownership and wrapping the provided
    /// `Read + Seek`er.
    pub fn new(inner: T) -> Self {
        ReadSeekSource {
            inner: Mutex::new(inner),
            byte_len: RwLock::new(None),
        }
    }
}

impl<T: Read + Seek + Send + Sync> MediaSource for ReadSeekSource<T> {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        // Check if length was calculated before
        let byte_len = self.byte_len.read().unwrap();
        if let Some(cached) = *byte_len {
            return cached;
        }
        std::mem::drop(byte_len); // Release read lock

        let mut inner = self.inner.lock().unwrap();
        let calculated_stream_len = match stream_len(&mut *inner) {
            Ok(len) => Some(len),
            Err(_) => None, // Ignore error, cache failure
        };
        *self.byte_len.write().unwrap() = Some(calculated_stream_len);
        calculated_stream_len
    }
}

impl<T: Read + Seek + Send + Sync> Read for ReadSeekSource<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.get_mut().unwrap().read(buf)
    }
}

impl<T: Read + Seek + Send + Sync> Seek for ReadSeekSource<T> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.inner.get_mut().unwrap().seek(pos)
    }
}
