//! File:       Opus/Conductor/dev/gameworld/src/heights.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! A region's heights file: how high the dirt is in every column of a
//! region whose ground is "heights" (Omega).  Made once, when the world is
//! made, and kept, so the hills never move (Jacob, 2026-09-30: "Omega
//! chunks will need to be saved with their bumpiness").  An untouched
//! chunk is built from it; a chunk somebody has changed has its own file,
//! and that always wins.
//!
//! `Content/world/Regions/Omega/omega.heights`:
//!
//! ```text
//! 8 bytes   OPUSHGHT
//! u16       version, 1
//! u64       the seed it was made from, the same as region.map's
//! i16       the westmost column's x
//! i16       the southmost column's z
//! u16       how many columns east-west
//! u16       how many columns north-south
//! i8 x every column   the dirt's height, -5 to 5: the south line first,
//!                     in a line west to east.
//! ```
//!
//! Omega is 8192 by 16384 columns, a byte each, so about 134 MB.  Saving
//! every Omega chunk whole instead would have been about 8 GB.

use std::sync::Arc;

use crate::bytes::Reader;
use crate::noise;

const TAG: &[u8; 8] = b"OPUSHGHT";
const VERSION: u16 = 1;

/// The bytes before the heights start.
const HEADER: usize = 8 + 2 + 8 + 2 + 2 + 2 + 2;

/// A heights file, read.  The bytes are shared with DiskMan's copy, so
/// holding this costs no second 134 MB.
pub struct Heights {
    seed: u64,
    west: i32,
    south: i32,
    width: i32,
    depth: i32,
    contents: Arc<Vec<u8>>,
}

impl Heights {
    /// A heights file read back.  `contents` is the whole file, as DiskMan
    /// hands it over.
    pub fn from_contents(contents: Arc<Vec<u8>>) -> Result<Heights, String> {
        let mut reader = Reader::new(&contents);
        reader.tag_and_version(TAG, VERSION)?;
        let seed = reader.u64("the seed")?;
        let west = reader.i16("the westmost column")? as i32;
        let south = reader.i16("the southmost column")? as i32;
        let width = reader.u16("the width")? as i32;
        let depth = reader.u16("the depth")? as i32;
        reader.take((width * depth) as usize, "the heights")?;
        reader.finish()?;
        Ok(Heights { seed, west, south, width, depth, contents })
    }

    /// The seed it was made from.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// The box it covers: westmost x, southmost z, width, depth.
    pub fn covers(&self) -> (i32, i32, i32, i32) {
        (self.west, self.south, self.width, self.depth)
    }

    /// How high the dirt is at column x,z.  `None` outside the file.
    pub fn at(&self, x: i32, z: i32) -> Option<i32> {
        let across = x - self.west;
        let up = z - self.south;
        if across < 0 || across >= self.width || up < 0 || up >= self.depth {
            return None;
        }
        let byte = self.contents[HEADER + (up * self.width + across) as usize];
        Some(byte as i8 as i32)
    }
}

/// Makes the heights file for the box west, south, width by depth, from
/// `seed`, as the bytes to write.  It's 134 million columns for Omega, so
/// it takes a while: `after_line` is called after each line of columns,
/// south to north, with how many are done, and makes it give up (`None`)
/// by answering false.
pub fn make(seed: u64, west: i32, south: i32, width: i32, depth: i32,
            mut after_line: impl FnMut(i32) -> bool) -> Option<Vec<u8>> {
    let mut bytes = Vec::with_capacity(HEADER + (width * depth) as usize);
    bytes.extend_from_slice(TAG);
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.extend_from_slice(&seed.to_le_bytes());
    bytes.extend_from_slice(&(west as i16).to_le_bytes());
    bytes.extend_from_slice(&(south as i16).to_le_bytes());
    bytes.extend_from_slice(&(width as u16).to_le_bytes());
    bytes.extend_from_slice(&(depth as u16).to_le_bytes());

    for line in 0..depth {
        for across in 0..width {
            bytes.push(noise::omega_height(seed, west + across, south + line) as u8);
        }
        if !after_line(line + 1) {
            return None;
        }
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heights_come_back_as_they_were_made() {
        let bytes = make(5, 100, -40, 64, 48, |_| true).unwrap();
        let heights = Heights::from_contents(Arc::new(bytes)).unwrap();
        assert_eq!(heights.seed(), 5);
        assert_eq!(heights.covers(), (100, -40, 64, 48));
        for (x, z) in [(100, -40), (163, 7), (120, 0)] {
            assert_eq!(heights.at(x, z), Some(noise::omega_height(5, x, z) as i32));
        }
        assert_eq!(heights.at(99, 0), None);
        assert_eq!(heights.at(164, 0), None);
        assert_eq!(heights.at(100, 8), None);
    }

    #[test]
    fn making_can_be_given_up_part_way() {
        assert!(make(5, 0, 0, 8, 8, |line| line < 3).is_none());
    }

    #[test]
    fn a_short_file_is_turned_away() {
        let mut bytes = make(5, 0, 0, 8, 8, |_| true).unwrap();
        bytes.pop();
        assert!(Heights::from_contents(Arc::new(bytes)).is_err());
    }
}
