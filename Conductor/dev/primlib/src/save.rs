//! File:       Opus/Conductor/dev/primlib/src/save.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Saving a GameObject: the templates it came from, and every component's
//! saved fields, written out as Lua text for its database row.
//!
//! Which fields a component keeps is up to the component.  Each one has a
//! `saved()` right under its struct in `components.rs` that names the
//! fields it keeps, and a `load()` that reads them back.  It's the job a
//! `[SavedField]` attribute would do in C#, done with plain functions.  A
//! field that isn't named there isn't saved, and on the way back it keeps
//! whatever the template gave it.
//!
//! Reading the text back is Lua's job, so it lives in `lua-parser`, which
//! hands back a `Save`.  `Save::apply()` then lays it over a blueprint:
//! start from the template, put the saved values on top.  So a component
//! added to a template later turns up on old saves too, at its default.

use crate::components::{Component, Kind, Vector3};
use crate::entity::Entity;
use crate::template::Blueprint;
use crate::world::World;

/// One saved value.  Only what Lua and a database row can both hold
/// plainly: numbers, text, true or false, and lists and tables of those.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Text(String),
    Bool(bool),
    List(Vec<Value>),
    Map(Vec<(String, Value)>),
}

impl Value {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(number) => Some(*number),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(list) => Some(list),
            _ => None,
        }
    }

    /// The value's named fields.  An empty list counts as an empty table
    /// too, since Lua can't tell `{}` the list from `{}` the table.
    pub fn as_map(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Map(fields) => Some(fields),
            Value::List(list) if list.is_empty() => Some(&[]),
            _ => None,
        }
    }

    /// What kind of value it is, for a message saying it's the wrong one.
    fn describe(&self) -> String {
        match self {
            Value::Number(number) => format!("the number {number}"),
            Value::Text(_) => "text".to_string(),
            Value::Bool(value) => value.to_string(),
            Value::List(_) => "a list".to_string(),
            Value::Map(_) => "a table of fields".to_string(),
        }
    }
}

/// One component's saved fields, by name.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fields {
    list: Vec<(String, Value)>,
}

impl Fields {
    pub fn new() -> Fields {
        Fields { list: Vec::new() }
    }

    /// Fields read back from a save.
    pub fn from_list(list: Vec<(String, Value)>) -> Fields {
        Fields { list }
    }

    pub fn list(&self) -> &[(String, Value)] {
        &self.list
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.list.iter().find(|(field, _)| field == name).map(|(_, value)| value)
    }

    /// Sets a field, in place of one with the same name if there is one.
    pub fn put(&mut self, name: &str, value: Value) {
        match self.list.iter_mut().find(|(field, _)| field == name) {
            Some((_, existing)) => *existing = value,
            None => self.list.push((name.to_string(), value)),
        }
    }

    // The ones a component's `saved()` calls, one per kind of field.

    pub fn put_number(&mut self, name: &str, number: f64) {
        self.put(name, Value::Number(number));
    }

    pub fn put_text(&mut self, name: &str, text: &str) {
        self.put(name, Value::Text(text.to_string()));
    }

    pub fn put_bool(&mut self, name: &str, value: bool) {
        self.put(name, Value::Bool(value));
    }

    pub fn put_texts(&mut self, name: &str, texts: &[String]) {
        self.put(name, Value::List(texts.iter().map(|text| Value::Text(text.clone())).collect()));
    }

    /// A Vector3 as `{ x = ..., y = ..., z = ... }`.
    pub fn put_vector3(&mut self, name: &str, vector: Vector3) {
        // Rust note: `f64::from()` widens an f32 to an f64 exactly, so the
        // number written is the f32's own value, and reading it back and
        // narrowing it gives the same f32.
        self.put(name, Value::Map(vec![
            ("x".to_string(), Value::Number(f64::from(vector.x))),
            ("y".to_string(), Value::Number(f64::from(vector.y))),
            ("z".to_string(), Value::Number(f64::from(vector.z))),
        ]));
    }

    // The ones a component's `load()` calls.  Each leaves `into` as it was
    // when the save doesn't have the field, and says what's wrong when the
    // field is there but isn't what it should be.

    pub fn read_u32(&self, name: &str, into: &mut u32) -> Result<(), String> {
        let Some(value) = self.get(name) else {
            return Ok(());
        };
        match value.as_number() {
            Some(number) if number.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&number) => {
                // Rust note: `as` turns the f64 into a u32.  It's safe
                // here: the line above checked it's a whole number that
                // fits.
                *into = number as u32;
                Ok(())
            }
            _ => Err(format!("{name} should be a whole number from 0 to {}, not {}", u32::MAX, value.describe())),
        }
    }

    pub fn read_f32(&self, name: &str, into: &mut f32) -> Result<(), String> {
        let Some(value) = self.get(name) else {
            return Ok(());
        };
        match value.as_number() {
            Some(number) if number.is_finite() => {
                *into = number as f32;
                Ok(())
            }
            _ => Err(format!("{name} should be a number, not {}", value.describe())),
        }
    }

    pub fn read_text(&self, name: &str, into: &mut String) -> Result<(), String> {
        let Some(value) = self.get(name) else {
            return Ok(());
        };
        match value.as_text() {
            Some(text) => {
                *into = text.to_string();
                Ok(())
            }
            None => Err(format!("{name} should be text, not {}", value.describe())),
        }
    }

    pub fn read_bool(&self, name: &str, into: &mut bool) -> Result<(), String> {
        let Some(value) = self.get(name) else {
            return Ok(());
        };
        match value.as_bool() {
            Some(read) => {
                *into = read;
                Ok(())
            }
            None => Err(format!("{name} should be true or false, not {}", value.describe())),
        }
    }

    pub fn read_texts(&self, name: &str, into: &mut Vec<String>) -> Result<(), String> {
        let Some(value) = self.get(name) else {
            return Ok(());
        };
        let wrong = || format!("{name} should be a list of text, not {}", value.describe());
        let list = value.as_list().ok_or_else(wrong)?;
        let mut texts = Vec::new();
        for item in list {
            texts.push(item.as_text().ok_or_else(wrong)?.to_string());
        }
        *into = texts;
        Ok(())
    }

    pub fn read_vector3(&self, name: &str, into: &mut Vector3) -> Result<(), String> {
        let Some(value) = self.get(name) else {
            return Ok(());
        };
        let fields = value.as_map()
            .ok_or_else(|| format!("{name} should be a table with x, y and z, not {}", value.describe()))?;
        let fields = Fields::from_list(fields.to_vec());
        let mut vector = *into;
        fields.read_f32("x", &mut vector.x).map_err(|why| format!("{name}: {why}"))?;
        fields.read_f32("y", &mut vector.y).map_err(|why| format!("{name}: {why}"))?;
        fields.read_f32("z", &mut vector.z).map_err(|why| format!("{name}: {why}"))?;
        *into = vector;
        Ok(())
    }
}

/// A saved GameObject: the templates it came from, and each component's
/// saved fields.  A component with nothing to save isn't in it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Save {
    /// Its own template first, then the ones that took in
    /// (`Character`, `Living`).
    pub templates: Vec<String>,
    pub components: Vec<(Kind, Fields)>,
}

impl Save {
    /// The save of one GameObject as it is right now.  None if it's gone.
    pub fn of(world: &World, entity: Entity) -> Option<Save> {
        if !world.is_alive(entity) {
            return None;
        }
        let mut save = Save { templates: world.templates(entity).to_vec(), components: Vec::new() };
        for kind in world.kinds(entity) {
            let Some(component) = world.component(entity, kind) else {
                continue;
            };
            let mut fields = Fields::new();
            component.saved(&mut fields);
            if !fields.is_empty() {
                save.components.push((kind, fields));
            }
        }
        Some(save)
    }

    /// Lays the saved values over a blueprint made from the template: each
    /// saved component starts as the blueprint's (or its default, if the
    /// blueprint doesn't have that kind) and takes the save's fields.  The
    /// first thing wrong in the save is the answer, and the blueprint is
    /// left half done, so a caller that gets an error throws it away.
    pub fn apply(&self, blueprint: &mut Blueprint) -> Result<(), String> {
        for (kind, fields) in &self.components {
            let mut component = blueprint.get(*kind).cloned().unwrap_or_else(|| Component::default_of(*kind));
            component.load(fields).map_err(|why| format!("{}: {why}", kind.name()))?;
            blueprint.set(component);
        }
        Ok(())
    }

    /// The save as Lua text, for the database row.  It's a script that
    /// hands back one table, and `lua-parser` runs it in a locked-down Lua
    /// to read it back.
    pub fn to_lua(&self) -> String {
        let mut text = String::from("-- A saved GameObject, written by Conductor.  Read back in a locked-down Lua.\n");
        text.push_str("return {\n");
        let templates: Vec<String> = self.templates.iter().map(|name| quote(name)).collect();
        text.push_str(&format!("  templates = {{ {} }},\n", templates.join(", ")));
        text.push_str("  components = {\n");
        for (kind, fields) in &self.components {
            text.push_str(&format!("    {} = ", kind.name()));
            write_map(&mut text, fields.list());
            text.push_str(",\n");
        }
        text.push_str("  },\n}\n");
        text
    }
}

fn write_value(text: &mut String, value: &Value) {
    match value {
        Value::Number(number) => text.push_str(&number_text(*number)),
        Value::Text(words) => text.push_str(&quote(words)),
        Value::Bool(value) => text.push_str(if *value { "true" } else { "false" }),
        Value::List(list) => {
            if list.is_empty() {
                text.push_str("{}");
                return;
            }
            text.push_str("{ ");
            for (number, item) in list.iter().enumerate() {
                if number > 0 {
                    text.push_str(", ");
                }
                write_value(text, item);
            }
            text.push_str(" }");
        }
        Value::Map(fields) => write_map(text, fields),
    }
}

/// `{ name = value, ... }`.  The names are ours (a component's field
/// names), so they're always plain Lua names and need no quotes.
fn write_map(text: &mut String, fields: &[(String, Value)]) {
    if fields.is_empty() {
        text.push_str("{}");
        return;
    }
    text.push_str("{ ");
    for (number, (name, value)) in fields.iter().enumerate() {
        if number > 0 {
            text.push_str(", ");
        }
        text.push_str(name);
        text.push_str(" = ");
        write_value(text, value);
    }
    text.push_str(" }");
}

/// A number as Lua reads it.  A whole number is written without a point
/// (`2000`), anything else as Rust's shortest form that reads back to the
/// same number (`0.10000000149011612`).  A number that isn't one (NaN, or
/// infinity) can't be written in Lua at all, and is written as 0 rather
/// than leave a save that won't load.
fn number_text(number: f64) -> String {
    if !number.is_finite() {
        return "0".to_string();
    }
    if number.fract() == 0.0 && number.abs() < 1e15 {
        // Rust note: `as i64` drops the ".0" Rust would otherwise print.
        // Safe here: it's whole, and well inside what an i64 holds.
        return (number as i64).to_string();
    }
    format!("{number:?}")
}

/// Text in double quotes, the way Lua reads it back exactly: backslashes,
/// quotes and line breaks escaped, and any other control character as a
/// three-digit `\ddd`.  Everything else, accents and all, goes as it is.
fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for letter in text.chars() {
        match letter {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if (control as u32) < 32 || control as u32 == 127 => {
                out.push_str(&format!("\\{:03}", control as u32));
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Pool, PrimitiveShape, ShortName, Titles, Transform};
    use crate::gameobject;

    #[test]
    fn a_save_keeps_only_the_saved_fields() {
        let mut world = World::new();
        let mut blueprint = Blueprint::from_template("character", &gameobject::character_template());
        blueprint.set(Component::ShortName(ShortName::new("jacob")));
        let jacob = world.spawn(&blueprint);

        let save = Save::of(&world, jacob).unwrap_or_default();
        assert_eq!(save.templates, vec!["Character".to_string(), "Living".to_string()]);
        let kinds: Vec<Kind> = save.components.iter().map(|(kind, _)| *kind).collect();
        assert!(kinds.contains(&Kind::Transform) && kinds.contains(&Kind::Health));
        assert!(!kinds.contains(&Kind::PrimitiveShape), "the shape always comes from the template");
        assert!(!kinds.contains(&Kind::PlayerCharacter), "rebuilt from the row, never saved");

        let transform = save.components.iter()
            .find(|(kind, _)| *kind == Kind::Transform)
            .map(|(_, fields)| fields.clone())
            .unwrap_or_default();
        assert!(transform.get("position").is_some() && transform.get("scale").is_some());
        assert!(transform.get("parent").is_none(), "only the fields saved() names");
    }

    #[test]
    fn the_lua_text_reads_plainly() {
        let mut fields = Fields::new();
        fields.put_number("current", 1500.0);
        fields.put_number("max", 2000.0);
        let save = Save { templates: vec!["Living".to_string()], components: vec![(Kind::Health, fields)] };
        let text = save.to_lua();
        assert!(text.contains("templates = { \"Living\" }"), "{text}");
        assert!(text.contains("Health = { current = 1500, max = 2000 },"), "{text}");
        assert!(text.contains("return {"), "{text}");
    }

    #[test]
    fn text_is_quoted_so_lua_reads_it_back_exactly() {
        assert_eq!(quote("plain"), "\"plain\"");
        assert_eq!(quote("say \"hi\"\\now"), "\"say \\\"hi\\\"\\\\now\"");
        assert_eq!(quote("two\nlines"), "\"two\\nlines\"");
        assert_eq!(quote("bell\u{7}"), "\"bell\\007\"");
        assert_eq!(quote("Séréna"), "\"Séréna\"");
    }

    #[test]
    fn numbers_are_written_the_way_lua_reads_them() {
        assert_eq!(number_text(2000.0), "2000");
        assert_eq!(number_text(-3.0), "-3");
        assert_eq!(number_text(f64::from(0.1f32)), "0.10000000149011612");
        assert_eq!(number_text(f64::NAN), "0");
    }

    #[test]
    fn a_save_lays_over_the_template() {
        let mut world = World::new();
        let template = gameobject::character_template();
        let mut original = Blueprint::from_template("character", &template);
        original.set(Component::ShortName(ShortName::new("jacob")));
        original.set(Component::Transform(Transform::at(Vector3::new(4.5, 1.0, -2.25))));
        original.set(Component::Health(Pool { current: 150, max: 200 }));
        original.set(Component::Titles(Titles::one("the Brave")));
        let jacob = world.spawn(&original);
        let save = Save::of(&world, jacob).unwrap_or_default();

        let mut again = Blueprint::from_template("character", &template);
        assert_eq!(save.apply(&mut again), Ok(()));
        assert_eq!(again.get(Kind::ShortName), original.get(Kind::ShortName));
        assert_eq!(again.get(Kind::Transform), original.get(Kind::Transform));
        assert_eq!(again.get(Kind::Health), original.get(Kind::Health));
        assert_eq!(again.get(Kind::Titles), original.get(Kind::Titles), "titles weren't in the template");
        assert_eq!(again.get(Kind::PrimitiveShape), Some(&Component::PrimitiveShape(PrimitiveShape::Capsule)));
    }

    #[test]
    fn a_wrong_field_is_turned_away_with_why() {
        let mut fields = Fields::new();
        fields.put_text("current", "lots");
        let save = Save { templates: Vec::new(), components: vec![(Kind::Health, fields)] };
        let mut blueprint = Blueprint::from_template("character", &gameobject::character_template());
        let why = save.apply(&mut blueprint).unwrap_err();
        assert!(why.contains("Health") && why.contains("current") && why.contains("whole number"), "{why}");

        let mut pool = Fields::new();
        pool.put_number("max", 1.5);
        let mut max = 0;
        assert!(pool.read_u32("max", &mut max).is_err());
        assert!(pool.read_u32("missing", &mut max).is_ok(), "a field the save doesn't have is fine");
    }
}
