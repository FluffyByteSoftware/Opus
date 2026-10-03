//! File:       Opus/Conductor/dev/gameworld/src/overworld.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! `Content/world/simple_overworld.map`: the world's rough shape, for the
//! client to draw the distance with.  Every 16 by 16 blocks of ground is
//! one patch, and a patch is two numbers: its average height and the
//! block most often on top.  Nothing under the ground is in it, and no
//! single block: "didn't want to give them the entire worlds voxel
//! information so they could find all the secrets" (Jacob, 2026-10-03).
//! The real chunks come from the server near the player, and win.
//!
//! Ensemble will read the same file, so its layout is a contract, the same
//! as a packet: `Documentation/LLM/SIMPLE_OVERWORLD_MAP.md` has it with a
//! worked example, and when this file and it disagree, this file is what
//! gets fixed.  A change to the layout bumps its version.
//!
//! ```text
//! 8 bytes   OPUSOVWM
//! u16       version, 1
//! u64       the seed the world was made from, the same as region.map's
//! i16       the westmost block's x (-8192 at world_size 16)
//! i16       the southmost block's z (-8192)
//! u16       a patch's side, in blocks (16)
//! u16       how many patches east-west (1024)
//! u16       how many patches north-south (1024)
//! every patch, 4 bytes:  the south line first, in a line west to east
//!   i16     its height: the average of its columns' top blocks, rounded
//!           to the nearest, a half away from 0
//!   u16     its top: the kind most of its columns have on top, the lower
//!           number on a tie
//! ```
//!
//! 4,194,332 bytes at world_size 16, and 16,777,244 at 32.  Not squeezed
//! (Jacob: "no squeezing concern").  Every number is little-endian.
//!
//! GameWorld writes it before the first chunk goes out, when it's missing
//! or isn't this world's, so the door stays shut until it's there (Jacob:
//! "should do this before we allow connections").  It's made from the
//! ground's rules, never from somebody's digging, so it's always safe to
//! write over: unlike a chunk file, it's never the only copy of anything.

use std::sync::Arc;

use conductor_tools::diskman;
use conductor_tools::scribe::{self, Channel};
use conductor_tools::services::{self, State};

use crate::block::Block;
use crate::build;
use crate::bytes::Reader;
use crate::chunk::ChunkPos;
use crate::heights::Heights;
use crate::regionmap::RegionMap;

const TAG: &[u8; 8] = b"OPUSOVWM";
const VERSION: u16 = 1;

/// A patch's side, in blocks (Jacob, 2026-10-03: "the world is going to be
/// rather large though so maybe we go with 16x16?").  A chunk is two
/// patches a side, so a patch is never across two regions.
pub const PATCH: i32 = 16;

/// The bytes before the patches start.
const HEADER: usize = 8 + 2 + 8 + 2 + 2 + 2 + 2 + 2;

/// One patch of ground.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Patch {
    /// The average height of its columns' top blocks.
    pub height: i16,
    /// The kind most of its columns have on top.
    pub top: Block,
}

/// The whole file.
#[derive(Debug, Clone, PartialEq)]
pub struct Overworld {
    /// The seed the world was made from, so a map from another world is
    /// known for one.
    pub seed: u64,
    /// The westmost block's x and the southmost block's z.
    pub west: i32,
    pub south: i32,
    /// A patch's side, in blocks.
    pub side: i32,
    /// How many patches east-west and north-south.
    pub width: i32,
    pub depth: i32,
    /// Every patch, the south line first, in a line west to east.
    patches: Vec<Patch>,
}

impl Overworld {
    /// The patch `across` from the west edge and `up` from the south edge,
    /// both counted in patches.  `None` outside the map.
    pub fn patch(&self, across: i32, up: i32) -> Option<Patch> {
        if across < 0 || across >= self.width || up < 0 || up >= self.depth {
            return None;
        }
        self.patches.get((up * self.width + across) as usize).copied()
    }

    /// True if this is the map `make()` would make for `map`: the same seed,
    /// the same ground and patches of the same size.
    pub fn fits(&self, map: &RegionMap) -> bool {
        let (west, south, width, depth) = map.block_bounds();
        self.seed == map.seed && self.west == west && self.south == south && self.side == PATCH
            && self.width * PATCH == width && self.depth * PATCH == depth
    }

    /// The map as its file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER + self.patches.len() * 4);
        bytes.extend_from_slice(TAG);
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        bytes.extend_from_slice(&self.seed.to_le_bytes());
        bytes.extend_from_slice(&(self.west as i16).to_le_bytes());
        bytes.extend_from_slice(&(self.south as i16).to_le_bytes());
        bytes.extend_from_slice(&(self.side as u16).to_le_bytes());
        bytes.extend_from_slice(&(self.width as u16).to_le_bytes());
        bytes.extend_from_slice(&(self.depth as u16).to_le_bytes());
        for patch in &self.patches {
            bytes.extend_from_slice(&patch.height.to_le_bytes());
            bytes.extend_from_slice(&patch.top.0.to_le_bytes());
        }
        bytes
    }

    /// The map read back from its file.
    pub fn from_bytes(bytes: &[u8]) -> Result<Overworld, String> {
        let mut reader = Reader::new(bytes);
        reader.tag_and_version(TAG, VERSION)?;
        let seed = reader.u64("the seed")?;
        let west = reader.i16("the westmost block")? as i32;
        let south = reader.i16("the southmost block")? as i32;
        let side = reader.u16("a patch's side")? as i32;
        let width = reader.u16("how many patches east-west")? as i32;
        let depth = reader.u16("how many patches north-south")? as i32;
        if side == 0 {
            return Err("its patches are 0 blocks a side".to_string());
        }
        // The count came off the disk, so the bytes are taken (and found
        // to be there) before anything is made that size: a corrupt header
        // is turned away, not a try at 17 GB of memory.
        let cells = (width as usize).checked_mul(depth as usize).and_then(|cells| cells.checked_mul(4))
            .ok_or_else(|| "a map too big to be ours".to_string())?;
        let all = reader.take(cells, "the patches")?;
        reader.finish()?;

        let patches = all.chunks_exact(4)
            .map(|four| Patch {
                height: i16::from_le_bytes([four[0], four[1]]),
                top: Block(u16::from_le_bytes([four[2], four[3]])),
            })
            .collect();
        Ok(Overworld { seed, west, south, side, width, depth, patches })
    }
}

/// Works out the simple overworld map for the world `map` and its regions'
/// heights files (by region number, `None` for a region without one).
/// Every column of the world is looked at once, so it takes a while:
/// `after_line` is called after each line of patches, south to north, with
/// how many are done, and makes it give up by answering false.
pub fn make(map: &RegionMap, heights: &[Option<Heights>], mut after_line: impl FnMut(i32) -> bool)
    -> Result<Overworld, String> {
    let (west, south, blocks_wide, blocks_deep) = map.block_bounds();
    if blocks_wide % PATCH != 0 || blocks_deep % PATCH != 0 {
        return Err(format!("the world, {blocks_wide} by {blocks_deep} blocks, isn't whole patches of {PATCH}"));
    }
    let width = blocks_wide / PATCH;
    let depth = blocks_deep / PATCH;
    let mut patches = Vec::with_capacity((width * depth) as usize);

    for up in 0..depth {
        for across in 0..width {
            patches.push(patch_at(map, heights, west + across * PATCH, south + up * PATCH)?);
        }
        if !after_line(up + 1) {
            return Err("it was given up part way".to_string());
        }
    }
    Ok(Overworld { seed: map.seed, west, south, side: PATCH, width, depth, patches })
}

/// The patch whose south-west column is x,z.
fn patch_at(map: &RegionMap, heights: &[Option<Heights>], x: i32, z: i32) -> Result<Patch, String> {
    // Every row of a column is in the same region today (`RegionMap::
    // first()`), so the region of the row the ground starts in, block 0's,
    // says how the column is made.
    let (number, region) = ChunkPos::of_block(x, 0, z)
        .and_then(|pos| map.region_at(pos))
        .ok_or_else(|| format!("the patch at {x},{z} is outside region.map"))?;
    let region_heights = heights.get(number).and_then(|heights| heights.as_ref());

    let mut sum = 0i64;
    let mut kinds: Vec<(Block, u32)> = Vec::new();
    for column_z in z..z + PATCH {
        for column_x in x..x + PATCH {
            let (height, block) = build::top(region, region_heights, column_x, column_z)?;
            sum += height as i64;
            match kinds.iter_mut().find(|(kind, _)| *kind == block) {
                Some((_, count)) => *count += 1,
                None => kinds.push((block, 1)),
            }
        }
    }
    // A sum of whole numbers over 256 is exact in an f64, and `round()`
    // takes a half away from 0, as the contract says.
    let height = (sum as f64 / (PATCH * PATCH) as f64).round() as i16;
    Ok(Patch { height, top: commonest(&kinds) })
}

/// The kind with the most columns, the lower number on a tie, so the same
/// ground always makes the same file.
fn commonest(kinds: &[(Block, u32)]) -> Block {
    let mut best = (Block::AIR, 0u32);
    for &(kind, count) in kinds {
        if count > best.1 || (count == best.1 && kind.0 < best.0.0) {
            best = (kind, count);
        }
    }
    best.0
}

/// Makes sure `simple_overworld.map` is there and is this world's, writing
/// it if it isn't, and waits on DiskMan to say it's on the disk (this is
/// GameWorld's own thread, so the waiting costs nobody).  Hands back the
/// file's bytes, which GameWorld keeps for networking to send players at
/// PLAY.  An error means there's no map to send them, and the door stays
/// shut.
pub fn ensure(map: &RegionMap, heights: &[Option<Heights>]) -> Result<Arc<Vec<u8>>, String> {
    let path = crate::overworld_path();
    match diskman::read(&path).wait() {
        Ok(bytes) => match Overworld::from_bytes(&bytes) {
            Ok(found) if found.fits(map) => {
                scribe::debug(Channel::Game, &format!("GameWorld read {}: it's this world's.", path.display()));
                return Ok(bytes);
            }
            Ok(_) => scribe::info(Channel::Game, &format!("{} is another world's.  GameWorld is making it again for \
                this one.", path.display())),
            Err(why) => scribe::info(Channel::Game, &format!("{} isn't right: {why}.  GameWorld is making it again.",
                                                             path.display())),
        },
        Err(e) if e.is_not_found() => scribe::info(Channel::Game, &format!("GameWorld is making {}: it isn't \
            there yet.", path.display())),
        Err(e) => return Err(format!("{} can't be read: {e}", path.display())),
    }

    let (_, _, _, blocks_deep) = map.block_bounds();
    let lines = blocks_deep / PATCH;
    let note = |done: i32| format!("Making the simple overworld map: {done} of {lines} lines of patches.");
    services::set(services::GAMEWORLD, State::Starting, &note(0));
    let started = std::time::Instant::now();

    let overworld = make(map, heights, |done| {
        // The same pace as the heights file's: the page now and then, and
        // a check-in every line so the Services tab doesn't call it stuck.
        if done % 64 == 0 {
            services::set(services::GAMEWORLD, State::Starting, &note(done));
        }
        services::seen(services::GAMEWORLD);
        !crate::stopping()
    })?;

    // Rust note: DiskMan takes the bytes it writes, so it gets a copy and
    // the bytes themselves come back to the caller.  16 MB at the most,
    // once a world.
    let bytes = overworld.to_bytes();
    let size = bytes.len();
    diskman::write(&path, bytes.clone()).wait()
        .map_err(|e| format!("{} couldn't be written: {e}", path.display()))?;
    scribe::debug(Channel::Game, &format!("GameWorld wrote {} ({size} bytes) in {:.1} s.", path.display(),
                                          started.elapsed().as_secs_f64()));
    Ok(Arc::new(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A heights file for the box west, south, width by depth, with the
    /// height in each column from `height`, made by hand so a test can
    /// say what the ground is.  The layout is `heights.rs`'s.
    fn heights_from(seed: u64, west: i32, south: i32, width: i32, depth: i32,
                    height: impl Fn(i32, i32) -> i8) -> Heights {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"OPUSHGHT");
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&seed.to_le_bytes());
        bytes.extend_from_slice(&(west as i16).to_le_bytes());
        bytes.extend_from_slice(&(south as i16).to_le_bytes());
        bytes.extend_from_slice(&(width as u16).to_le_bytes());
        bytes.extend_from_slice(&(depth as u16).to_le_bytes());
        for z in south..south + depth {
            for x in west..west + width {
                bytes.push(height(x, z) as u8);
            }
        }
        Heights::from_contents(Arc::new(bytes)).unwrap()
    }

    /// A world at `size`, with Omega's heights from `height`.
    fn world(size: i32, height: impl Fn(i32, i32) -> i8) -> (RegionMap, Vec<Option<Heights>>) {
        let map = RegionMap::first(7, size);
        let (west, south, width, depth) = map.block_box(1).unwrap();
        let omega = heights_from(7, west, south, width, depth, height);
        (map, vec![None, Some(omega)])
    }

    /// The worked example in SIMPLE_OVERWORLD_MAP.md, byte for byte: two
    /// patches east-west, one north-south, 16 blocks a side, from block
    /// -16,-8.
    const WORKED_EXAMPLE: [u8; 36] = [
        b'O', b'P', b'U', b'S', b'O', b'V', b'W', b'M',
        0x01, 0x00,                                     // version 1
        0x2A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // seed 42
        0xF0, 0xFF,                                     // west -16
        0xF8, 0xFF,                                     // south -8
        0x10, 0x00,                                     // side 16
        0x02, 0x00,                                     // 2 east-west
        0x01, 0x00,                                     // 1 north-south
        0x00, 0x00, 0x01, 0x00,                         // height 0, DIRT
        0xFD, 0xFF, 0x02, 0x00,                         // height -3, STONE
    ];

    #[test]
    fn the_worked_example_reads_as_the_document_says() {
        let overworld = Overworld::from_bytes(&WORKED_EXAMPLE).unwrap();
        assert_eq!((overworld.seed, overworld.west, overworld.south), (42, -16, -8));
        assert_eq!((overworld.side, overworld.width, overworld.depth), (16, 2, 1));
        assert_eq!(overworld.patch(0, 0), Some(Patch { height: 0, top: Block::DIRT }));
        assert_eq!(overworld.patch(1, 0), Some(Patch { height: -3, top: Block::STONE }));
        assert_eq!(overworld.patch(2, 0), None);
        assert_eq!(overworld.patch(0, 1), None);
        assert_eq!(overworld.to_bytes(), WORKED_EXAMPLE);
    }

    #[test]
    fn alpha_is_flat_dirt_and_omega_is_its_hills_averaged() {
        // Omega's dirt at 1 and 2 by turns east-west, so every patch's
        // average is 1.5, and a half goes away from 0: 2.  South of 0,
        // -1 and -2 by turns, -1.5, so -2.
        let (map, heights) = world(1, |x, z| {
            let (low, high) = if z < 0 { (-1, -2) } else { (1, 2) };
            if x % 2 == 0 { low } else { high }
        });
        let overworld = make(&map, &heights, |_| true).unwrap();
        assert!(overworld.fits(&map));
        assert_eq!((overworld.west, overworld.south, overworld.width, overworld.depth), (-512, -512, 64, 64));

        // Alpha, west of 0, every line.
        for (across, up) in [(0, 0), (31, 63), (12, 40)] {
            assert_eq!(overworld.patch(across, up), Some(Patch { height: 0, top: Block::DIRT }), "{across},{up}");
        }
        // Omega, north and south of 0.
        for (across, up) in [(32, 32), (63, 63), (40, 50)] {
            assert_eq!(overworld.patch(across, up), Some(Patch { height: 2, top: Block::DIRT }), "{across},{up}");
        }
        for (across, up) in [(32, 31), (63, 0)] {
            assert_eq!(overworld.patch(across, up), Some(Patch { height: -2, top: Block::DIRT }), "{across},{up}");
        }
    }

    #[test]
    fn the_map_comes_back_from_its_file_as_it_went_in() {
        let (map, heights) = world(1, |x, z| ((x / 7 + z / 5) % 11 - 5) as i8);
        let overworld = make(&map, &heights, |_| true).unwrap();
        let bytes = overworld.to_bytes();
        assert_eq!(bytes.len(), HEADER + 64 * 64 * 4);
        assert_eq!(Overworld::from_bytes(&bytes).unwrap(), overworld);
    }

    #[test]
    fn the_commonest_top_wins_and_a_tie_goes_to_the_lower_number() {
        assert_eq!(commonest(&[(Block::STONE, 200), (Block::DIRT, 56)]), Block::STONE);
        assert_eq!(commonest(&[(Block::STONE, 128), (Block::DIRT, 128)]), Block::DIRT);
        assert_eq!(commonest(&[(Block::DIRT, 255), (Block::STONE, 1)]), Block::DIRT);
    }

    #[test]
    fn another_worlds_map_doesnt_fit() {
        let (map, heights) = world(1, |_, _| 0);
        let overworld = make(&map, &heights, |_| true).unwrap();
        assert!(overworld.fits(&map));
        assert!(!overworld.fits(&RegionMap::first(8, 1)));
        assert!(!overworld.fits(&RegionMap::first(7, 2)));
    }

    #[test]
    fn making_can_be_given_up_part_way() {
        let (map, heights) = world(1, |_, _| 0);
        assert!(make(&map, &heights, |line| line < 3).is_err());
    }

    #[test]
    fn omega_without_its_heights_is_an_error() {
        let map = RegionMap::first(7, 1);
        assert!(make(&map, &[None, None], |_| true).is_err());
    }

    #[test]
    fn a_short_or_long_or_foreign_file_is_turned_away() {
        assert!(Overworld::from_bytes(&WORKED_EXAMPLE[..WORKED_EXAMPLE.len() - 1]).is_err());
        let mut long = WORKED_EXAMPLE.to_vec();
        long.push(0);
        assert!(Overworld::from_bytes(&long).is_err());
        assert!(Overworld::from_bytes(b"OPUSHGHT").is_err());
    }

    #[test]
    fn a_header_claiming_a_map_too_big_to_be_there_is_turned_away() {
        // The two counts are the last four bytes of the header.  At 65,535
        // each that's 17 GB of patches the file doesn't have: a plain
        // "no", not a try at the memory.
        let mut bytes = WORKED_EXAMPLE.to_vec();
        bytes[HEADER - 4..HEADER - 2].copy_from_slice(&u16::MAX.to_le_bytes());
        bytes[HEADER - 2..HEADER].copy_from_slice(&u16::MAX.to_le_bytes());
        assert!(Overworld::from_bytes(&bytes).is_err());
    }

    /// How long the map takes to make at world_size 16, the size Jacob
    /// runs.  Ignored by `cargo test`; run it by hand, optimized, from
    /// `Conductor/dev`:
    ///
    /// ```text
    /// cargo test --release -p conductor-gameworld overworld_map_timing_at_size_16 -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn overworld_map_timing_at_size_16() {
        let (map, heights) = world(16, |_, _| 0);
        let started = std::time::Instant::now();
        let overworld = make(&map, &heights, |_| true).unwrap();
        let took = started.elapsed();
        let bytes = overworld.to_bytes();
        println!("simple_overworld.map at world_size 16: {} patches, {} bytes, made in {:.2} s.",
                 overworld.width * overworld.depth, bytes.len(), took.as_secs_f64());
    }
}
