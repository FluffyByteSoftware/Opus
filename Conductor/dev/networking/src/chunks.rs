//! File:       Opus/Conductor/dev/networking/src/chunks.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The chunks around a player (protocol version 12).  The client pulls
//! them (Jacob, 2026-10-03: "Pull"): it works out which chunks it hasn't
//! got around where its character stands and asks for them, up to 64 at
//! a time, and asks again for whatever doesn't come.  The server keeps no
//! list of who has what.  Each chunk asked for is checked here and sent
//! squeezed, straight out of GameWorld's cache (`squeezed()`), or refused
//! with why.
//!
//! **This is the one place that says what chunks a player may be sent**
//! (`may_see()`).  Today that's every chunk within `view_chunks` of the
//! column their character stands in, every row, so 891 at 4.  Hidden
//! things are sent ahead and hidden by the client for now (Jacob: "I am
//! not too worried about cheating"), and holding something back one day
//! starts here ("a flexibility to add more security").

use conductor_gameworld::{ChunkPos, Sendable};

use crate::protocol::{self, ChunkRefusal};

/// The packets that answer a request for the chunks at `places` from a
/// player whose character stands in `standing`, the column x and z in
/// chunks: each chunk's pieces, or its ChunkRefused.
pub fn answer(standing: (i32, i32), places: &[ChunkPos]) -> Vec<Vec<u8>> {
    let reach = conductor_gameworld::view_chunks();
    let mut packets = Vec::new();
    for &pos in places {
        if !may_see(standing, pos, reach) {
            packets.push(protocol::chunk_refused(pos, ChunkRefusal::OutOfView));
            continue;
        }
        match conductor_gameworld::squeezed(pos) {
            Sendable::Ready(bytes) => {
                let pieces = protocol::chunk_pieces(pos, &bytes);
                // No chunk squeezes too big to send; one that somehow did
                // is refused rather than sent short.
                if pieces.is_empty() {
                    packets.push(protocol::chunk_refused(pos, ChunkRefusal::Unavailable));
                }
                packets.extend(pieces);
            }
            Sendable::Coming => packets.push(protocol::chunk_refused(pos, ChunkRefusal::NotYet)),
            // Past the edge of the world is out of anybody's view.
            Sendable::Missing if !conductor_gameworld::in_world(pos) => {
                packets.push(protocol::chunk_refused(pos, ChunkRefusal::OutOfView));
            }
            Sendable::Missing => packets.push(protocol::chunk_refused(pos, ChunkRefusal::Unavailable)),
        }
    }
    packets
}

/// Whether a player whose character stands in column `standing` may be
/// sent the chunk at `pos`: within `reach` chunks of it east, west, north
/// and south, any row.  Whether the chunk is in the world is GameWorld's
/// to say.
fn may_see(standing: (i32, i32), pos: ChunkPos, reach: i32) -> bool {
    (pos.x - standing.0).abs() <= reach && (pos.z - standing.1).abs() <= reach
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_player_sees_the_square_around_their_column_every_row() {
        let standing = (0, -1);
        let seen = |x, z, row| may_see(standing, ChunkPos { x, z, row }, 4);
        assert!(seen(0, -1, 0));
        assert!(seen(0, -1, 10));
        assert!(seen(4, 3, 1));
        assert!(seen(-4, -5, 1));
        assert!(!seen(5, -1, 1));
        assert!(!seen(0, 4, 1));
        assert!(!seen(-5, -6, 1));

        let count = (-10..=10).flat_map(|z| (-10..=10).map(move |x| (x, z)))
            .filter(|&(x, z)| may_see(standing, ChunkPos { x, z, row: 0 }, 4))
            .count();
        assert_eq!(count, 81);
    }
}
