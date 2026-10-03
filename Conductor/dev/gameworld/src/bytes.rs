//! File:       Opus/Conductor/dev/gameworld/src/bytes.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Reading the world's binary files a number at a time.  Every number in
//! them is little-endian (the small end first), the way the machines we
//! run on keep them anyway.  Writing is `to_le_bytes()` onto a Vec, which
//! needs no help.

/// Walks through a file's bytes from the front.  Every read says what it
/// was after if the file runs out first, so a short file is a clear
/// complaint instead of a crash.
pub struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Reader<'a> {
        Reader { bytes, at: 0 }
    }

    /// The next `count` bytes.
    pub fn take(&mut self, count: usize, what: &str) -> Result<&'a [u8], String> {
        let end = self.at.checked_add(count)
            .filter(|&end| end <= self.bytes.len())
            .ok_or_else(|| format!("the file ends before {what}"))?;
        // Rust note: copying the reference out first is what lets the bytes
        // handed back live as long as the file's bytes do, not only as
        // long as this call.
        let bytes: &'a [u8] = self.bytes;
        self.at = end;
        Ok(&bytes[end - count..end])
    }

    pub fn u8(&mut self, what: &str) -> Result<u8, String> {
        Ok(self.take(1, what)?[0])
    }

    pub fn u16(&mut self, what: &str) -> Result<u16, String> {
        let bytes = self.take(2, what)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn i16(&mut self, what: &str) -> Result<i16, String> {
        let bytes = self.take(2, what)?;
        Ok(i16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn u64(&mut self, what: &str) -> Result<u64, String> {
        let bytes = self.take(8, what)?;
        let mut eight = [0u8; 8];
        eight.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(eight))
    }

    /// Checks the tag every one of our files starts with, and the version
    /// after it.  A file of another kind, or from a newer Conductor, is
    /// turned away here.
    pub fn tag_and_version(&mut self, tag: &[u8; 8], version: u16) -> Result<(), String> {
        self.tag_and_versions(tag, version, version).map(|_| ())
    }

    /// The same, for a file this Conductor reads more than one version of:
    /// any from `oldest` to `newest`.  Hands back the version it found.
    pub fn tag_and_versions(&mut self, tag: &[u8; 8], oldest: u16, newest: u16) -> Result<u16, String> {
        if self.take(8, "its tag")? != tag {
            return Err(format!("it doesn't start with {}", String::from_utf8_lossy(tag)));
        }
        let found = self.u16("its version")?;
        if found < oldest || found > newest {
            let reads = if oldest == newest {
                format!("version {newest}")
            } else {
                format!("versions {oldest} to {newest}")
            };
            return Err(format!("it's version {found}, and this Conductor reads {reads}"));
        }
        Ok(found)
    }

    /// Whatever is left, as a complaint if there's any.  A file with more
    /// in it than it should have is as wrong as one that's short.
    pub fn finish(&self) -> Result<(), String> {
        match self.bytes.len() - self.at {
            0 => Ok(()),
            extra => Err(format!("it has {extra} bytes more than it should")),
        }
    }
}
