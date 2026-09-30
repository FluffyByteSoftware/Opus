//! File:       Opus/Conductor/dev/primlib/src/lib.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! primlib, the game library.  An ECS (entity, component, system) written
//! by hand: an entity is only a number, the components hung on it are what
//! make it a goblin or a sword, and the systems (none yet) are what act on
//! every entity with a given set of components each tick.
//!
//! A template (`NPC`) is a cheat sheet of components and their defaults,
//! so we don't write the same 50 lines in 50 NPCs.  A blueprint
//! (`goblin_a`) starts from a template and changes what it needs to.  The
//! world spawns copies of a blueprint, each with its own values.

pub mod components;
pub mod entity;
pub mod store;
pub mod template;
pub mod world;

// Rust note: these `pub use` lines let the rest of Conductor write
// `conductor_primlib::World` instead of `conductor_primlib::world::World`.
pub use components::{
    Animator, Component, Kind, LongName, Model, Pool, PrimitiveShape, ShortName, Titles, Transform, Vector3,
};
pub use entity::Entity;
pub use template::{Blueprint, Template};
pub use world::World;
