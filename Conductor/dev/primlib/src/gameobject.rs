//! File:       Opus/Conductor/dev/primlib/src/gameobject.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The templates the game itself is built on, and making a player's
//! character from its save.  They're written in Rust for now; they move to
//! Lua with the rest of the templates later.
//!
//! `Living` is a small template other templates take in whole, the way
//! `inherit STD_LIVING;` did in the Discworld mudlib: the names, health,
//! endurance and mana.  Something can have health without being living (a
//! door you can break), but everything living has all of them, and a name
//! is a requirement.
//!
//! `Character` is a player's character: Living, a place in the world, a
//! capsule to draw until there are models, and the `PlayerCharacter` that
//! says whose it is.  A new one starts with 10 health, 10 endurance and 10
//! mana.

use crate::components::{Component, Kind, PlayerCharacter, Pool, PrimitiveShape};
use crate::save::Save;
use crate::template::{Blueprint, Template};

pub const LIVING: &str = "Living";
pub const CHARACTER: &str = "Character";

/// Living: `ShortName`, `LongName`, `Health`, `Endurance` and `Mana`, at
/// their defaults.  Whatever takes it in sets its own values.
pub fn living_template() -> Template {
    let mut living = Template::new(LIVING);
    living.add_default(Kind::ShortName);
    living.add_default(Kind::LongName);
    living.add_default(Kind::Health);
    living.add_default(Kind::Endurance);
    living.add_default(Kind::Mana);
    living
}

/// A new character's health, endurance and mana, each.  Jacob's numbers.
pub const STARTING_POOLS: u32 = 10;

/// Character: Living with its pools full at `STARTING_POOLS`, plus a
/// `Transform` (at 0, 0, 0, where everybody starts for now), a capsule,
/// and a `PlayerCharacter` belonging to nobody until one is made from a
/// save.
pub fn character_template() -> Template {
    let mut character = Template::new(CHARACTER);
    character.take_in(&living_template());
    character.add(Component::Health(Pool::full(STARTING_POOLS)));
    character.add(Component::Endurance(Pool::full(STARTING_POOLS)));
    character.add(Component::Mana(Pool::full(STARTING_POOLS)));
    character.add_default(Kind::Transform);
    character.add(Component::PrimitiveShape(PrimitiveShape::Capsule));
    character.add_default(Kind::PlayerCharacter);
    character
}

/// A player's character, ready to spawn: the Character template, the save
/// laid over it, and the account and row it belongs to.  An error says
/// what's wrong with the save, and nothing should be spawned.
pub fn character_from_save(account_id: i64, character_id: i64, save: &Save) -> Result<Blueprint, String> {
    let mut blueprint = Blueprint::from_template("character", &character_template());
    save.apply(&mut blueprint)?;
    blueprint.set(Component::PlayerCharacter(PlayerCharacter::new(account_id, character_id)));
    check_living(&blueprint)?;
    Ok(blueprint)
}

/// Whether a blueprint that is Living has what a living thing must: every
/// one of Living's components, and a short name that isn't blank.
pub fn check_living(blueprint: &Blueprint) -> Result<(), String> {
    if !blueprint.templates().iter().any(|name| name == LIVING) {
        return Ok(());
    }
    for component in living_template().components() {
        if !blueprint.has(component.kind()) {
            return Err(format!("it's living, so it needs a {}, and it hasn't one", component.kind().name()));
        }
    }
    match blueprint.get(Kind::ShortName) {
        Some(Component::ShortName(name)) if !name.text.trim().is_empty() => Ok(()),
        _ => Err("it's living, so it needs a name, and its ShortName is blank".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{LongName, ShortName, Transform, Vector3};
    use crate::save::Fields;
    use crate::world::World;

    fn jacobs_save() -> Save {
        let mut name = Fields::new();
        name.put_text("text", "jacob");
        let mut long_name = Fields::new();
        long_name.put_text("text", "Jacob the Brave");
        let mut transform = Fields::new();
        transform.put_vector3("position", Vector3::new(10.0, 0.0, -4.0));
        let mut health = Fields::new();
        health.put_number("current", 80.0);
        health.put_number("max", 100.0);
        Save {
            templates: vec![CHARACTER.to_string(), LIVING.to_string()],
            components: vec![
                (Kind::Transform, transform),
                (Kind::ShortName, name),
                (Kind::LongName, long_name),
                (Kind::Health, health),
            ],
        }
    }

    #[test]
    fn living_is_the_names_and_the_three_pools() {
        let kinds: Vec<Kind> = living_template().components().iter().map(|component| component.kind()).collect();
        assert_eq!(kinds, vec![Kind::ShortName, Kind::LongName, Kind::Health, Kind::Endurance, Kind::Mana]);
    }

    #[test]
    fn a_character_is_living_with_a_place_a_capsule_and_a_player() {
        let character = character_template();
        assert!(character.is(CHARACTER) && character.is(LIVING));
        let kinds: Vec<Kind> = character.components().iter().map(|component| component.kind()).collect();
        assert_eq!(kinds, vec![
            Kind::ShortName, Kind::LongName, Kind::Health, Kind::Endurance, Kind::Mana,
            Kind::Transform, Kind::PrimitiveShape, Kind::PlayerCharacter,
        ]);
        assert!(character.components().contains(&Component::PrimitiveShape(PrimitiveShape::Capsule)));
        for pool in [Component::Health(Pool::full(10)), Component::Endurance(Pool::full(10)),
                     Component::Mana(Pool::full(10))] {
            assert!(character.components().contains(&pool), "a new character starts with 10 of each");
        }
    }

    #[test]
    fn a_character_is_made_from_its_save() {
        let blueprint = character_from_save(7, 42, &jacobs_save());
        assert!(blueprint.is_ok(), "{blueprint:?}");
        let blueprint = blueprint.unwrap_or_else(|_| Blueprint::from_template("x", &Template::new("x")));

        let mut world = World::new();
        let jacob = world.spawn(&blueprint);
        assert!(world.is(jacob, LIVING) && world.is(jacob, CHARACTER));
        assert_eq!(world.player_character(jacob), Some(&PlayerCharacter::new(7, 42)));
        assert_eq!(world.short_name(jacob), Some(&ShortName::new("jacob")));
        assert_eq!(world.long_name(jacob), Some(&LongName::new("Jacob the Brave")));
        assert_eq!(world.transform(jacob).map(|transform| transform.position), Some(Vector3::new(10.0, 0.0, -4.0)));
        assert_eq!(world.transform(jacob).map(|transform| transform.scale), Some(Transform::default().scale));
        assert_eq!(world.health(jacob), Some(&Pool { current: 80, max: 100 }));
        assert_eq!(world.mana(jacob), Some(&Pool::full(10)), "not in the save, so the template's");
        assert_eq!(world.primitive_shape(jacob), Some(&PrimitiveShape::Capsule));
    }

    #[test]
    fn a_living_thing_without_a_name_is_turned_away() {
        let mut save = jacobs_save();
        save.components.retain(|(kind, _)| *kind != Kind::ShortName);
        let why = character_from_save(7, 42, &save).unwrap_err();
        assert!(why.contains("name"), "{why}");
    }

    #[test]
    fn a_living_thing_missing_a_pool_is_turned_away() {
        let mut blueprint = Blueprint::from_template("character", &character_template());
        blueprint.set(Component::ShortName(ShortName::new("jacob")));
        assert_eq!(check_living(&blueprint), Ok(()));
        blueprint.remove(Kind::Endurance);
        let why = check_living(&blueprint).unwrap_err();
        assert!(why.contains("Endurance"), "{why}");
    }
}
