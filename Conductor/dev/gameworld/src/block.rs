//! File:       Opus/Conductor/dev/gameworld/src/block.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! What a voxel holds.  A voxel is 1 m a side, and holds two things: its
//! kind, one number of two bytes, so there's room for 65,536 kinds ("I
//! think 65k will be enough", Jacob, 2026-09-30), and its density, how
//! solid it is, one byte (2026-10-03).
//!
//! The kinds come in two sorts (7 Days to Die's split, Jacob, 2026-10-03):
//! **terrain**, the ground, drawn smooth where its density crosses
//! halfway, and **structure**, the things built, drawn as whole cubes.
//! Both live in the same chunks.

/// A voxel's kind.  The number is what goes in a chunk's file and down to
/// the client, so **a kind's number never changes** once it's out there,
/// and a number that's been dropped is never handed out again.  A new
/// kind takes the next number.
// Rust note: `Block(u16)` is a struct with one unnamed field, reached as
// `.0`.  It costs nothing over a bare u16, but a u16 can't be handed in
// where a block is wanted by mistake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block(pub u16);

impl Block {
    pub const AIR: Block = Block(0);
    pub const DIRT: Block = Block(1);
    pub const STONE: Block = Block(2);
    /// Planks, to look at.
    pub const WOOD: Block = Block(3);
    // 4 was GOLD, the block that marked 0,0,0, dropped on 2026-10-03.
    // Never used again.
    /// The floor at -31 and -32, the layers nobody can dig through.
    pub const BEDROCK: Block = Block(5);
    /// Grey bricks, what a wall is built of (2026-10-03).
    pub const MASONED_STONE: Block = Block(6);

    /// The kind's name, for the log and the web admin.  A number from a
    /// file that isn't one of ours (a newer Conductor wrote it) is UNKNOWN,
    /// and is kept as it is.
    pub fn name(self) -> &'static str {
        match self {
            Block::AIR => "AIR",
            Block::DIRT => "DIRT",
            Block::STONE => "STONE",
            Block::WOOD => "WOOD",
            Block::BEDROCK => "BEDROCK",
            Block::MASONED_STONE => "MASONED_STONE",
            _ => "UNKNOWN",
        }
    }

    /// Whether the kind is terrain, drawn smooth.  The rest are structure,
    /// drawn as cubes, and AIR is neither ("AIR: NOTHING").  A kind we
    /// don't know is drawn as a cube.
    pub fn is_terrain(self) -> bool {
        matches!(self, Block::DIRT | Block::STONE | Block::BEDROCK)
    }

    /// The density the kind has when nothing has shaped it: empty for AIR,
    /// full for everything else.  Ground built from a region's rule, and
    /// anything read from a file older than densities, gets these.
    pub fn plain_density(self) -> Density {
        if self == Block::AIR {
            Density::EMPTY
        } else {
            Density::FULL
        }
    }
}

/// How solid a voxel is: 0 empty to 255 full, with the surface where it
/// crosses halfway, so 128 and over is solid (Jacob, 2026-10-03).  Every
/// voxel has one, AIR included: the air beside the ground is what says
/// how far out the ground's surface sits.  A structure voxel is always
/// full, and nothing reads its density.
///
/// **A voxel's kind is AIR exactly when its density is under halfway**
/// (`fits()`), so whatever asks a kind whether it's solid gets the same
/// answer the density would give.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Density(pub u8);

impl Density {
    pub const EMPTY: Density = Density(0);
    /// The surface: this and over is solid.
    pub const HALF: Density = Density(128);
    pub const FULL: Density = Density(255);

    pub fn is_solid(self) -> bool {
        self.0 >= Density::HALF.0
    }

    /// Whether a voxel of kind `block` may have this density: AIR under
    /// halfway, anything else halfway or over, and a structure kind only
    /// full.
    pub fn fits(self, block: Block) -> bool {
        if block == Block::AIR {
            !self.is_solid()
        } else if block.is_terrain() {
            self.is_solid()
        } else {
            self == Density::FULL
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
        assert_eq!(Block::BEDROCK.0, 5);
        assert_eq!(Block::MASONED_STONE.0, 6);
        // GOLD's, never handed out again.
        assert_eq!(Block(4).name(), "UNKNOWN");
        assert_eq!(Block(999).name(), "UNKNOWN");
    }

    #[test]
    fn dirt_stone_and_bedrock_are_terrain_and_the_rest_arent() {
        assert!(Block::DIRT.is_terrain());
        assert!(Block::STONE.is_terrain());
        assert!(Block::BEDROCK.is_terrain());
        assert!(!Block::WOOD.is_terrain());
        assert!(!Block::MASONED_STONE.is_terrain());
        assert!(!Block::AIR.is_terrain());
        assert!(!Block(999).is_terrain());
    }

    #[test]
    fn halfway_and_over_is_solid() {
        assert!(!Density::EMPTY.is_solid());
        assert!(!Density(127).is_solid());
        assert!(Density::HALF.is_solid());
        assert!(Density::FULL.is_solid());
    }

    #[test]
    fn a_kind_and_its_density_agree_on_solid() {
        assert!(Density(127).fits(Block::AIR));
        assert!(!Density::HALF.fits(Block::AIR));
        assert!(Density::HALF.fits(Block::DIRT));
        assert!(!Density(127).fits(Block::STONE));
        assert!(Density::FULL.fits(Block::WOOD));
        assert!(!Density(200).fits(Block::MASONED_STONE));
        for kind in [Block::AIR, Block::DIRT, Block::STONE, Block::WOOD, Block::BEDROCK, Block::MASONED_STONE] {
            assert!(kind.plain_density().fits(kind), "{}", kind.name());
        }
    }
}
