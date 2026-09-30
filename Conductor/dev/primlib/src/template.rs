//! File:       Opus/Conductor/dev/primlib/src/template.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Templates and blueprints: the cheat sheets objects are made from.
//!
//! A template (`NPC`) is a list of components with their defaults, so we
//! don't write the same 50 lines in 50 NPCs.  A blueprint (`goblin_a`)
//! starts as a copy of its template's list, then changes what it needs:
//! a value, a component the template doesn't have, or one it does that a
//! goblin_a shouldn't.  The world spawns copies of a blueprint.
//!
//! Both hold at most one component of each kind; setting one that's
//! already there replaces it.  Today they're built in Rust.  They'll be
//! written as Lua scripts later, and the scripts will call these same
//! functions.

use crate::components::{Component, Kind};

/// Puts the component on the list, in place of one of the same kind if
/// there is one, on the end if there isn't.
fn put(list: &mut Vec<Component>, component: Component) {
    let kind = component.kind();
    // Rust note: `iter_mut().find()` walks the list for the first one that
    // matches, and hands back a way to change it in place.
    match list.iter_mut().find(|existing| existing.kind() == kind) {
        Some(existing) => *existing = component,
        None => list.push(component),
    }
}

/// A named list of components and their defaults: what a whole kind of
/// object starts with.
#[derive(Clone, Debug)]
pub struct Template {
    name: String,
    components: Vec<Component>,
}

impl Template {
    /// An empty template.
    pub fn new(name: &str) -> Template {
        Template { name: name.to_string(), components: Vec::new() }
    }

    /// Adds a component with its default, or replaces the one of its kind.
    pub fn add(&mut self, component: Component) {
        put(&mut self.components, component);
    }

    /// Adds a kind at the value `Component::default_of()` gives it.
    pub fn add_default(&mut self, kind: Kind) {
        put(&mut self.components, Component::default_of(kind));
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn components(&self) -> &[Component] {
        &self.components
    }
}

/// What a new copy of one object looks like: its template's components,
/// with its own changes.
#[derive(Clone, Debug)]
pub struct Blueprint {
    name: String,
    template: String,
    components: Vec<Component>,
}

impl Blueprint {
    /// A blueprint that starts as a copy of the template's list.  Changing
    /// the template afterwards doesn't change the blueprint.
    pub fn from_template(name: &str, template: &Template) -> Blueprint {
        Blueprint {
            name: name.to_string(),
            template: template.name.clone(),
            components: template.components.clone(),
        }
    }

    /// Sets a component's value, adding it if the template didn't have it.
    pub fn set(&mut self, component: Component) {
        put(&mut self.components, component);
    }

    /// Takes a component off the list, one the template gave it that this
    /// object shouldn't have.  False if it wasn't there.
    pub fn remove(&mut self, kind: Kind) -> bool {
        let before = self.components.len();
        // Rust note: `retain` keeps the ones the little function says yes
        // to, and drops the rest.
        self.components.retain(|component| component.kind() != kind);
        self.components.len() != before
    }

    /// Whether the blueprint has a component of this kind.
    pub fn has(&self, kind: Kind) -> bool {
        self.components.iter().any(|component| component.kind() == kind)
    }

    /// The blueprint's component of this kind, with its value.
    pub fn get(&self, kind: Kind) -> Option<&Component> {
        self.components.iter().find(|component| component.kind() == kind)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The name of the template it started from.
    pub fn template_name(&self) -> &str {
        &self.template
    }

    pub fn components(&self) -> &[Component] {
        &self.components
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{LongName, Pool, ShortName, Titles, Transform, Vector3};

    #[test]
    fn setting_a_kind_twice_keeps_one() {
        let mut npc = Template::new("NPC");
        npc.add_default(Kind::Health);
        npc.add(Component::Health(Pool::full(100)));
        assert_eq!(npc.components(), &[Component::Health(Pool::full(100))]);
    }

    #[test]
    fn a_blueprint_starts_as_its_template() {
        let mut npc = Template::new("NPC");
        npc.add_default(Kind::Transform);
        npc.add(Component::Health(Pool::full(100)));

        let goblin = Blueprint::from_template("goblin_a", &npc);
        assert_eq!(goblin.name(), "goblin_a");
        assert_eq!(goblin.template_name(), "NPC");
        assert_eq!(goblin.components(), npc.components());
    }

    #[test]
    fn a_blueprint_can_change_add_and_drop() {
        let mut npc = Template::new("NPC");
        npc.add_default(Kind::Transform);
        npc.add_default(Kind::ShortName);
        npc.add_default(Kind::Mana);

        let mut goblin = Blueprint::from_template("goblin_a", &npc);
        goblin.set(Component::ShortName(ShortName::new("goblin")));
        goblin.set(Component::Titles(Titles::one("the plucky")));
        assert!(goblin.remove(Kind::Mana));
        assert!(!goblin.remove(Kind::Mana));

        assert_eq!(goblin.get(Kind::ShortName), Some(&Component::ShortName(ShortName::new("goblin"))));
        assert!(goblin.has(Kind::Titles), "the template didn't have titles; the blueprint added them");
        assert!(!goblin.has(Kind::Mana), "a goblin_a doesn't get the NPC's mana");
        assert!(npc.components().contains(&Component::default_of(Kind::Mana)), "the template keeps its own");
    }

    #[test]
    fn a_small_template_for_something_that_isnt_an_npc() {
        let mut item = Template::new("Item");
        item.add(Component::Transform(Transform::at(Vector3::new(5.0, 0.0, 5.0))));
        item.add(Component::ShortName(ShortName::new("sword")));

        let mut sword = Blueprint::from_template("sword_a", &item);
        sword.set(Component::LongName(LongName::new("a Rusty Sword")));
        let kinds: Vec<Kind> = sword.components().iter().map(|component| component.kind()).collect();
        assert_eq!(kinds, vec![Kind::Transform, Kind::ShortName, Kind::LongName]);
    }
}
