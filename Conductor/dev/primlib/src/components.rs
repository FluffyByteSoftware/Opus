//! File:       Opus/Conductor/dev/primlib/src/components.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The components: plain data, one struct per kind.  The first ones are
//! from my sample NPC, with its position, rotation and scale made into one
//! Transform the way Unity has it, then what the client draws: a Model, a
//! PrimitiveShape to fall back on, and an Animator.  `PlayerCharacter`
//! says a player steers it.  A `Collider` is the room it takes up in the
//! world (2026-10-03, movement).  `Kind` names a kind of component (for a
//! template's list, a script, or the log), and `Component` is one of them
//! with its value, which is how a template or blueprint holds them.
//!
//! Right under each struct, `saved()` names the fields a save keeps and
//! `load()` reads them back (`save.rs` has the rest).  A field that isn't
//! named in `saved()` isn't saved.
//!
//! Adding a kind: its struct here with its `saved()` and `load()`, a line
//! in `Kind` (and `Kind::ALL` and `name()`), a line in `Component` (and
//! `kind()`, `default_of()`, `saved()` and `load()`), then its store, the
//! four matches and two getters in `world.rs`.

use crate::save::Fields;

/// Three numbers, x, y and z, as in Unity.  Y is up.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vector3 {
        Vector3 { x, y, z }
    }
}

/// Where an object is, which way it faces, and how big it is, the way
/// Unity's Transform has them.
///
/// The rotation is three angles in degrees, the way Unity's inspector
/// shows one.  Ensemble turns them into Unity's own rotation with
/// `Quaternion.Euler(x, y, z)`.  The server doesn't do any rotation math
/// yet; if it ever has to, this is the one place that changes.
///
/// No parent yet: every transform is in the world's own terms.
///
/// The velocity isn't Unity's (its Transform has none): it's where the
/// object is going, in blocks a second, which every client that sees it
/// is sent so it can carry the object along between one word from the
/// server and the next (movement, 2026-10-03).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub position: Vector3,
    pub rotation: Vector3,
    /// 1, 1, 1 is as the model was made.
    pub scale: Vector3,
    /// Blocks a second along x, y and z.  0, 0, 0 is standing still.
    pub velocity: Vector3,
}

impl Transform {
    /// At `position`, facing the way the model was made, at its own size,
    /// standing still.
    pub fn at(position: Vector3) -> Transform {
        Transform { position, rotation: Vector3::default(), scale: Vector3::new(1.0, 1.0, 1.0),
                    velocity: Vector3::default() }
    }

    /// Whether it's standing still.
    pub fn is_still(&self) -> bool {
        self.velocity == Vector3::default()
    }

    /// Saved: where it is, which way it faces, and how big it is.  Not
    /// the velocity: a character comes back into the world standing still.
    pub fn saved(&self, out: &mut Fields) {
        out.put_vector3("position", self.position);
        out.put_vector3("rotation", self.rotation);
        out.put_vector3("scale", self.scale);
    }

    pub fn load(&mut self, from: &Fields) -> Result<(), String> {
        from.read_vector3("position", &mut self.position)?;
        from.read_vector3("rotation", &mut self.rotation)?;
        from.read_vector3("scale", &mut self.scale)
    }
}

// Rust note: `Default` is written out by hand here, because the one Rust
// would make for us starts every number at 0, and a scale of 0 is an
// object nobody can see.
impl Default for Transform {
    fn default() -> Transform {
        Transform::at(Vector3::default())
    }
}

/// The model the client draws for this object: the path it loads it from.
/// The server never opens it; it's only passed along for the client to
/// make sense of.  A string rather than a list of every model, because a
/// list was tried once and got messy.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Model {
    pub path: String,
}

impl Model {
    pub fn new(path: &str) -> Model {
        Model { path: path.to_string() }
    }

    /// Nothing saved: the model is what the template or blueprint says.
    pub fn saved(&self, _out: &mut Fields) {}

    pub fn load(&mut self, _from: &Fields) -> Result<(), String> {
        Ok(())
    }
}

/// The shape the client draws when it can't draw the object's model:
/// Unity's six built-in ones.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PrimitiveShape {
    // Rust note: `#[default]` marks the one `PrimitiveShape::default()`
    // hands back.
    #[default]
    Cube,
    Sphere,
    Capsule,
    Cylinder,
    Plane,
    Quad,
}

impl PrimitiveShape {
    /// Nothing saved: the shape is what the template says.
    pub fn saved(&self, _out: &mut Fields) {}

    pub fn load(&mut self, _from: &Fields) -> Result<(), String> {
        Ok(())
    }
}

/// The room an object takes up in the world, for bumping into things: a
/// capsule, a cylinder or a box, standing with its bottom at the object's
/// position (a character's feet), the same as the shape the client draws.
/// Jacob, 2026-10-03: "we're gonna design in our game library collider
/// primitives capsule/cylinder, and cube that should cover our needs for
/// this.  This will have to be our representative of the player in the
/// servers memory."  The client builds its own collider from these
/// numbers, so the two sides agree on how big a character is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Collider {
    /// A capsule `height` tall, `radius` round, its ends rounded.
    Capsule { radius: f32, height: f32 },
    /// A cylinder `height` tall, `radius` round, flat at both ends.
    Cylinder { radius: f32, height: f32 },
    /// A box `size` across, x, y and z.
    Cube { size: Vector3 },
}

impl Collider {
    /// A character's: a capsule 1 block wide and 2 tall, the shape it's
    /// drawn as (Jacob, 2026-10-03: "our current shape").
    pub const CHARACTER: Collider = Collider::Capsule { radius: 0.5, height: 2.0 };

    /// How far it reaches out from its middle, flat on the ground: the
    /// radius, or half the box's wider side.
    pub fn reach(&self) -> f32 {
        match *self {
            Collider::Capsule { radius, .. } | Collider::Cylinder { radius, .. } => radius,
            Collider::Cube { size } => size.x.max(size.z) / 2.0,
        }
    }

    /// How tall it is.
    pub fn height(&self) -> f32 {
        match *self {
            Collider::Capsule { height, .. } | Collider::Cylinder { height, .. } => height,
            Collider::Cube { size } => size.y,
        }
    }

    /// Nothing saved: the collider is what the template says.
    pub fn saved(&self, _out: &mut Fields) {}

    pub fn load(&mut self, _from: &Fields) -> Result<(), String> {
        Ok(())
    }
}

// Rust note: written out by hand, since an enum whose choices carry data
// can't just mark one `#[default]` with its numbers.  A box one block a
// side, the same as the cube drawn by default.
impl Default for Collider {
    fn default() -> Collider {
        Collider::Cube { size: Vector3::new(1.0, 1.0, 1.0) }
    }
}

/// What the object's model is doing, for the client to play.  A skeleton
/// for now: animations in the game come later.  An object without one
/// isn't animated.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Animator {
    /// The name of what the model is doing ("idle", "walk").  Empty is
    /// nothing.
    pub current_track: String,
    pub is_looping_currently: bool,
}

impl Animator {
    /// Playing `track`, looping or not.
    pub fn playing(track: &str, looping: bool) -> Animator {
        Animator { current_track: track.to_string(), is_looping_currently: looping }
    }

    /// Nothing saved: what the model was doing doesn't outlast a logout.
    pub fn saved(&self, _out: &mut Fields) {}

    pub fn load(&mut self, _from: &Fields) -> Result<(), String> {
        Ok(())
    }
}

/// What an object is called for short, usually lower case: "goblin"
/// rather than "a Goblin Warrior".
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShortName {
    pub text: String,
}

impl ShortName {
    pub fn new(text: &str) -> ShortName {
        ShortName { text: text.to_string() }
    }

    /// Saved: the name.
    pub fn saved(&self, out: &mut Fields) {
        out.put_text("text", &self.text);
    }

    pub fn load(&mut self, from: &Fields) -> Result<(), String> {
        from.read_text("text", &mut self.text)
    }
}

/// An object's full name, capitalized, with a title or a name of its own
/// if it has one: "a Goblin Warrior named Serah".
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LongName {
    pub text: String,
}

impl LongName {
    pub fn new(text: &str) -> LongName {
        LongName { text: text.to_string() }
    }

    /// Saved: the name.
    pub fn saved(&self, out: &mut Fields) {
        out.put_text("text", &self.text);
    }

    pub fn load(&mut self, from: &Fields) -> Result<(), String> {
        from.read_text("text", &mut self.text)
    }
}

/// The titles an object has earned ("the Cursed", "the plucky"), with one
/// of them picked as the one it goes by.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Titles {
    pub list: Vec<String>,
    /// Which one in the list it goes by, if any.
    pub picked: Option<usize>,
}

impl Titles {
    /// One title, picked.
    pub fn one(title: &str) -> Titles {
        Titles { list: vec![title.to_string()], picked: Some(0) }
    }

    /// Adds a title to the list, without picking it.
    pub fn add(&mut self, title: &str) {
        self.list.push(title.to_string());
    }

    /// Picks the title at `index` to go by.  False, and nothing changes, if
    /// there isn't one there.
    pub fn pick(&mut self, index: usize) -> bool {
        if index >= self.list.len() {
            return false;
        }
        self.picked = Some(index);
        true
    }

    /// The title it goes by, if one is picked.
    pub fn current(&self) -> Option<&str> {
        match self.picked {
            Some(index) => self.list.get(index).map(|title| title.as_str()),
            None => None,
        }
    }

    /// Saved: every title, and which one it goes by.  `picked` is counted
    /// from 1 the way Lua counts, and 0 is none picked.
    pub fn saved(&self, out: &mut Fields) {
        out.put_texts("list", &self.list);
        let picked = match self.picked {
            Some(index) => index + 1,
            None => 0,
        };
        out.put_number("picked", picked as f64);
    }

    pub fn load(&mut self, from: &Fields) -> Result<(), String> {
        from.read_texts("list", &mut self.list)?;
        if from.get("picked").is_none() {
            return Ok(());
        }
        let mut picked = 0;
        from.read_u32("picked", &mut picked)?;
        self.picked = match picked {
            0 => None,
            number if (number as usize) <= self.list.len() => Some(number as usize - 1),
            number => return Err(format!("picked is title {number}, but there are only {}", self.list.len())),
        };
        Ok(())
    }
}

/// Something that runs down and fills back up, with a cap: health,
/// endurance and mana are all one of these.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pool {
    pub current: u32,
    pub max: u32,
}

impl Pool {
    /// A pool that starts full.  No need to set the current as well; it
    /// starts at the max unless something says otherwise.
    pub fn full(max: u32) -> Pool {
        Pool { current: max, max }
    }

    /// Takes `amount` off, stopping at 0.
    pub fn damage(&mut self, amount: u32) {
        // Rust note: `saturating_sub` stops at 0 instead of going below
        // it, which a u32 can't do anyway (it would stop the program in a
        // test build and wrap round to four billion in a release one).
        self.current = self.current.saturating_sub(amount);
    }

    /// Puts `amount` back, stopping at the max.
    pub fn heal(&mut self, amount: u32) {
        self.current = self.current.saturating_add(amount).min(self.max);
    }

    /// Sets the current, no higher than the max.
    pub fn set_current(&mut self, current: u32) {
        self.current = current.min(self.max);
    }

    /// Sets the max.  The current comes down with it if it was over.
    pub fn set_max(&mut self, max: u32) {
        self.max = max;
        self.current = self.current.min(max);
    }

    /// Whether it's run dry.
    pub fn is_empty(&self) -> bool {
        self.current == 0
    }

    /// Saved: how full it is, and the cap.
    pub fn saved(&self, out: &mut Fields) {
        out.put_number("current", f64::from(self.current));
        out.put_number("max", f64::from(self.max));
    }

    /// Reads the max first, so a current over it comes down to it.
    pub fn load(&mut self, from: &Fields) -> Result<(), String> {
        from.read_u32("max", &mut self.max)?;
        let mut current = self.current;
        from.read_u32("current", &mut current)?;
        self.current = current.min(self.max);
        Ok(())
    }
}

/// Says a player steers this GameObject: the account it belongs to, and
/// the `player_characters` row it's saved to, both by their `id`.  The
/// account is what the server keeps track of; this is how the game gets
/// from the character back to it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerCharacter {
    pub account_id: i64,
    pub character_id: i64,
}

impl PlayerCharacter {
    pub fn new(account_id: i64, character_id: i64) -> PlayerCharacter {
        PlayerCharacter { account_id, character_id }
    }

    /// Nothing saved: it's put back from the row the save was read from.
    pub fn saved(&self, _out: &mut Fields) {}

    pub fn load(&mut self, _from: &Fields) -> Result<(), String> {
        Ok(())
    }
}

/// A kind of component, by name, without its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Transform,
    Model,
    PrimitiveShape,
    Animator,
    ShortName,
    LongName,
    Titles,
    Health,
    Endurance,
    Mana,
    PlayerCharacter,
    Collider,
}

impl Kind {
    /// Every kind, in the order they're listed here.
    pub const ALL: [Kind; 12] = [
        Kind::Transform,
        Kind::Model,
        Kind::PrimitiveShape,
        Kind::Animator,
        Kind::ShortName,
        Kind::LongName,
        Kind::Titles,
        Kind::Health,
        Kind::Endurance,
        Kind::Mana,
        Kind::PlayerCharacter,
        Kind::Collider,
    ];

    /// The kind's name, as a script or the log would write it.
    pub fn name(&self) -> &'static str {
        match self {
            Kind::Transform => "Transform",
            Kind::Model => "Model",
            Kind::PrimitiveShape => "PrimitiveShape",
            Kind::Animator => "Animator",
            Kind::ShortName => "ShortName",
            Kind::LongName => "LongName",
            Kind::Titles => "Titles",
            Kind::Health => "Health",
            Kind::Endurance => "Endurance",
            Kind::Mana => "Mana",
            Kind::PlayerCharacter => "PlayerCharacter",
            Kind::Collider => "Collider",
        }
    }

    /// The kind with this name, if there is one.  Exact, capitals and all.
    pub fn from_name(name: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|kind| kind.name() == name)
    }
}

/// One component with its value.  A template's list and a blueprint's list
/// are made of these.
// Rust note: an enum in Rust can carry data in each of its choices, so a
// `Component` is exactly one of these, holding that kind's value.
#[derive(Clone, Debug, PartialEq)]
pub enum Component {
    Transform(Transform),
    Model(Model),
    PrimitiveShape(PrimitiveShape),
    Animator(Animator),
    ShortName(ShortName),
    LongName(LongName),
    Titles(Titles),
    Health(Pool),
    Endurance(Pool),
    Mana(Pool),
    PlayerCharacter(PlayerCharacter),
    Collider(Collider),
}

impl Component {
    /// Which kind this is.
    pub fn kind(&self) -> Kind {
        match self {
            Component::Transform(_) => Kind::Transform,
            Component::Model(_) => Kind::Model,
            Component::PrimitiveShape(_) => Kind::PrimitiveShape,
            Component::Animator(_) => Kind::Animator,
            Component::ShortName(_) => Kind::ShortName,
            Component::LongName(_) => Kind::LongName,
            Component::Titles(_) => Kind::Titles,
            Component::Health(_) => Kind::Health,
            Component::Endurance(_) => Kind::Endurance,
            Component::Mana(_) => Kind::Mana,
            Component::PlayerCharacter(_) => Kind::PlayerCharacter,
            Component::Collider(_) => Kind::Collider,
        }
    }

    /// The kind with its default value: a transform at 0, 0, 0 facing the
    /// way its model was made at its own size, no model path, a cube, an
    /// animator playing nothing, empty names and titles, pools of 0, a
    /// player character belonging to nobody, and a box one block a side.  A
    /// template sets its own where these won't do.
    pub fn default_of(kind: Kind) -> Component {
        match kind {
            Kind::Transform => Component::Transform(Transform::default()),
            Kind::Model => Component::Model(Model::default()),
            Kind::PrimitiveShape => Component::PrimitiveShape(PrimitiveShape::default()),
            Kind::Animator => Component::Animator(Animator::default()),
            Kind::ShortName => Component::ShortName(ShortName::default()),
            Kind::LongName => Component::LongName(LongName::default()),
            Kind::Titles => Component::Titles(Titles::default()),
            Kind::Health => Component::Health(Pool::default()),
            Kind::Endurance => Component::Endurance(Pool::default()),
            Kind::Mana => Component::Mana(Pool::default()),
            Kind::PlayerCharacter => Component::PlayerCharacter(PlayerCharacter::default()),
            Kind::Collider => Component::Collider(Collider::default()),
        }
    }

    /// Puts the component's saved fields in `out`, by asking its own
    /// `saved()`.
    pub fn saved(&self, out: &mut Fields) {
        match self {
            Component::Transform(value) => value.saved(out),
            Component::Model(value) => value.saved(out),
            Component::PrimitiveShape(value) => value.saved(out),
            Component::Animator(value) => value.saved(out),
            Component::ShortName(value) => value.saved(out),
            Component::LongName(value) => value.saved(out),
            Component::Titles(value) => value.saved(out),
            Component::Health(value) => value.saved(out),
            Component::Endurance(value) => value.saved(out),
            Component::Mana(value) => value.saved(out),
            Component::PlayerCharacter(value) => value.saved(out),
            Component::Collider(value) => value.saved(out),
        }
    }

    /// Reads saved fields into the component, by asking its own `load()`.
    pub fn load(&mut self, from: &Fields) -> Result<(), String> {
        match self {
            Component::Transform(value) => value.load(from),
            Component::Model(value) => value.load(from),
            Component::PrimitiveShape(value) => value.load(from),
            Component::Animator(value) => value.load(from),
            Component::ShortName(value) => value.load(from),
            Component::LongName(value) => value.load(from),
            Component::Titles(value) => value.load(from),
            Component::Health(value) => value.load(from),
            Component::Endurance(value) => value.load(from),
            Component::Mana(value) => value.load(from),
            Component::PlayerCharacter(value) => value.load(from),
            Component::Collider(value) => value.load(from),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pool_starts_full() {
        let health = Pool::full(2000);
        assert_eq!(health.current, 2000);
        assert_eq!(health.max, 2000);
    }

    #[test]
    fn damage_stops_at_zero_and_healing_at_the_max() {
        let mut health = Pool::full(100);
        health.damage(250);
        assert_eq!(health.current, 0);
        assert!(health.is_empty());
        health.heal(40);
        assert_eq!(health.current, 40);
        health.heal(1000);
        assert_eq!(health.current, 100);
    }

    #[test]
    fn lowering_the_max_brings_the_current_down() {
        let mut mana = Pool::full(50);
        mana.set_max(2);
        assert_eq!(mana, Pool { current: 2, max: 2 });
        mana.set_current(10);
        assert_eq!(mana.current, 2);
    }

    #[test]
    fn titles_pick_one_to_go_by() {
        let mut titles = Titles::one("the plucky");
        assert_eq!(titles.current(), Some("the plucky"));
        titles.add("the Cursed");
        assert!(titles.pick(1));
        assert_eq!(titles.current(), Some("the Cursed"));
        assert!(!titles.pick(5));
        assert_eq!(titles.current(), Some("the Cursed"));
        assert_eq!(Titles::default().current(), None);
    }

    #[test]
    fn a_transform_starts_at_its_own_size() {
        let transform = Transform::default();
        assert_eq!(transform.position, Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(transform.rotation, Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(transform.scale, Vector3::new(1.0, 1.0, 1.0));
        assert!(transform.is_still());
    }

    #[test]
    fn the_velocity_is_never_saved() {
        let mut transform = Transform::at(Vector3::new(1.0, 2.0, 3.0));
        transform.velocity = Vector3::new(4.0, 0.0, 0.0);
        let mut saved = Fields::new();
        transform.saved(&mut saved);
        assert!(saved.get("velocity").is_none());
        let mut back = Transform::default();
        assert_eq!(back.load(&saved), Ok(()));
        assert!(back.is_still(), "a character comes back standing still");
        assert_eq!(back.position, Vector3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn a_character_takes_up_a_capsule_one_wide_and_two_tall() {
        assert_eq!(Collider::CHARACTER.reach(), 0.5);
        assert_eq!(Collider::CHARACTER.height(), 2.0);
        let crate_box = Collider::Cube { size: Vector3::new(2.0, 1.0, 3.0) };
        assert_eq!(crate_box.reach(), 1.5);
        assert_eq!(crate_box.height(), 1.0);
    }

    #[test]
    fn the_fallback_shape_is_a_cube() {
        assert_eq!(PrimitiveShape::default(), PrimitiveShape::Cube);
    }

    #[test]
    fn a_pool_reads_back_with_its_current_under_its_max() {
        let mut saved = Fields::new();
        Pool { current: 150, max: 200 }.saved(&mut saved);
        let mut pool = Pool::full(10);
        assert_eq!(pool.load(&saved), Ok(()));
        assert_eq!(pool, Pool { current: 150, max: 200 });

        let mut over = Fields::new();
        over.put_number("current", 500.0);
        over.put_number("max", 100.0);
        assert_eq!(pool.load(&over), Ok(()));
        assert_eq!(pool, Pool { current: 100, max: 100 });
    }

    #[test]
    fn titles_read_back_with_the_one_picked() {
        let mut titles = Titles::one("the plucky");
        titles.add("the Cursed");
        titles.pick(1);
        let mut saved = Fields::new();
        titles.saved(&mut saved);

        let mut read = Titles::default();
        assert_eq!(read.load(&saved), Ok(()));
        assert_eq!(read, titles);

        let mut none = Fields::new();
        Titles::default().saved(&mut none);
        assert_eq!(read.load(&none), Ok(()));
        assert_eq!(read, Titles::default());

        let mut wrong = Fields::new();
        wrong.put_texts("list", &["one".to_string()]);
        wrong.put_number("picked", 3.0);
        assert!(read.load(&wrong).is_err(), "there's no third title to pick");
    }

    #[test]
    fn a_transform_reads_back_whole() {
        let mut moved = Transform::at(Vector3::new(1.0, 2.0, 3.0));
        moved.rotation = Vector3::new(0.0, 45.0, 0.0);
        moved.scale = Vector3::new(2.0, 2.0, 2.0);
        let mut saved = Fields::new();
        moved.saved(&mut saved);

        let mut read = Transform::default();
        assert_eq!(read.load(&saved), Ok(()));
        assert_eq!(read, moved);
    }

    #[test]
    fn every_kind_has_a_name_that_leads_back_to_it() {
        for kind in Kind::ALL {
            assert_eq!(Kind::from_name(kind.name()), Some(kind));
            assert_eq!(Component::default_of(kind).kind(), kind);
        }
        assert_eq!(Kind::from_name("position"), None, "names are exact");
    }
}
