//! File:       Opus/Conductor/dev/gameworld/src/noise.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Omega's hills.  Jacob's ask, 2026-09-30: "random noise with +/- 5 on
//! the Y so it looks bumpy", as smooth rolling hills.  Written by hand, no
//! crate.
//!
//! The idea is "value noise".  Lay a grid over the ground with a point
//! every so many blocks, give each point a random height, and for every
//! column in between blend the four points around it, easing in and out
//! so there are no creases.  Two grids are added together: a wide one (a
//! point every 64 blocks, 64 m) for the hills, and a tight one (every 16,
//! 16 m) for the bumps on them.
//!
//! "Random" here means worked out from the seed and the point's place, so
//! the same seed makes the same hills every time.  That's what lets a lost
//! heights file be made again from the seed in `region.map`.
//!
//! This only runs when the world is made.  After that the hills live in
//! Omega's heights file, so a change here never moves a hill that's
//! already there.

/// The highest a hill goes, and the lowest a dip, in blocks from 0.
pub const MOST: i32 = 5;

/// The wide grid, for the hills: a point every 64 blocks, up to 3.5 up or
/// down.  The tight one, for the bumps on them: every 16, up to 1.5.  3.5
/// and 1.5 add up to the 5.
const HILLS: (i32, f64) = (64, 3.5);
const BUMPS: (i32, f64) = (16, 1.5);

/// How high the dirt is at column x,z of Omega, -5 to +5.
///
/// Where Omega meets Alpha, the ground just changes: "a sharp divide it
/// just suddenly becomes the other biome" (Jacob, 2026-09-30), so there
/// can be a step of up to 5 blocks at 0.  A real build will blend one
/// biome into the next; this one doesn't.
pub fn omega_height(seed: u64, x: i32, z: i32) -> i8 {
    let hills = smooth(seed, x, z, HILLS.0) * HILLS.1;
    // A different seed for the second grid, or its points would sit on
    // top of the first grid's and make the same shape smaller.
    let bumps = smooth(seed ^ 0x9e37_79b9_7f4a_7c15, x, z, BUMPS.0) * BUMPS.1;
    let height = (hills + bumps).round() as i32;
    height.clamp(-MOST, MOST) as i8
}

/// One grid's height at x,z, from -1 to 1, blended from the four points
/// around it.
fn smooth(seed: u64, x: i32, z: i32, spacing: i32) -> f64 {
    let cell_x = x.div_euclid(spacing);
    let cell_z = z.div_euclid(spacing);
    let across_x = ease(x.rem_euclid(spacing) as f64 / spacing as f64);
    let across_z = ease(z.rem_euclid(spacing) as f64 / spacing as f64);

    let south = blend(point(seed, cell_x, cell_z), point(seed, cell_x + 1, cell_z), across_x);
    let north = blend(point(seed, cell_x, cell_z + 1), point(seed, cell_x + 1, cell_z + 1), across_x);
    blend(south, north, across_z)
}

/// Eases 0 to 1 in and out, so a slope flattens off at each grid point
/// instead of meeting the next one at an angle.
fn ease(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

/// `a` when `t` is 0, `b` when it's 1, and the line between.
fn blend(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// A grid point's height, -1 to 1, from the seed and where it is.
fn point(seed: u64, x: i32, z: i32) -> f64 {
    let mixed = scramble(seed ^ (x as u32 as u64) ^ ((z as u32 as u64) << 32));
    // The top 53 bits, as a fraction from 0 to 1, then stretched to -1..1.
    (mixed >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
}

/// Stirs a number up so that numbers next to each other come out nothing
/// alike.  This is SplitMix64's last step, a well-known one.
fn scramble(mut n: u64) -> u64 {
    n = n.wrapping_add(0x9e37_79b9_7f4a_7c15);
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    n ^ (n >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_makes_the_same_hills() {
        for (x, z) in [(40, 7), (1000, -3000), (8000, 8000)] {
            assert_eq!(omega_height(42, x, z), omega_height(42, x, z));
        }
        let differs = (0..200).any(|z| omega_height(1, 500, z * 7) != omega_height(2, 500, z * 7));
        assert!(differs, "two seeds made the same ground");
    }

    #[test]
    fn the_hills_stay_within_5_and_roll_instead_of_jumping() {
        let mut highest = i8::MIN;
        let mut lowest = i8::MAX;
        for z in -300..300 {
            for x in 0..300 {
                let here = omega_height(99, x, z);
                assert!((-5..=5).contains(&here));
                highest = highest.max(here);
                lowest = lowest.min(here);
                let east = omega_height(99, x + 1, z);
                let north = omega_height(99, x, z + 1);
                assert!((here - east).abs() <= 1 && (here - north).abs() <= 1, "a step at {x},{z}");
            }
        }
        assert!(highest >= 2 && lowest <= -2, "it's hardly bumpy: {lowest} to {highest}");
    }
}
