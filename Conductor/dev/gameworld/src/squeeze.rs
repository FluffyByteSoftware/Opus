//! File:       Opus/Conductor/dev/gameworld/src/squeeze.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! A chunk squeezed for sending to a player (protocol version 12).  A
//! chunk is 64 KB as it is, 32,768 blocks of two bytes, and a player is
//! sent 891 of them, so it goes as runs instead: the kinds of block in
//! it, listed once, then "this many of that one", "this many of the
//! next".  Squeezed by hand, no crate (Jacob, 2026-10-03: "runs only for
//! now").  An all-air chunk comes to 8 bytes, a flat one to 13.
//!
//! ```text
//! u8           how it's squeezed: 1, runs.  The only one today; zipping
//!              could come as another without a new packet.
//! u16          how many kinds of block the chunk has, 1 to 32,768
//! u16 x kinds  the kinds, by their numbers, in the order they first
//!              turn up
//! then runs, until all 32,768 blocks are covered, in the file's order
//! (bottom layer first; in a layer, the south row first; in a row, west
//! to east):
//!   the run's length, 1 to 32,768, less one: one byte for 0 to 127;
//!     for more, the low 7 bits with the top bit set, then the rest in a
//!     second byte
//!   which kind, as its place in the list from 0: a u8 when the list
//!     has 256 kinds or fewer, a u16 when it has more
//! ```
//!
//! PROTOCOL.md has the same, with a worked example.  `unsqueeze()` is the
//! way back, for the tests and as the reader the client copies.

use std::collections::HashMap;

use crate::block::Block;
use crate::bytes::Reader;
use crate::chunk::{BLOCKS, Chunk, ChunkPos};

/// The first byte: the blocks go as runs.
pub const RUNS: u8 = 1;

/// A run's length less one goes in one byte below this: runs of 1 to 128.
const ONE_BYTE_LENGTHS: usize = 128;

/// The most kinds whose places fit in a u8.
const BYTE_PLACES: usize = 256;

/// The chunk, squeezed.
pub fn squeeze(chunk: &Chunk) -> Vec<u8> {
    let blocks = chunk.blocks();

    // The runs first, each as its kind's place in the list and its
    // length, the list growing as new kinds turn up.  A kind is looked up
    // once a run, not once a block.
    let mut kinds: Vec<Block> = Vec::new();
    let mut places: HashMap<u16, u16> = HashMap::new();
    let mut runs: Vec<(u16, usize)> = Vec::new();
    let mut at = 0;
    while at < blocks.len() {
        let kind = blocks[at];
        let mut length = 1;
        while at + length < blocks.len() && blocks[at + length] == kind {
            length += 1;
        }
        let place = *places.entry(kind.0).or_insert_with(|| {
            kinds.push(kind);
            (kinds.len() - 1) as u16
        });
        runs.push((place, length));
        at += length;
    }
    let wide = kinds.len() > BYTE_PLACES;

    let mut bytes = Vec::with_capacity(3 + kinds.len() * 2 + runs.len() * 3);
    bytes.push(RUNS);
    bytes.extend_from_slice(&(kinds.len() as u16).to_le_bytes());
    for kind in &kinds {
        bytes.extend_from_slice(&kind.0.to_le_bytes());
    }
    for (place, length) in runs {
        put_length(&mut bytes, length);
        if wide {
            bytes.extend_from_slice(&place.to_le_bytes());
        } else {
            bytes.push(place as u8);
        }
    }
    bytes
}

/// A squeezed chunk back as the chunk at `pos`.  An `Err` says what's
/// wrong with it: a way of squeezing this Conductor doesn't know, a
/// place past the end of the list, runs that come to more or fewer than
/// 32,768 blocks, or bytes left over.
pub fn unsqueeze(bytes: &[u8], pos: ChunkPos) -> Result<Chunk, String> {
    let mut reader = Reader::new(bytes);
    let how = reader.u8("how it's squeezed")?;
    if how != RUNS {
        return Err(format!("it's squeezed as {how}, and this Conductor knows only {RUNS}, runs"));
    }
    let count = reader.u16("how many kinds")? as usize;
    if count == 0 || count > BLOCKS {
        return Err(format!("it lists {count} kinds of block, and a chunk has 1 to {BLOCKS}"));
    }
    let mut kinds = Vec::with_capacity(count);
    for _ in 0..count {
        kinds.push(Block(reader.u16("its kinds")?));
    }
    let wide = count > BYTE_PLACES;

    let mut blocks = Vec::with_capacity(BLOCKS);
    while blocks.len() < BLOCKS {
        let length = take_length(&mut reader)?;
        let place = if wide { reader.u16("a run's kind")? as usize } else { reader.u8("a run's kind")? as usize };
        let Some(&kind) = kinds.get(place) else {
            return Err(format!("a run is of kind {place} in a list of {count}"));
        };
        if blocks.len() + length > BLOCKS {
            return Err(format!("its runs come to more than {BLOCKS} blocks"));
        }
        blocks.resize(blocks.len() + length, kind);
    }
    reader.finish()?;
    Ok(Chunk::from_blocks(pos, blocks))
}

/// A run's length, less one, in one byte or two.
fn put_length(bytes: &mut Vec<u8>, length: usize) {
    let less_one = length - 1;
    if less_one < ONE_BYTE_LENGTHS {
        bytes.push(less_one as u8);
    } else {
        bytes.push(0x80 | (less_one & 0x7F) as u8);
        bytes.push((less_one >> 7) as u8);
    }
}

/// The other way round: a run's length, 1 to 32,768.
fn take_length(reader: &mut Reader) -> Result<usize, String> {
    let first = reader.u8("a run's length")? as usize;
    if first < ONE_BYTE_LENGTHS {
        return Ok(first + 1);
    }
    let second = reader.u8("a run's length")? as usize;
    Ok(((second << 7) | (first & 0x7F)) + 1)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::build;
    use crate::chunk::{ROWS, SIDE};
    use crate::heights::{self, Heights};
    use crate::regionmap::{Ground, Region};

    fn alpha() -> Region {
        Region { name: "Alpha".to_string(), ground: Ground::Flat }
    }

    fn omega() -> Region {
        Region { name: "Omega".to_string(), ground: Ground::Heights }
    }

    /// Omega's hills over the box west, south, width by depth, from a seed.
    fn hills(seed: u64, west: i32, south: i32, width: i32, depth: i32) -> Heights {
        let bytes = heights::make(seed, west, south, width, depth, |_| true).unwrap();
        Heights::from_contents(Arc::new(bytes)).unwrap()
    }

    fn same_blocks(a: &Chunk, b: &Chunk) -> bool {
        a.pos() == b.pos() && a.blocks() == b.blocks()
    }

    #[test]
    fn an_all_air_chunk_is_eight_bytes() {
        let pos = ChunkPos { x: 3, z: -4, row: 6 };
        let chunk = Chunk::filled(pos, Block::AIR);
        let bytes = squeeze(&chunk);
        // Runs, one kind, AIR, then 32,768 less one is 0x7FFF: FF FF, and
        // the kind's place, 0.
        assert_eq!(bytes, vec![1, 1, 0, 0, 0, 0xFF, 0xFF, 0]);
        assert!(same_blocks(&unsqueeze(&bytes, pos).unwrap(), &chunk));
    }

    #[test]
    fn alphas_flat_ground_is_a_handful_of_bytes() {
        // Row 0: two layers of BEDROCK, 2,048 blocks, then 30 of STONE.
        let pos = ChunkPos { x: -3, z: 5, row: 0 };
        let chunk = build::untouched(pos, &alpha(), None).unwrap();
        let bytes = squeeze(&chunk);
        assert_eq!(bytes, vec![1, 2, 0, 5, 0, 2, 0, 0xFF, 0x0F, 0, 0xFF, 0xEF, 1]);
        assert!(same_blocks(&unsqueeze(&bytes, pos).unwrap(), &chunk));

        // Row 1: one layer of DIRT at 0, then AIR.
        let pos = ChunkPos { x: -3, z: 5, row: 1 };
        let chunk = build::untouched(pos, &alpha(), None).unwrap();
        assert_eq!(squeeze(&chunk).len(), 13);
    }

    #[test]
    fn omegas_hills_come_back_as_they_went() {
        let heights = hills(11, 64, 64, 32, 32);
        for row in 0..ROWS {
            let pos = ChunkPos { x: 2, z: 2, row };
            let chunk = build::untouched(pos, &omega(), Some(&heights)).unwrap();
            let bytes = squeeze(&chunk);
            assert!(bytes.len() < BLOCKS * 2, "row {row} came to {} bytes", bytes.len());
            assert!(same_blocks(&unsqueeze(&bytes, pos).unwrap(), &chunk), "row {row}");
        }
    }

    #[test]
    fn the_gold_at_0_0_0_comes_through() {
        let heights = hills(3, 0, 0, 32, 32);
        let pos = ChunkPos { x: 0, z: 0, row: 1 };
        let chunk = build::untouched(pos, &omega(), Some(&heights)).unwrap();
        let back = unsqueeze(&squeeze(&chunk), pos).unwrap();
        assert_eq!(back.block(0, 0, 0), Block::GOLD);
    }

    #[test]
    fn a_chunk_of_every_block_different_takes_two_byte_places() {
        // 32,768 kinds, each run one block long: the worst there is.
        let pos = ChunkPos { x: 0, z: 0, row: 4 };
        let mut chunk = Chunk::filled(pos, Block::AIR);
        for y in 0..SIDE {
            for z in 0..SIDE {
                for x in 0..SIDE {
                    chunk.set(x, y, z, Block(((y * SIDE + z) * SIDE + x) as u16));
                }
            }
        }
        let bytes = squeeze(&chunk);
        assert_eq!(bytes.len(), 1 + 2 + BLOCKS * 2 + BLOCKS * 3);
        assert!(same_blocks(&unsqueeze(&bytes, pos).unwrap(), &chunk));
    }

    #[test]
    fn runs_of_every_length_come_back() {
        // A run of each length across the one-byte and two-byte edge.
        for length in [1, 2, 127, 128, 129, 255, 256, 1024, 32_767] {
            let pos = ChunkPos { x: 1, z: 1, row: 1 };
            let mut chunk = Chunk::filled(pos, Block::STONE);
            let mut set = 0;
            'filling: for y in 0..SIDE {
                for z in 0..SIDE {
                    for x in 0..SIDE {
                        if set == length {
                            break 'filling;
                        }
                        chunk.set(x, y, z, Block::DIRT);
                        set += 1;
                    }
                }
            }
            let bytes = squeeze(&chunk);
            assert!(same_blocks(&unsqueeze(&bytes, pos).unwrap(), &chunk), "a run of {length}");
        }
    }

    #[test]
    fn a_bad_squeeze_is_turned_away() {
        let pos = ChunkPos { x: 0, z: 0, row: 3 };
        let good = squeeze(&Chunk::filled(pos, Block::AIR));
        // Another way of squeezing.
        let mut zipped = good.clone();
        zipped[0] = 2;
        assert!(unsqueeze(&zipped, pos).is_err());
        // No kinds.
        assert!(unsqueeze(&[1, 0, 0], pos).is_err());
        // A kind past the end of the list.
        let mut past = good.clone();
        past[7] = 1;
        assert!(unsqueeze(&past, pos).is_err());
        // Short, long, and one run too many.
        assert!(unsqueeze(&good[..good.len() - 1], pos).is_err());
        let mut long = good.clone();
        long.push(0);
        assert!(unsqueeze(&long, pos).is_err());
        assert!(unsqueeze(&[1, 1, 0, 0, 0, 0xFF, 0xFF, 0, 0, 0], pos).is_err());
        // Runs that overshoot: two runs of 16,385 come to 32,770.
        assert!(unsqueeze(&[1, 1, 0, 0, 0, 0x80, 0x80, 0, 0x80, 0x80, 0], pos).is_err());
    }

    /// How big Omega's chunks come out squeezed, and how long squeezing
    /// takes, over the 9 by 9 columns a player is sent at `view_chunks` 4,
    /// every row.  Ignored by `cargo test`; run it by hand, optimized,
    /// from `Conductor/dev`:
    ///
    /// ```text
    /// cargo test --release -p conductor-gameworld squeezed_view_sizes -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn squeezed_view_sizes() {
        let reach = 4;
        let heights = hills(7, 0, -(reach + 1) * SIDE, (2 * reach + 1) * SIDE, (2 * reach + 2) * SIDE);
        for (name, region, from) in [("Alpha", alpha(), None), ("Omega", omega(), Some(&heights))] {
            let mut chunks = Vec::new();
            for z in -reach..=reach {
                for x in 0..=2 * reach {
                    let x = if from.is_some() { x } else { -1 - x };
                    for row in 0..ROWS {
                        chunks.push(build::untouched(ChunkPos { x, z, row }, &region, from).unwrap());
                    }
                }
            }
            let started = std::time::Instant::now();
            let squeezed: Vec<Vec<u8>> = chunks.iter().map(squeeze).collect();
            let took = started.elapsed();
            let total: usize = squeezed.iter().map(Vec::len).sum();
            let biggest = squeezed.iter().map(Vec::len).max().unwrap_or(0);
            let pieces: usize = squeezed.iter().map(|bytes| bytes.len().div_ceil(1192)).sum();
            println!("{name}, {} chunks: {} bytes squeezed ({} KB), the biggest {} bytes, {} pieces of 1,192, \
                      squeezed in {:.1} ms ({:.0} us a chunk).  As they are: {} MB.",
                     chunks.len(), total, total / 1024, biggest, pieces, took.as_secs_f64() * 1000.0,
                     took.as_secs_f64() * 1e6 / chunks.len() as f64, chunks.len() * BLOCKS * 2 / 1_048_576);
        }
    }
}
