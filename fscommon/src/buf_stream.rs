use core::cmp;
use super::io;
use io::{Read, Write, Seek};
//use io::prelude::*;

const BUF_SIZE: usize = 512;

/// The `BufStream` struct adds buffering to underlying file or device.
///
/// It is basically composition of `BufReader` and `BufWritter`.
/// Buffer size is fixed to 512 to avoid dynamic allocation.
/// `BufStream` automatically flushes itself when being dropped.
pub struct BufStream<T: Read+Write+Seek>  {
    inner: T,
    buf: [u8; BUF_SIZE],
    len: usize,
    pub pos: usize,
    write: bool,
}

impl<T: Read+Write+Seek> BufStream<T> {
    /// Creates a new `BufStream` object for a given inner stream.
    pub fn new(inner: T) -> Self {
        BufStream {
            inner,
            buf: [0; BUF_SIZE],
            pos: 0,
            len: 0,
            write: false,
        }
    }

    fn flush_buf(&mut self) -> io::Result<()> {
        if self.write {
            self.inner.write_all(&self.buf[..self.pos])?;
            self.pos = 0;
        }
        Ok(())
    }

    fn make_reader(&mut self) -> io::Result<()> {
        if self.write {
            self.flush_buf()?;
            self.write = false;
            self.len = 0;
            self.pos = 0;
        }
        Ok(())
    }

    fn make_writter(&mut self) -> io::Result<()> {
        if !self.write {
            self.inner.seek(io::SeekFrom::Current(-(self.len as i64 - self.pos as i64)))?;
            self.write = true;
            self.len = 0;
            self.pos = 0;
        }
        Ok(())
    }

    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.make_reader()?;
        if self.pos >= self.len {
            debug_assert!(self.pos == self.len);
            self.len = self.inner.read(&mut self.buf)?;
            self.pos = 0;
        }
        Ok(&self.buf[self.pos..self.len])
    }

    fn consume(&mut self, amt: usize) {
        self.pos = cmp::min(self.pos + amt, self.len);
    }
}

//#[cfg(any(feature = "std", feature = "core_io/collections"))]
impl<T: Read+Write+Seek> io::BufRead for BufStream<T> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        BufStream::fill_buf(self)
    }

    fn consume(&mut self, amt: usize) {
        BufStream::consume(self, amt)
    }
}

impl<T: Read+Write+Seek> Read for BufStream<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Make sure we are in read mode
        self.make_reader()?;
        // Check if this read is bigger than buffer size
        if self.pos == self.len && buf.len() >= BUF_SIZE {
            return self.inner.read(buf);
        }
        let nread = {
            let mut rem = self.fill_buf()?;
            rem.read(buf)?
        };
        self.consume(nread);
        Ok(nread)
    }
}

impl<T: Read+Write+Seek> Write for BufStream<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Make sure we are in write mode
        self.make_writter()?;
        if self.pos + buf.len() > BUF_SIZE {
            self.flush_buf()?;
            if buf.len() >= BUF_SIZE {
                return self.inner.write(buf);
            }
        }
        let written = (&mut self.buf[self.pos..]).write(buf)?;
        self.pos += written;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_buf()?;
        self.inner.flush()
    }
}

impl<T: Read+Write+Seek> Seek for BufStream<T> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.flush_buf()?;
        let new_pos = match pos {
            io::SeekFrom::Current(x) => io::SeekFrom::Current(x - (self.len as i64 - self.pos as i64)),
            _ => pos,
        };
        self.pos = 0;
        self.len = 0;
        self.inner.seek(new_pos)
    }
}

impl<T: Read+Write+Seek> Drop for BufStream<T> {
    fn drop(&mut self) {
        if let Err(err) = self.flush() {
            error!("flush failed {}", err);
        }
    }
}

#[cfg(test)]

mod tests {
    use super::*;
    use alloc::string::{String, ToString};

    #[test]
    fn it_works() {
        let mut buf = "Test data".to_string().into_bytes();
        buf.resize(200, 0);
        let cur = io::Cursor::new(&mut buf[..]);
        let mut buf_stream = BufStream::new(cur);

        let mut data:[u8; 100] = [0; 100];
        buf_stream.read(&mut data).unwrap();
        assert_eq!(data[0] as char, 'T' as char);
        let mut s = String::new();
        s.extend(data.iter().filter(|&c| *c != 0).map(|&c| c as char));
        assert_eq!(s, "Test data");

        buf_stream.seek(io::SeekFrom::Start(5)).unwrap();
        let mut data:[u8; 100] = [0; 100];
        buf_stream.read(&mut data).unwrap();
        let mut s = String::new();
        s.extend(data.iter().filter(|&c| *c != 0).map(|&c| c as char));
        assert_eq!(s, "data");

        buf_stream.write_all("\nHello".as_bytes()).unwrap();
        buf_stream.seek(io::SeekFrom::Start(0)).unwrap();
        let mut data:[u8; 200] = [0; 200];
        buf_stream.read(&mut data).unwrap();
        let mut s = String::new();
        s.extend(data.iter().filter(|&c| *c != 0).map(|&c| c as char));
        assert_eq!(s, "Test data\nHello");
        //data.clear();
        //buf_stream.read_line(&mut data).unwrap();
        //assert_eq!(data, "Hello");
    }
}
