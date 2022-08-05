use super::device::fsio;
use fatfs::{IoBase, IoError, Read, Seek, SeekFrom, Write};

/*
impl From<SeekFrom> for fsio::SeekFrom {
    fn from(from: SeekFrom) -> Self {
        match from {
            SeekFrom::Start(n) => fsio::SeekFrom::Start(n),
            SeekFrom::End(n) => fsio::SeekFrom::End(n),
            SeekFrom::Current(n) => fsio::SeekFrom::Current(n),
        }
    }
}

impl From<fsio::SeekFrom> for SeekFrom {
    fn from(from: fsio::SeekFrom) -> Self {
        match from {
            fsio::SeekFrom::Start(n) => SeekFrom::Start(n),
            fsio::SeekFrom::End(n) => SeekFrom::End(n),
            fsio::SeekFrom::Current(n) => SeekFrom::Current(n),
        }
    }
}
*/

pub struct TransferError(fsio::Error);

impl From<fsio::Error> for TransferError {
    fn from(from: fsio::Error) -> Self {
        TransferError(from)
    }
}

impl Into<fsio::Error> for TransferError {
    fn into(self) -> fsio::Error {
        self.0
    }
}

impl core::fmt::Debug for TransferError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl IoError for TransferError {
    fn is_interrupted(&self) -> bool {
        self.0.kind() == fsio::ErrorKind::Interrupted
    }

    fn new_unexpected_eof_error() -> Self {
        Self(fsio::ErrorKind::UnexpectedEof.into())
    }

    fn new_write_zero_error() -> Self {
        Self(fsio::ErrorKind::WriteZero.into())
    }
}

pub struct IoWrapper<T> {
    inner: T,
}

impl<T> IoWrapper<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T> IoBase for IoWrapper<T> {
    type Error = TransferError;
}

impl<T: fsio::Read> Read for IoWrapper<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.inner.read(buf).map_err(|e| TransferError(e))
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.inner.read_exact(buf).map_err(|e| TransferError(e))
    }
}

impl<T: fsio::Write> Write for IoWrapper<T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.inner.write(buf).map_err(|e| TransferError(e))
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.inner.write_all(buf).map_err(|e| TransferError(e))
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush().map_err(|e| TransferError(e))
    }
}

impl<T: fsio::Seek> Seek for IoWrapper<T> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        self.inner
            .seek(match pos {
                SeekFrom::Start(n) => fsio::SeekFrom::Start(n),
                SeekFrom::End(n) => fsio::SeekFrom::End(n),
                SeekFrom::Current(n) => fsio::SeekFrom::Current(n),
            })
            .map_err(|e| TransferError(e))
    }
}

impl<T> From<T> for IoWrapper<T> {
    fn from(from: T) -> Self {
        Self::new(from)
    }
}
