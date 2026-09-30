//! File:       Opus/Conductor/dev/primlib/src/world.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The world: every entity, and a store for each kind of component.  It's
//! where objects are spawned from a blueprint, have components added and
//! taken away, and are despawned.  Every call checks the entity is alive
//! first, so an old handle gets nothing rather than somebody else's goblin.
//! Each one also keeps the names of the templates it was made from, so
//! the game can ask whether it's living.
//!
//! There are no locks.  The GameClock's thread owns the world and
//! everything else asks it.

use crate::components::{
    Animator, Component, Kind, LongName, Model, PlayerCharacter, Pool, PrimitiveShape, ShortName, Titles, Transform,
};
use crate::entity::{Entities, Entity};
use crate::store::Store;
use crate::template::Blueprint;

pub struct World {
    entities: Entities,
    transform: Store<Transform>,
    model: Store<Model>,
    primitive_shape: Store<PrimitiveShape>,
    animator: Store<Animator>,
    short_name: Store<ShortName>,
    long_name: Store<LongName>,
    titles: Store<Titles>,
    health: Store<Pool>,
    endurance: Store<Pool>,
    mana: Store<Pool>,
    player_character: Store<PlayerCharacter>,
    /// The templates each entity was made from, its own template's first.
    /// Not a component: nothing adds or takes one away after the spawn.
    templates: Store<Vec<String>>,
}

impl World {
    /// An empty world.
    pub fn new() -> World {
        World {
            entities: Entities::new(),
            transform: Store::new(),
            model: Store::new(),
            primitive_shape: Store::new(),
            animator: Store::new(),
            short_name: Store::new(),
            long_name: Store::new(),
            titles: Store::new(),
            health: Store::new(),
            endurance: Store::new(),
            mana: Store::new(),
            player_character: Store::new(),
            templates: Store::new(),
        }
    }

    /// A new entity with no components at all.
    pub fn spawn_empty(&mut self) -> Entity {
        self.entities.create()
    }

    /// A copy of the blueprint: a new entity with every component on the
    /// blueprint's list, each with its own value, and the blueprint's list
    /// of templates.  A hundred calls make a hundred goblins, and hurting
    /// one doesn't touch the rest.
    pub fn spawn(&mut self, blueprint: &Blueprint) -> Entity {
        let entity = self.entities.create();
        for component in blueprint.components() {
            self.add(entity, component.clone());
        }
        self.templates.insert(entity.index(), blueprint.templates().to_vec());
        entity
    }

    /// Takes the entity out of the world, components and all.  False if it
    /// was already gone.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.entities.is_alive(entity) {
            return false;
        }
        // Every store's slot is emptied, so whatever takes the slot next
        // starts with nothing.
        for kind in Kind::ALL {
            self.remove(entity, kind);
        }
        self.templates.remove(entity.index());
        self.entities.destroy(entity)
    }

    /// The templates the entity was made from, its own template's first.
    /// Empty for one made with `spawn_empty()`, or one that's gone.
    pub fn templates(&self, entity: Entity) -> &[String] {
        match self.slot(entity).and_then(|slot| self.templates.get(slot)) {
            Some(templates) => templates.as_slice(),
            None => &[],
        }
    }

    /// Whether the entity was made from the template named, or from one
    /// that took it in: `is(goblin, "Living")`.
    pub fn is(&self, entity: Entity, template: &str) -> bool {
        self.templates(entity).iter().any(|name| name == template)
    }

    /// Whether the entity is still in the world.
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.entities.is_alive(entity)
    }

    /// How many entities are in the world.
    pub fn count(&self) -> usize {
        self.entities.count()
    }

    /// Every entity in the world.  A list, not a live view, so a system can
    /// go through it and change the world as it goes.
    pub fn all(&self) -> Vec<Entity> {
        self.entities.all()
    }

    /// The entity's slot, if it's alive.  Every call below starts here.
    fn slot(&self, entity: Entity) -> Option<usize> {
        if self.entities.is_alive(entity) { Some(entity.index()) } else { None }
    }

    /// Gives the entity a component, replacing one of the same kind if it
    /// already has one.  False if the entity is gone.
    pub fn add(&mut self, entity: Entity, component: Component) -> bool {
        let Some(slot) = self.slot(entity) else {
            return false;
        };
        match component {
            Component::Transform(value) => self.transform.insert(slot, value),
            Component::Model(value) => self.model.insert(slot, value),
            Component::PrimitiveShape(value) => self.primitive_shape.insert(slot, value),
            Component::Animator(value) => self.animator.insert(slot, value),
            Component::ShortName(value) => self.short_name.insert(slot, value),
            Component::LongName(value) => self.long_name.insert(slot, value),
            Component::Titles(value) => self.titles.insert(slot, value),
            Component::Health(value) => self.health.insert(slot, value),
            Component::Endurance(value) => self.endurance.insert(slot, value),
            Component::Mana(value) => self.mana.insert(slot, value),
            Component::PlayerCharacter(value) => self.player_character.insert(slot, value),
        }
        true
    }

    /// Takes a component off the entity.  False if it didn't have one (or
    /// is gone).
    pub fn remove(&mut self, entity: Entity, kind: Kind) -> bool {
        let Some(slot) = self.slot(entity) else {
            return false;
        };
        match kind {
            Kind::Transform => self.transform.remove(slot).is_some(),
            Kind::Model => self.model.remove(slot).is_some(),
            Kind::PrimitiveShape => self.primitive_shape.remove(slot).is_some(),
            Kind::Animator => self.animator.remove(slot).is_some(),
            Kind::ShortName => self.short_name.remove(slot).is_some(),
            Kind::LongName => self.long_name.remove(slot).is_some(),
            Kind::Titles => self.titles.remove(slot).is_some(),
            Kind::Health => self.health.remove(slot).is_some(),
            Kind::Endurance => self.endurance.remove(slot).is_some(),
            Kind::Mana => self.mana.remove(slot).is_some(),
            Kind::PlayerCharacter => self.player_character.remove(slot).is_some(),
        }
    }

    /// Whether the entity has a component of this kind.
    pub fn has(&self, entity: Entity, kind: Kind) -> bool {
        let Some(slot) = self.slot(entity) else {
            return false;
        };
        match kind {
            Kind::Transform => self.transform.has(slot),
            Kind::Model => self.model.has(slot),
            Kind::PrimitiveShape => self.primitive_shape.has(slot),
            Kind::Animator => self.animator.has(slot),
            Kind::ShortName => self.short_name.has(slot),
            Kind::LongName => self.long_name.has(slot),
            Kind::Titles => self.titles.has(slot),
            Kind::Health => self.health.has(slot),
            Kind::Endurance => self.endurance.has(slot),
            Kind::Mana => self.mana.has(slot),
            Kind::PlayerCharacter => self.player_character.has(slot),
        }
    }

    /// Every kind of component the entity has, in `Kind::ALL`'s order.
    pub fn kinds(&self, entity: Entity) -> Vec<Kind> {
        Kind::ALL.into_iter().filter(|kind| self.has(entity, *kind)).collect()
    }

    /// A copy of one of the entity's components, with its value.  For
    /// whatever needs it whole (the log, and one day the client and the
    /// database); a system reads and changes one through the getters below.
    pub fn component(&self, entity: Entity, kind: Kind) -> Option<Component> {
        // Rust note: `?` hands back `None` from the whole function right
        // there if the entity is gone.  `.copied()` and `.cloned()` turn
        // a borrowed value into a copy of its own.
        let slot = self.slot(entity)?;
        match kind {
            Kind::Transform => self.transform.get(slot).copied().map(Component::Transform),
            Kind::Model => self.model.get(slot).cloned().map(Component::Model),
            Kind::PrimitiveShape => self.primitive_shape.get(slot).copied().map(Component::PrimitiveShape),
            Kind::Animator => self.animator.get(slot).cloned().map(Component::Animator),
            Kind::ShortName => self.short_name.get(slot).cloned().map(Component::ShortName),
            Kind::LongName => self.long_name.get(slot).cloned().map(Component::LongName),
            Kind::Titles => self.titles.get(slot).cloned().map(Component::Titles),
            Kind::Health => self.health.get(slot).copied().map(Component::Health),
            Kind::Endurance => self.endurance.get(slot).copied().map(Component::Endurance),
            Kind::Mana => self.mana.get(slot).copied().map(Component::Mana),
            Kind::PlayerCharacter => self.player_character.get(slot).copied().map(Component::PlayerCharacter),
        }
    }

    // The getters: one to read and one to change, for each kind.  Written
    // out rather than made by a macro, so each one reads plainly.

    pub fn transform(&self, entity: Entity) -> Option<&Transform> {
        self.transform.get(self.slot(entity)?)
    }

    pub fn transform_mut(&mut self, entity: Entity) -> Option<&mut Transform> {
        let slot = self.slot(entity)?;
        self.transform.get_mut(slot)
    }

    pub fn model(&self, entity: Entity) -> Option<&Model> {
        self.model.get(self.slot(entity)?)
    }

    pub fn model_mut(&mut self, entity: Entity) -> Option<&mut Model> {
        let slot = self.slot(entity)?;
        self.model.get_mut(slot)
    }

    pub fn primitive_shape(&self, entity: Entity) -> Option<&PrimitiveShape> {
        self.primitive_shape.get(self.slot(entity)?)
    }

    pub fn primitive_shape_mut(&mut self, entity: Entity) -> Option<&mut PrimitiveShape> {
        let slot = self.slot(entity)?;
        self.primitive_shape.get_mut(slot)
    }

    pub fn animator(&self, entity: Entity) -> Option<&Animator> {
        self.animator.get(self.slot(entity)?)
    }

    pub fn animator_mut(&mut self, entity: Entity) -> Option<&mut Animator> {
        let slot = self.slot(entity)?;
        self.animator.get_mut(slot)
    }

    pub fn short_name(&self, entity: Entity) -> Option<&ShortName> {
        self.short_name.get(self.slot(entity)?)
    }

    pub fn short_name_mut(&mut self, entity: Entity) -> Option<&mut ShortName> {
        let slot = self.slot(entity)?;
        self.short_name.get_mut(slot)
    }

    pub fn long_name(&self, entity: Entity) -> Option<&LongName> {
        self.long_name.get(self.slot(entity)?)
    }

    pub fn long_name_mut(&mut self, entity: Entity) -> Option<&mut LongName> {
        let slot = self.slot(entity)?;
        self.long_name.get_mut(slot)
    }

    pub fn titles(&self, entity: Entity) -> Option<&Titles> {
        self.titles.get(self.slot(entity)?)
    }

    pub fn titles_mut(&mut self, entity: Entity) -> Option<&mut Titles> {
        let slot = self.slot(entity)?;
        self.titles.get_mut(slot)
    }

    pub fn health(&self, entity: Entity) -> Option<&Pool> {
        self.health.get(self.slot(entity)?)
    }

    pub fn health_mut(&mut self, entity: Entity) -> Option<&mut Pool> {
        let slot = self.slot(entity)?;
        self.health.get_mut(slot)
    }

    pub fn endurance(&self, entity: Entity) -> Option<&Pool> {
        self.endurance.get(self.slot(entity)?)
    }

    pub fn endurance_mut(&mut self, entity: Entity) -> Option<&mut Pool> {
        let slot = self.slot(entity)?;
        self.endurance.get_mut(slot)
    }

    pub fn mana(&self, entity: Entity) -> Option<&Pool> {
        self.mana.get(self.slot(entity)?)
    }

    pub fn mana_mut(&mut self, entity: Entity) -> Option<&mut Pool> {
        let slot = self.slot(entity)?;
        self.mana.get_mut(slot)
    }

    pub fn player_character(&self, entity: Entity) -> Option<&PlayerCharacter> {
        self.player_character.get(self.slot(entity)?)
    }

    pub fn player_character_mut(&mut self, entity: Entity) -> Option<&mut PlayerCharacter> {
        let slot = self.slot(entity)?;
        self.player_character.get_mut(slot)
    }
}

impl Default for World {
    fn default() -> World {
        World::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Vector3;
    use crate::template::Template;

    /// My sample NPC template: every kind there is, at its default.
    fn npc() -> Template {
        let mut npc = Template::new("NPC");
        for kind in Kind::ALL {
            npc.add_default(kind);
        }
        npc
    }

    /// My sample goblin, from the NPC template.
    fn goblin_a() -> Blueprint {
        let mut goblin = Blueprint::from_template("goblin_a", &npc());
        let mut transform = Transform::default();
        transform.rotation = Vector3::new(0.0, 0.0, -90.0);
        goblin.set(Component::Transform(transform));
        goblin.set(Component::Model(Model::new("Goblins/GoblinArcher")));
        goblin.set(Component::PrimitiveShape(PrimitiveShape::Capsule));
        goblin.set(Component::Animator(Animator::playing("idle", true)));
        goblin.set(Component::ShortName(ShortName::new("goblin")));
        goblin.set(Component::LongName(LongName::new("goblin archer")));
        goblin.set(Component::Titles(Titles::one("the plucky")));
        goblin.set(Component::Health(Pool::full(2000)));
        goblin.set(Component::Endurance(Pool::full(200)));
        goblin.set(Component::Mana(Pool::full(2)));
        goblin
    }

    #[test]
    fn a_spawned_goblin_has_what_its_blueprint_says() {
        let mut world = World::new();
        let goblin = world.spawn(&goblin_a());

        assert_eq!(world.kinds(goblin), Kind::ALL.to_vec());
        let transform = world.transform(goblin).copied().unwrap_or_default();
        assert_eq!(transform.position, Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(transform.rotation, Vector3::new(0.0, 0.0, -90.0));
        assert_eq!(transform.scale, Vector3::new(1.0, 1.0, 1.0));
        assert_eq!(world.model(goblin).map(|model| model.path.as_str()), Some("Goblins/GoblinArcher"));
        assert_eq!(world.primitive_shape(goblin), Some(&PrimitiveShape::Capsule));
        assert_eq!(world.animator(goblin), Some(&Animator::playing("idle", true)));
        assert_eq!(world.short_name(goblin).map(|name| name.text.as_str()), Some("goblin"));
        assert_eq!(world.titles(goblin).and_then(|titles| titles.current()), Some("the plucky"));
        assert_eq!(world.health(goblin), Some(&Pool { current: 2000, max: 2000 }));
        assert_eq!(world.mana(goblin), Some(&Pool { current: 2, max: 2 }));
    }

    #[test]
    fn a_hundred_goblins_each_with_their_own_health() {
        let mut world = World::new();
        let blueprint = goblin_a();
        let mut goblins = Vec::new();
        for _ in 0..100 {
            goblins.push(world.spawn(&blueprint));
        }
        assert_eq!(world.count(), 100);

        if let Some(health) = world.health_mut(goblins[7]) {
            health.damage(500);
        }
        assert_eq!(world.health(goblins[7]).map(|health| health.current), Some(1500));
        for (number, goblin) in goblins.iter().enumerate() {
            if number != 7 {
                assert_eq!(world.health(*goblin).map(|health| health.current), Some(2000));
            }
        }
    }

    #[test]
    fn components_come_and_go() {
        let mut world = World::new();
        let thing = world.spawn_empty();
        assert!(world.kinds(thing).is_empty());

        assert!(world.add(thing, Component::Transform(Transform::at(Vector3::new(1.0, 2.0, 3.0)))));
        assert!(world.has(thing, Kind::Transform));
        if let Some(transform) = world.transform_mut(thing) {
            transform.position.y += 10.0;
        }
        let moved = Transform::at(Vector3::new(1.0, 12.0, 3.0));
        assert_eq!(world.component(thing, Kind::Transform), Some(Component::Transform(moved)));

        assert!(world.remove(thing, Kind::Transform));
        assert!(!world.remove(thing, Kind::Transform), "it's already gone");
        assert_eq!(world.transform(thing), None);
    }

    #[test]
    fn a_despawned_goblin_leaves_nothing_behind() {
        let mut world = World::new();
        let blueprint = goblin_a();
        let old = world.spawn(&blueprint);
        assert!(world.despawn(old));
        assert!(!world.despawn(old));
        assert_eq!(world.count(), 0);

        // The next entity takes the old slot.  It starts empty, and the old
        // handle can't reach it.
        let new = world.spawn_empty();
        assert_eq!(new.index(), old.index());
        assert!(world.kinds(new).is_empty());
        assert!(!world.add(old, Component::Health(Pool::full(1))));
        assert_eq!(world.health(old), None);
        assert_eq!(world.health(new), None);
    }

    #[test]
    fn one_goblin_changes_track_and_the_rest_keep_idling() {
        let mut world = World::new();
        let blueprint = goblin_a();
        let walker = world.spawn(&blueprint);
        let idler = world.spawn(&blueprint);

        if let Some(animator) = world.animator_mut(walker) {
            animator.current_track = "walk".to_string();
        }
        assert_eq!(world.animator(walker).map(|animator| animator.current_track.as_str()), Some("walk"));
        assert_eq!(world.animator(idler).map(|animator| animator.current_track.as_str()), Some("idle"));
    }

    #[test]
    fn a_copy_remembers_its_templates_and_forgets_them_when_despawned() {
        let mut world = World::new();
        let mut living = Template::new("Living");
        living.add(Component::Health(Pool::full(10)));
        let mut npc = Template::new("NPC");
        npc.take_in(&living);
        let goblin = world.spawn(&Blueprint::from_template("goblin_a", &npc));

        assert!(world.is(goblin, "NPC") && world.is(goblin, "Living"));
        assert!(!world.is(goblin, "Character"));
        assert_eq!(world.templates(goblin), &["NPC".to_string(), "Living".to_string()]);

        assert!(world.despawn(goblin));
        let rock = world.spawn_empty();
        assert_eq!(rock.index(), goblin.index());
        assert!(world.templates(rock).is_empty(), "the slot's next entity doesn't inherit the goblin's");
        assert!(!world.is(goblin, "Living"), "the old handle gets nothing");
    }

    #[test]
    fn a_player_character_says_whose_it_is() {
        let mut world = World::new();
        let thing = world.spawn_empty();
        assert!(world.add(thing, Component::PlayerCharacter(PlayerCharacter::new(7, 42))));
        assert_eq!(world.player_character(thing).map(|player| player.account_id), Some(7));
        assert_eq!(world.player_character(thing).map(|player| player.character_id), Some(42));
    }

    #[test]
    fn changing_a_blueprint_later_leaves_the_copies_alone() {
        let mut world = World::new();
        let mut blueprint = goblin_a();
        let goblin = world.spawn(&blueprint);
        blueprint.set(Component::Health(Pool::full(1)));
        assert_eq!(world.health(goblin).map(|health| health.max), Some(2000));
    }
}
