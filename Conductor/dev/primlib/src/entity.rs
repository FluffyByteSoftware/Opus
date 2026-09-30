//! File:       Opus/Conductor/dev/primlib/src/entity.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! An entity, and the list that hands them out and takes them back.  An
//! entity is a slot number and a generation.  When an entity is despawned
//! its slot goes back on the free list to be reused, and the slot's
//! generation goes up by one.  So a handle to a dead goblin (slot 5,
//! generation 0) never reads the goblin that took slot 5 after it
//! (generation 1): every look-up checks both.

/// One object in the world.  On its own it's only a name; the components
/// the world holds for it are what it is.
// Rust note: `Copy` means an `Entity` is copied when it's passed around,
// like a number, instead of moved.  It's two small numbers, so that's
// cheaper than borrowing it.  `PartialEq` and `Eq` let two be compared
// with `==`, and `Debug` lets a test print one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl Entity {
    /// The slot this entity sits in.  Every `Store` keeps its component for
    /// the entity at this position.
    pub fn index(&self) -> usize {
        self.index as usize
    }

    /// How many times the slot had been used before this entity got it.
    pub fn generation(&self) -> u32 {
        self.generation
    }
}

/// Every slot there's ever been, which are in use, and which are free.
pub struct Entities {
    /// The current generation of each slot.
    generations: Vec<u32>,
    /// Whether each slot has a live entity in it.
    alive: Vec<bool>,
    /// The slots that are free to reuse.  The most recently freed is
    /// reused first.
    free: Vec<u32>,
    /// How many are alive, kept so `count()` doesn't have to walk the list.
    living: usize,
}

impl Entities {
    pub fn new() -> Entities {
        Entities { generations: Vec::new(), alive: Vec::new(), free: Vec::new(), living: 0 }
    }

    /// A new entity: a freed slot if there is one, a new slot on the end
    /// if there isn't.
    pub fn create(&mut self) -> Entity {
        self.living += 1;
        match self.free.pop() {
            Some(index) => {
                self.alive[index as usize] = true;
                Entity { index, generation: self.generations[index as usize] }
            }
            None => {
                let index = self.generations.len() as u32;
                self.generations.push(0);
                self.alive.push(true);
                Entity { index, generation: 0 }
            }
        }
    }

    /// Takes the entity out.  Hands back false if it was already gone (or
    /// is an old handle to a slot that's been reused), and does nothing.
    pub fn destroy(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        let index = entity.index();
        self.alive[index] = false;
        // Rust note: `wrapping_add` goes back to 0 after the biggest u32
        // instead of stopping the program.  A slot would have to be reused
        // four billion times for that to happen.
        self.generations[index] = self.generations[index].wrapping_add(1);
        self.free.push(entity.index);
        self.living -= 1;
        true
    }

    /// Whether this is a live entity: its slot is in use, and by it, not by
    /// something that took the slot after it.
    pub fn is_alive(&self, entity: Entity) -> bool {
        let index = entity.index();
        index < self.alive.len() && self.alive[index] && self.generations[index] == entity.generation
    }

    /// How many entities are alive.
    pub fn count(&self) -> usize {
        self.living
    }

    /// Every live entity, in slot order.
    pub fn all(&self) -> Vec<Entity> {
        let mut list = Vec::with_capacity(self.living);
        for index in 0..self.alive.len() {
            if self.alive[index] {
                list.push(Entity { index: index as u32, generation: self.generations[index] });
            }
        }
        list
    }
}

// Rust note: `Default` is what `Entities::default()` makes.  Clippy asks
// for it on anything with a `new()` that takes nothing.
impl Default for Entities {
    fn default() -> Entities {
        Entities::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_entities_get_new_slots() {
        let mut entities = Entities::new();
        let a = entities.create();
        let b = entities.create();
        assert_eq!(a.index(), 0);
        assert_eq!(b.index(), 1);
        assert_eq!(entities.count(), 2);
    }

    #[test]
    fn a_reused_slot_gets_a_new_generation() {
        let mut entities = Entities::new();
        let old = entities.create();
        assert!(entities.destroy(old));
        let new = entities.create();

        assert_eq!(new.index(), old.index());
        assert_eq!(new.generation(), old.generation() + 1);
        assert!(!entities.is_alive(old), "the old handle must not read the new entity");
        assert!(entities.is_alive(new));
    }

    #[test]
    fn destroying_twice_does_nothing_the_second_time() {
        let mut entities = Entities::new();
        let e = entities.create();
        assert!(entities.destroy(e));
        assert!(!entities.destroy(e));
        assert_eq!(entities.count(), 0);
    }

    #[test]
    fn all_lists_only_the_living() {
        let mut entities = Entities::new();
        let a = entities.create();
        let b = entities.create();
        let c = entities.create();
        entities.destroy(b);
        assert_eq!(entities.all(), vec![a, c]);
    }
}
