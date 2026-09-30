//! File:       Opus/Conductor/dev/lua-parser/src/save.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Reading a saved GameObject back.  primlib writes a save as Lua text
//! (`return { templates = ..., components = ... }`), and this runs it in
//! the same locked-down Lua every script gets, then turns the table it
//! hands back into primlib's `Save`.
//!
//! A save is only data, so anything in it that isn't (a function, say)
//! means it's been tampered with or mangled, and it's turned away.  So is
//! one that's the wrong shape.  A component name the game doesn't know any
//! more (one taken out since the save was written) is a Warn, and the rest
//! of the save still loads.

use conductor_primlib::{Fields, Kind, Save, Value};
use conductor_tools::scribe::{self, Channel};
use mlua::{Table, Value as LuaValue};

use crate::sandbox;

/// How deep tables can nest in a save.  A save is three or four deep
/// (the save, its components, a component, a position); more than this
/// isn't one of ours.
const MAX_DEPTH: u32 = 8;

/// Reads a save back.  `name` says where it came from, for the log
/// (`player_characters row 42`).  An error is the Warn to log, and the
/// save shouldn't be used.
pub fn read_save(name: &str, text: &str) -> Result<Save, String> {
    let top = sandbox::evaluate(name, text, |value| {
        convert(value, 0).map_err(|why| format!("{name} isn't a save: {why}"))
    })?;
    let wrong = |why: &str| format!("{name} isn't a save: {why}");

    let fields = top.as_map().ok_or_else(|| wrong("it should hand back a table"))?;
    let fields = Fields::from_list(fields.to_vec());

    let mut save = Save::default();
    if let Some(templates) = fields.get("templates") {
        let list = templates.as_list().ok_or_else(|| wrong("templates should be a list of names"))?;
        for template in list {
            let template = template.as_text().ok_or_else(|| wrong("templates should be a list of names"))?;
            save.templates.push(template.to_string());
        }
    }

    let components: &[(String, Value)] = match fields.get("components") {
        Some(components) => components.as_map().ok_or_else(|| wrong("components should be a table"))?,
        None => &[],
    };
    for (kind_name, component) in components {
        let Some(kind) = Kind::from_name(kind_name) else {
            scribe::warn(Channel::Script, &format!("{name} has a {kind_name}, which isn't a kind of component \
                any more.  It's left out, and the rest of the save loads."));
            continue;
        };
        let component_fields = component.as_map()
            .ok_or_else(|| wrong(&format!("{kind_name} should be a table of fields")))?;
        save.components.push((kind, Fields::from_list(component_fields.to_vec())));
    }
    // In the order the kinds are listed, so a save reads back the same
    // however Lua happened to walk the table.
    save.components.sort_by_key(|(kind, _)| Kind::ALL.iter().position(|each| each == kind));
    Ok(save)
}

/// One Lua value as a save's value.  Only numbers, text, true or false,
/// and tables of those.
fn convert(value: LuaValue, depth: u32) -> Result<Value, String> {
    if depth > MAX_DEPTH {
        return Err(format!("its tables are nested more than {MAX_DEPTH} deep"));
    }
    match value {
        LuaValue::Boolean(value) => Ok(Value::Bool(value)),
        // Rust note: `as f64` turns Lua's whole number into a plain number.
        // Past 2^53 it would lose the last digits, far beyond anything a
        // save holds.
        LuaValue::Integer(number) => Ok(Value::Number(number as f64)),
        LuaValue::Number(number) => Ok(Value::Number(number)),
        LuaValue::String(text) => match text.to_str() {
            Ok(text) => Ok(Value::Text((*text).to_string())),
            Err(_) => Err("it has text that isn't UTF-8".to_string()),
        },
        LuaValue::Table(table) => convert_table(table, depth),
        other => Err(format!("it has a {}, where only numbers, text, true or false and tables belong",
                             other.type_name())),
    }
}

/// A Lua table as a list (keys 1, 2, 3 ...) or as named fields.  A table
/// with both, or a list with a gap, isn't one of ours.
fn convert_table(table: Table, depth: u32) -> Result<Value, String> {
    let mut numbered: Vec<(i64, Value)> = Vec::new();
    let mut named: Vec<(String, Value)> = Vec::new();
    // Rust note: `pairs` walks every key and value in the table, each one
    // a `Result` since reading a Lua value can fail.
    for pair in table.pairs::<LuaValue, LuaValue>() {
        let (key, value) = pair.map_err(|e| format!("a table couldn't be read ({e})"))?;
        let value = convert(value, depth + 1)?;
        match key {
            LuaValue::Integer(number) => numbered.push((number, value)),
            LuaValue::String(text) => match text.to_str() {
                Ok(text) => named.push(((*text).to_string(), value)),
                Err(_) => return Err("it has a name that isn't UTF-8".to_string()),
            },
            other => return Err(format!("it has a table keyed by a {}", other.type_name())),
        }
    }

    if !numbered.is_empty() && !named.is_empty() {
        return Err("it has a table that's both a list and named fields".to_string());
    }
    if named.is_empty() {
        numbered.sort_by_key(|(number, _)| *number);
        for (place, (number, _)) in numbered.iter().enumerate() {
            if *number != place as i64 + 1 {
                return Err("it has a list with a gap in it".to_string());
            }
        }
        return Ok(Value::List(numbered.into_iter().map(|(_, value)| value).collect()));
    }
    named.sort_by(|(one, _), (other, _)| one.cmp(other));
    Ok(Value::Map(named))
}

#[cfg(test)]
mod tests {
    use super::*;
    use conductor_primlib::gameobject::{self, CHARACTER, LIVING};
    use conductor_primlib::{
        Blueprint, Component, LongName, PlayerCharacter, Pool, ShortName, Titles, Transform, Vector3, World,
    };

    /// A character with a bit of everything worth saving, spawned.
    fn jacob(world: &mut World) -> conductor_primlib::Entity {
        let mut blueprint = Blueprint::from_template("character", &gameobject::character_template());
        blueprint.set(Component::ShortName(ShortName::new("jacob")));
        blueprint.set(Component::LongName(LongName::new("Jacob \"the Brave\"\nof Opus")));
        let mut transform = Transform::at(Vector3::new(12.5, 0.1, -7.25));
        transform.rotation = Vector3::new(0.0, 90.0, 0.0);
        blueprint.set(Component::Transform(transform));
        blueprint.set(Component::Health(Pool { current: 150, max: 200 }));
        blueprint.set(Component::Endurance(Pool::full(80)));
        blueprint.set(Component::Mana(Pool { current: 0, max: 30 }));
        let mut titles = Titles::one("the Brave");
        titles.add("the Lost");
        titles.pick(1);
        blueprint.set(Component::Titles(titles));
        blueprint.set(Component::PlayerCharacter(PlayerCharacter::new(7, 42)));
        world.spawn(&blueprint)
    }

    #[test]
    fn a_character_goes_out_as_lua_and_comes_back_the_same() {
        let mut world = World::new();
        let original = jacob(&mut world);
        let text = Save::of(&world, original).unwrap_or_default().to_lua();

        let save = read_save("player_characters row 42", &text);
        assert!(save.is_ok(), "{save:?}\n{text}");
        let save = save.unwrap_or_default();
        assert_eq!(save.templates, vec![CHARACTER.to_string(), LIVING.to_string()]);

        let blueprint = gameobject::character_from_save(7, 42, &save);
        assert!(blueprint.is_ok(), "{blueprint:?}");
        let blueprint = blueprint
            .unwrap_or_else(|_| Blueprint::from_template("x", &conductor_primlib::Template::new("x")));
        let again = world.spawn(&blueprint);

        assert_eq!(world.kinds(again), world.kinds(original));
        for kind in world.kinds(original) {
            assert_eq!(world.component(again, kind), world.component(original, kind), "{}", kind.name());
        }
        assert!(world.is(again, LIVING));
    }

    #[test]
    fn a_save_that_isnt_lua_is_turned_away() {
        let why = read_save("row 1", "return { templates = ").unwrap_err();
        assert!(why.contains("row 1") && why.contains("didn't load"), "{why}");
    }

    #[test]
    fn a_save_of_the_wrong_shape_is_turned_away() {
        let why = read_save("row 1", "return 5").unwrap_err();
        assert!(why.contains("should hand back a table"), "{why}");
        let why = read_save("row 1", "return { components = { Health = 5 } }").unwrap_err();
        assert!(why.contains("Health should be a table"), "{why}");
        let why = read_save("row 1", "return { templates = { 1, 2 } }").unwrap_err();
        assert!(why.contains("list of names"), "{why}");
    }

    #[test]
    fn a_save_with_code_in_it_is_turned_away() {
        let why = read_save("row 1", "return { components = { Health = { current = function() end } } }")
            .unwrap_err();
        assert!(why.contains("function"), "{why}");
    }

    #[test]
    fn a_save_that_never_finishes_is_stopped() {
        let why = read_save("row 1", "while true do end").unwrap_err();
        assert!(why.contains("time limit"), "{why}");
    }

    #[test]
    fn a_kind_the_game_dropped_is_left_out_and_the_rest_loads() {
        let text = "return { components = { Wings = { span = 3 }, Health = { current = 5, max = 9 } } }";
        let save = read_save("row 1", text);
        assert!(save.is_ok(), "{save:?}");
        let kinds: Vec<Kind> = save.unwrap_or_default().components.iter().map(|(kind, _)| *kind).collect();
        assert_eq!(kinds, vec![Kind::Health]);
    }

    #[test]
    fn a_list_with_a_gap_is_turned_away() {
        let why = read_save("row 1", "return { templates = { [1] = 'Living', [3] = 'Character' } }").unwrap_err();
        assert!(why.contains("gap"), "{why}");
    }
}
