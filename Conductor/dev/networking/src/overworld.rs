//! File:       Opus/Conductor/dev/networking/src/overworld.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The simple overworld map as players are sent it at PLAY (protocol
//! version 11).  GameWorld makes sure of the file and keeps its bytes;
//! this takes them when the door opens, works out the SHA-256 the client
//! checks its copy against, and cuts the map into its pieces, each one
//! built once as the OverworldMapPiece packet it goes out as.  So a
//! player's request for 64 pieces costs 64 sends and nothing else: no
//! copying, no building, no disk.  That's about as much memory again as
//! the map (4 MB at `world_size` 16, 16 MB at 32), spent to save the CPU
//! on every PLAY, the trade Jacob leans to.
//!
//! Every player gets the whole map at every PLAY, over whatever their
//! client had (Jacob, 2026-10-03: "we're just gonna write over whatever
//! the client already has every time").  The layout of the file itself is
//! SIMPLE_OVERWORLD_MAP.md's; the packets are PROTOCOL.md's.

use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};

use conductor_tools::scribe::{self, Channel};

use crate::protocol::{self, MAP_PIECE_BYTES};

/// The map, ready to send.
pub struct Map {
    /// The whole file's size, in bytes.
    size: u32,
    /// Its SHA-256, 64 lowercase hex.
    hash: String,
    /// Every piece as its OverworldMapPiece packet, in order.
    pieces: Vec<Vec<u8>>,
}

// Rust note: `None` means the door isn't open.  An `Arc` so the UDP
// thread and Protogame can each hold the map while they send from it,
// without the lock held, and without a copy.
static MAP: Mutex<Option<Arc<Map>>> = Mutex::new(None);

/// Takes the map from GameWorld and readies it, as the door opens.  An
/// `Err` says why there's nothing to send players, and the door stays
/// shut: nobody gets into the world without it.
pub fn load() -> Result<(), String> {
    let Some(bytes) = conductor_gameworld::overworld_map() else {
        return Err("GameWorld has no simple overworld map to send players.".to_string());
    };
    let map = Map::of(&bytes)?;
    scribe::debug(Channel::Network, &format!("The simple overworld map is ready to send: {} bytes in {} pieces, \
        SHA-256 {}.", map.size, map.pieces.len(), map.hash));
    *lock() = Some(Arc::new(map));
    Ok(())
}

/// Lets the map go, as the door closes.
pub fn unload() {
    lock().take();
}

/// The map, while the door is open.
pub fn current() -> Option<Arc<Map>> {
    lock().clone()
}

fn lock() -> std::sync::MutexGuard<'static, Option<Arc<Map>>> {
    MAP.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Map {
    /// The map from the file's bytes.  An `Err` for one too big for the
    /// offer's u32, or empty, neither of which GameWorld makes.
    fn of(bytes: &[u8]) -> Result<Map, String> {
        if bytes.is_empty() {
            return Err("the simple overworld map is empty".to_string());
        }
        let size = u32::try_from(bytes.len())
            .map_err(|_| format!("the simple overworld map is {} bytes, too big to offer", bytes.len()))?;
        // Rust note: `chunks()` walks the bytes MAP_PIECE_BYTES at a time,
        // the last one whatever is left.
        let pieces = bytes.chunks(MAP_PIECE_BYTES).enumerate()
            .map(|(index, piece)| protocol::overworld_map_piece(index as u32, piece))
            .collect();
        Ok(Map { size, hash: sha256_hex(bytes), pieces })
    }

    /// Its SHA-256, 64 lowercase hex: what PlayerReady has to say back.
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// The answer to a UserPressPlay numbered `ask`.
    pub fn offer(&self, ask: u32) -> Vec<u8> {
        protocol::overworld_map_offer(ask, self.size, self.pieces.len() as u32, &self.hash)
    }

    /// The pieces numbered `first` and the `count - 1` after it, as their
    /// packets, leaving out any past the end of the map.
    pub fn pieces(&self, first: u32, count: u8) -> &[Vec<u8>] {
        let start = (first as usize).min(self.pieces.len());
        let end = start.saturating_add(count as usize).min(self.pieces.len());
        &self.pieces[start..end]
    }
}

/// A SHA-256 as 64 lowercase hex, the way PROTOCOL.md writes it.
fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_is_the_standard_one() {
        // FIPS 180-2's own example, "abc".
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn a_map_cuts_into_pieces_that_put_it_back_together() {
        // Two whole pieces and 28 bytes, like the real map's end.
        let bytes: Vec<u8> = (0..MAP_PIECE_BYTES * 2 + 28).map(|n| n as u8).collect();
        let map = Map::of(&bytes).unwrap();
        assert_eq!(map.size, bytes.len() as u32);
        assert_eq!(map.pieces.len(), 3);

        let mut joined = Vec::new();
        for (index, packet) in map.pieces(0, 64).iter().enumerate() {
            assert_eq!(packet[0], protocol::PacketType::OverworldMapPiece as u8);
            assert_eq!(&packet[1..5], &(index as u32).to_le_bytes());
            joined.extend_from_slice(&packet[5..]);
        }
        assert_eq!(joined, bytes);
        assert_eq!(map.pieces[2].len(), 5 + 28);

        // The offer says what the map is.
        let offer = map.offer(9);
        assert_eq!(&offer[..15], &[0x40, 9, 0, 0, 0, 0x1C, 0x08, 0, 0, 0x00, 0x04, 3, 0, 0, 0]);
        assert!(offer.ends_with(map.hash().as_bytes()));
    }

    #[test]
    fn pieces_past_the_end_are_left_out() {
        let map = Map::of(&[1; MAP_PIECE_BYTES * 3]).unwrap();
        assert_eq!(map.pieces(1, 64).len(), 2);
        assert_eq!(map.pieces(2, 1).len(), 1);
        assert!(map.pieces(3, 1).is_empty());
        assert!(map.pieces(u32::MAX, 64).is_empty());
        assert!(Map::of(&[]).is_err());
    }
}
