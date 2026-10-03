//! File:       Opus/Conductor/dev/gameworld/src/regionmap.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! `Content/world/region.map`: which region every chunk is in.  A region
//! is a zone is a biome (Jacob, 2026-09-30: "a biome name / zone or
//! collection of chunks"), and a chunk is in one, never across two.  A
//! chunk nobody has changed has no file of its own, so this is the only
//! place that says what it is.
//!
//! It's binary, since it's "a summary that tells the chunks how to
//! assemble themselves for the server _AND_ the client" (Jacob): Ensemble
//! will read the same file, so its layout is a contract, the same as a
//! packet.  `Documentation/LLM/REGION_MAP.md` is that contract, with a
//! worked example; when this file and it disagree, this file is what gets
//! fixed.  A change to the layout bumps its version, and both change
//! together.
//!
//! ```text
//! 8 bytes   OPUSRMAP
//! u16       version, 2
//! u64       the seed the world was made from
//! i16       the westmost chunk's x (-256 at world_size 16)
//! i16       the southmost chunk's z (-256)
//! u16       how many chunks east-west (512)
//! u16       how many chunks north-south (512)
//! u8        how many rows up and down (11)
//! u8        how many regions, then for each:
//!             u8   its ground: 0 flat, 1 heights
//!             u8   its name's length, then the name, UTF-8
//! u8 x every chunk   the number of the region it's in, counted from 0 in
//!                    the list above: the bottom row first; in a row,
//!                    the south line first; in a line, west to east.
//! ```
//!
//! Row 0's bottom is block -32, and every row is 32 blocks.  At world_size
//! 16 that's 2,883,584 chunks, a byte each, about 2.75 MB; at 32, four
//! times that.  Every number is little-endian.

use crate::bytes::Reader;
use crate::chunk::{ChunkPos, ROWS, SIDE};

const TAG: &[u8; 8] = b"OPUSRMAP";
/// Version 2 (2026-10-01): the same layout, with blocks 1 m and eleven rows
/// from -32 where version 1 had 50 cm blocks and two rows from -16.
const VERSION: u16 = 2;

/// How an untouched chunk in a region is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ground {
    /// Dirt at 0, stone under it down to -30, BEDROCK at -31 and -32, air
    /// over it.  Alpha.
    Flat,
    /// The same layers, with the dirt at the height the region's heights
    /// file says for each column.  Omega.
    Heights,
}

/// One region: its name and how its ground is made.
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    pub name: String,
    pub ground: Ground,
}

/// The whole map.
#[derive(Debug, Clone, PartialEq)]
pub struct RegionMap {
    /// The seed the world was made from.  Kept so a lost heights file can
    /// be made again exactly as it was.
    pub seed: u64,
    pub regions: Vec<Region>,
    /// The westmost chunk's x and the southmost chunk's z.
    west: i32,
    south: i32,
    /// How many chunks east-west and north-south.
    width: i32,
    depth: i32,
    /// A region number for every chunk, in the order the file has them.
    grid: Vec<u8>,
}

/// How many chunks across one `world_size` is: 1024 blocks (Jacob,
/// 2026-10-01: "world_size = 1 = 1024 blocks").  Half of it either side
/// of 0, so any whole size comes out in whole chunks with 0,0,0 in the
/// middle.
pub const CHUNKS_PER_SIZE: i32 = 1024 / SIDE;

impl RegionMap {
    /// The world's first map, made from `seed`, `size` times 1024 blocks a
    /// side (`world_size` in `game.cfg`): Alpha everything west of 0, flat,
    /// and Omega everything east of it, in rolling hills, every row the
    /// same.  At 16, the default, that's chunks -256 to 255 each way.
    pub fn first(seed: u64, size: i32) -> RegionMap {
        let regions = vec![
            Region { name: "Alpha".to_string(), ground: Ground::Flat },
            Region { name: "Omega".to_string(), ground: Ground::Heights },
        ];
        let across = size * CHUNKS_PER_SIZE;
        let west = -across / 2;
        let south = -across / 2;
        let mut map = RegionMap {
            seed,
            regions,
            west,
            south,
            width: across,
            depth: across,
            grid: vec![0; (across * across * ROWS as i32) as usize],
        };
        for row in 0..ROWS {
            for z in south..south + across {
                for x in west..west + across {
                    let region = if x < 0 { 0 } else { 1 };
                    if let Some(slot) = map.slot(ChunkPos { x, z, row }) {
                        map.grid[slot] = region;
                    }
                }
            }
        }
        map
    }

    /// True if this map is the shape `first()` makes at `size`: as wide as
    /// it is deep, the right number of chunks, 0,0,0 in the middle.
    pub fn is_size(&self, size: i32) -> bool {
        let across = size * CHUNKS_PER_SIZE;
        self.width == across && self.depth == across && self.west == -across / 2 && self.south == -across / 2
    }

    /// How many blocks the map runs east-west, for telling a person.
    pub fn blocks_across(&self) -> i32 {
        self.width * SIDE
    }

    /// The region a chunk is in, and its number in the list.  `None` for a
    /// chunk outside the world.
    pub fn region_at(&self, pos: ChunkPos) -> Option<(usize, &Region)> {
        let number = *self.grid.get(self.slot(pos)?)? as usize;
        self.regions.get(number).map(|region| (number, region))
    }

    /// The blocks a region covers, as the box around it: its westmost x,
    /// southmost z, and how many blocks east-west and north-south.  For
    /// sizing its heights file.  `None` for a region with no chunks.
    pub fn block_box(&self, number: usize) -> Option<(i32, i32, i32, i32)> {
        let mut found: Option<(i32, i32, i32, i32)> = None;
        for z in self.south..self.south + self.depth {
            for x in self.west..self.west + self.width {
                let here = (0..ROWS).any(|row| {
                    self.slot(ChunkPos { x, z, row }).is_some_and(|slot| self.grid[slot] as usize == number)
                });
                if here {
                    found = Some(match found {
                        None => (x, z, x, z),
                        Some((west, south, east, north)) => (west.min(x), south.min(z), east.max(x), north.max(z)),
                    });
                }
            }
        }
        found.map(|(west, south, east, north)| {
            (west * SIDE, south * SIDE, (east - west + 1) * SIDE, (north - south + 1) * SIDE)
        })
    }

    /// Where a chunk's region number sits in the grid.
    fn slot(&self, pos: ChunkPos) -> Option<usize> {
        let x = pos.x - self.west;
        let z = pos.z - self.south;
        if x < 0 || x >= self.width || z < 0 || z >= self.depth || pos.row >= ROWS {
            return None;
        }
        Some(((pos.row as i32 * self.depth + z) * self.width + x) as usize)
    }

    /// The map as its file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(64 + self.grid.len());
        bytes.extend_from_slice(TAG);
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        bytes.extend_from_slice(&self.seed.to_le_bytes());
        bytes.extend_from_slice(&(self.west as i16).to_le_bytes());
        bytes.extend_from_slice(&(self.south as i16).to_le_bytes());
        bytes.extend_from_slice(&(self.width as u16).to_le_bytes());
        bytes.extend_from_slice(&(self.depth as u16).to_le_bytes());
        bytes.push(ROWS);
        bytes.push(self.regions.len() as u8);
        for region in &self.regions {
            bytes.push(match region.ground {
                Ground::Flat => 0,
                Ground::Heights => 1,
            });
            bytes.push(region.name.len() as u8);
            bytes.extend_from_slice(region.name.as_bytes());
        }
        bytes.extend_from_slice(&self.grid);
        bytes
    }

    /// The map read back from its file.
    pub fn from_bytes(bytes: &[u8]) -> Result<RegionMap, String> {
        let mut reader = Reader::new(bytes);
        reader.tag_and_version(TAG, VERSION)?;
        let seed = reader.u64("the seed")?;
        let west = reader.i16("the westmost chunk")? as i32;
        let south = reader.i16("the southmost chunk")? as i32;
        let width = reader.u16("the width")? as i32;
        let depth = reader.u16("the depth")? as i32;
        let rows = reader.u8("the rows")?;
        if rows != ROWS {
            return Err(format!("it has {rows} rows of chunks, and the world has {ROWS}"));
        }

        let count = reader.u8("the number of regions")?;
        let mut regions = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let ground = match reader.u8("a region's ground")? {
                0 => Ground::Flat,
                1 => Ground::Heights,
                other => return Err(format!("a region's ground is {other}, which isn't one we know")),
            };
            let length = reader.u8("a region's name")? as usize;
            let name = std::str::from_utf8(reader.take(length, "a region's name")?)
                .map_err(|_| "a region's name isn't text".to_string())?
                .to_string();
            if name.is_empty() {
                return Err("a region has no name".to_string());
            }
            regions.push(Region { name, ground });
        }

        // The three numbers came off the disk, so the multiply is checked:
        // a corrupt header (or a map at the top of the u16 range) would
        // overflow an i32 and, in a debug build, take GameWorld's thread
        // down with it.
        let cells = (width as usize).checked_mul(depth as usize)
            .and_then(|cells| cells.checked_mul(rows as usize))
            .ok_or_else(|| "a grid too big to be ours".to_string())?;
        let grid = reader.take(cells, "the chunks")?.to_vec();
        reader.finish()?;
        if let Some(bad) = grid.iter().find(|&&number| number as usize >= regions.len()) {
            return Err(format!("a chunk is in region {bad}, and there are only {} regions", regions.len()));
        }
        Ok(RegionMap { seed, regions, west, south, width, depth, grid })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_is_west_of_0_and_omega_east() {
        let map = RegionMap::first(7, 16);
        let name = |x, z, row| map.region_at(ChunkPos { x, z, row }).map(|(_, region)| region.name.clone());
        assert_eq!(name(-1, 0, 0).as_deref(), Some("Alpha"));
        assert_eq!(name(-256, -256, 10).as_deref(), Some("Alpha"));
        assert_eq!(name(0, 0, 0).as_deref(), Some("Omega"));
        assert_eq!(name(255, 255, 10).as_deref(), Some("Omega"));
        assert_eq!(name(256, 0, 0), None);
        assert_eq!(name(-257, 0, 0), None);
        assert_eq!(name(0, 0, 11), None);
    }

    #[test]
    fn omegas_box_is_the_east_half() {
        let map = RegionMap::first(7, 16);
        assert_eq!(map.block_box(1), Some((0, -8192, 8192, 16384)));
        assert_eq!(map.block_box(0), Some((-8192, -8192, 8192, 16384)));
        assert_eq!(map.block_box(2), None);
    }

    #[test]
    fn the_size_is_1024_blocks_a_step_with_0_in_the_middle() {
        let small = RegionMap::first(7, 2);
        assert_eq!(small.blocks_across(), 2048);
        assert_eq!(small.block_box(0), Some((-1024, -1024, 1024, 2048)));
        assert!(small.region_at(ChunkPos { x: -32, z: 31, row: 0 }).is_some());
        assert!(small.region_at(ChunkPos { x: 32, z: 0, row: 0 }).is_none());
        let odd = RegionMap::first(7, 3);
        assert_eq!(odd.block_box(1), Some((0, -1536, 1536, 3072)));
        assert!(small.is_size(2));
        assert!(!small.is_size(3));
        assert!(odd.is_size(3));
        assert_eq!(RegionMap::first(7, 32).blocks_across(), 32768);
    }

    #[test]
    fn the_map_comes_back_from_its_file_as_it_went_in() {
        let map = RegionMap::first(0x1234_5678_9abc_def0, 16);
        let bytes = map.to_bytes();
        assert_eq!(RegionMap::from_bytes(&bytes), Ok(map));
    }

    #[test]
    fn a_chunk_in_a_region_that_isnt_there_is_turned_away() {
        let mut bytes = RegionMap::first(7, 16).to_bytes();
        let last = bytes.len() - 1;
        bytes[last] = 9;
        assert!(RegionMap::from_bytes(&bytes).is_err());
        assert!(RegionMap::from_bytes(&bytes[..100]).is_err());
    }

    #[test]
    fn a_header_claiming_a_grid_too_big_to_count_is_turned_away() {
        // The width and the depth sit after the tag, the version, the seed,
        // the west and the south: 8 + 2 + 8 + 2 + 2.  At 65,535 each, times
        // eleven rows, the count overflows an i32; it has to be a plain
        // "no", not a panic.
        let mut bytes = RegionMap::first(7, 16).to_bytes();
        bytes[22..24].copy_from_slice(&u16::MAX.to_le_bytes());
        bytes[24..26].copy_from_slice(&u16::MAX.to_le_bytes());
        assert!(RegionMap::from_bytes(&bytes).is_err());
    }
}
