//! File:       Opus/Conductor/dev/primlib/src/store.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! A `Store` holds one kind of component for every entity that has it,
//! kept at the entity's slot number.  An entity without that component
//! has an empty slot.  The store doesn't know about generations; the world
//! checks the entity is alive before it asks, and empties every store's
//! slot when the entity is despawned, so a reused slot always starts
//! empty.

/// One kind of component, a slot per entity.
// Rust note: `T` is the component this store holds, so the world has a
// `Store<Position>`, a `Store<Pool>` and so on, all from this one piece
// of code.  `Option<T>` is either `Some(value)` or `None`: here, `None`
// is an entity that doesn't have the component.
pub struct Store<T> {
    slots: Vec<Option<T>>,
}

impl<T> Store<T> {
    pub fn new() -> Store<T> {
        Store { slots: Vec::new() }
    }

    /// Puts the component in the slot, replacing whatever was there.  The
    /// list grows to reach the slot if it has to.
    pub fn insert(&mut self, index: usize, value: T) {
        if index >= self.slots.len() {
            // Rust note: `resize_with` grows the list, filling the new
            // slots with what the little function hands back, `None` here.
            self.slots.resize_with(index + 1, || None);
        }
        self.slots[index] = Some(value);
    }

    /// Empties the slot, and hands back what was in it.
    pub fn remove(&mut self, index: usize) -> Option<T> {
        match self.slots.get_mut(index) {
            // Rust note: `take()` moves the value out and leaves `None`.
            Some(slot) => slot.take(),
            None => None,
        }
    }

    /// Whether the slot holds a component.
    pub fn has(&self, index: usize) -> bool {
        matches!(self.slots.get(index), Some(Some(_)))
    }

    /// The component in the slot, to read.
    pub fn get(&self, index: usize) -> Option<&T> {
        match self.slots.get(index) {
            Some(slot) => slot.as_ref(),
            None => None,
        }
    }

    /// The component in the slot, to change.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        match self.slots.get_mut(index) {
            Some(slot) => slot.as_mut(),
            None => None,
        }
    }
}

impl<T> Default for Store<T> {
    fn default() -> Store<T> {
        Store::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_get_and_remove() {
        let mut store: Store<u32> = Store::new();
        store.insert(3, 42);
        assert!(store.has(3));
        assert!(!store.has(2), "the slots it grew past stay empty");
        assert_eq!(store.get(3), Some(&42));

        if let Some(value) = store.get_mut(3) {
            *value = 7;
        }
        assert_eq!(store.remove(3), Some(7));
        assert!(!store.has(3));
        assert_eq!(store.remove(3), None);
    }

    #[test]
    fn a_slot_past_the_end_is_empty() {
        let store: Store<u32> = Store::new();
        assert!(!store.has(100));
        assert_eq!(store.get(100), None);
    }
}
