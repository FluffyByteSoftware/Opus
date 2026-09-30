//! File:       Opus/Conductor/dev/gameworld/src/make.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Making the world, the first START SERVER that finds no `region.map`.
//! A seed is drawn, Omega's hills are worked out from it into its heights
//! file, and last of all `region.map` is written.  It goes last on
//! purpose: if the server is stopped or dies part way, there's no map, and
//! the next START SERVER starts the making over instead of finding half a
//! world.
//!
//! To make a new world, stop the server and delete `Content/world/`.

use conductor_tools::diskman;
use conductor_tools::fingerprinter;
use conductor_tools::scribe::{self, Channel};
use conductor_tools::services::{self, State};

use crate::heights;
use crate::regionmap::{Ground, RegionMap};

/// Makes the world and writes it out, waiting on DiskMan (this is
/// GameWorld's own thread, so the waiting costs nobody).  The map comes
/// back once it's all on disk.
pub fn world() -> Result<RegionMap, String> {
    let mut seed_bytes = [0u8; 8];
    fingerprinter::random_bytes(&mut seed_bytes)
        .map_err(|e| format!("the OS wouldn't give a seed: {e}"))?;
    let map = RegionMap::first(u64::from_le_bytes(seed_bytes));
    scribe::info(Channel::Game, &format!("GameWorld is making a new world, from seed {}.", map.seed));

    for number in 0..map.regions.len() {
        if map.regions[number].ground == Ground::Heights {
            heights_for(&map, number)?;
        }
    }

    let path = crate::region_map_path();
    diskman::write(&path, map.to_bytes()).wait()
        .map_err(|e| format!("{} couldn't be written: {e}", path.display()))?;
    scribe::info(Channel::Game, "GameWorld has made the world: Alpha to the west of 0,0,0, flat, and Omega to \
        the east, in hills.");
    Ok(map)
}

/// Works out region `number`'s heights from the map's seed and writes its
/// heights file.  For a new world, and for one whose heights file has
/// gone missing: the same seed makes the same hills.
pub fn heights_for(map: &RegionMap, number: usize) -> Result<(), String> {
    let name = &map.regions[number].name;
    let (west, south, width, depth) = map.block_box(number)
        .ok_or_else(|| format!("{name} has no chunks on the map"))?;

    let note = |done: i32| format!("Making the world: {name}'s hills, {done} of {depth} lines of columns.");
    services::set(services::GAMEWORLD, State::Starting, &note(0));
    let started = std::time::Instant::now();

    let bytes = heights::make(map.seed, west, south, width, depth, |done| {
        // A few hundred lines at a time is often enough for the page, and
        // checking in every line keeps the Services tab from calling it
        // stuck.
        if done % 256 == 0 {
            services::set(services::GAMEWORLD, State::Starting, &note(done));
        }
        services::seen(services::GAMEWORLD);
        !crate::stopping()
    }).ok_or_else(|| format!("the server was stopped while {name}'s hills were being made"))?;

    let path = crate::heights_path(name);
    let size = bytes.len();
    diskman::write(&path, bytes).wait()
        .map_err(|e| format!("{} couldn't be written: {e}", path.display()))?;
    scribe::debug(Channel::Game, &format!("GameWorld made {} ({} MB) in {:.1} s.", path.display(),
                                          size / (1024 * 1024), started.elapsed().as_secs_f64()));
    Ok(())
}
