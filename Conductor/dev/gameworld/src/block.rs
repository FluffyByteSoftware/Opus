//! File:       Opus/Conductor/dev/gameworld/src/block.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! What a block is made of.  A block is 1 m a side, and for now all it
//! holds is its kind: one number, two bytes, so there's room for 65,536
//! kinds ("I think 65k will be enough", Jacob, 2026-09-30).

/// A block's kind.  The number is what goes in a chunk's file and down to
/// the client, so **a kind's number never changes** once it's out there.
/// A new kind takes the next number.
// Rust note: `Block(u16)` is a struct with one unnamed field, reached as
// `.0`.  It costs nothing over a bare u16, but a u16 can't be handed in
// where a block is wanted by mistake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block(pub u16);

impl Block {
    pub const AIR: Block = Block(0);
    pub const DIRT: Block = Block(1);
    pub const STONE: Block = Block(2);
    pub const WOOD: Block = Block(3);
    /// The block at 0,0,0, so the middle of the world can be seen.
    pub const GOLD: Block = Block(4);
    /// The floor at -31 and -32, the layers nobody can dig through.
    pub const BEDROCK: Block = Block(5);

    /// The kind's name, for the log and the web admin.  A number from a
    /// file that isn't one of ours (a newer Conductor wrote it) is UNKNOWN,
    /// and is kept as it is.
    pub fn name(self) -> &'static str {
        match self {
            Block::AIR => "AIR",
            Block::DIRT => "DIRT",
            Block::STONE => "STONE",
            Block::WOOD => "WOOD",
            Block::GOLD => "GOLD",
            Block::BEDROCK => "BEDROCK",
            _ => "UNKNOWN",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_numbers_never_change() {
        assert_eq!(Block::AIR.0, 0);
        assert_eq!(Block::DIRT.0, 1);
        assert_eq!(Block::STONE.0, 2);
        assert_eq!(Block::WOOD.0, 3);
        assert_eq!(Block::GOLD.0, 4);
        assert_eq!(Block::BEDROCK.0, 5);
        assert_eq!(Block(999).name(), "UNKNOWN");
    }
}
