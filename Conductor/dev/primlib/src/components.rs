//! File:       Opus/Conductor/dev/primlib/src/components.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The components: plain data, one struct per kind.  The first nine are
//! the ones in my sample NPC.  `Kind` names a kind of component (for a
//! template's list, a script, or the log), and `Component` is one of them
//! with its value, which is how a template or blueprint holds them.
//!
//! Adding a kind: its struct here, a line in `Kind` (and `Kind::ALL` and
//! `name()`), a line in `Component` (and `kind()`), then its store and
//! getters in `world.rs`.

/// Where an object is.  Y is up, as in Unity.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Position {
    pub fn new(x: f32, y: f32, z: f32) -> Position {
        Position { x, y, z }
    }
}

/// Which way an object faces: three angles in degrees, the way Unity's
/// inspector shows a rotation.  Ensemble turns them into Unity's own
/// rotation with `Quaternion.Euler(x, y, z)`.  The server doesn't do any
/// rotation math yet; if it ever has to, this is the one place that
/// changes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rotation {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Rotation {
    pub fn new(x: f32, y: f32, z: f32) -> Rotation {
        Rotation { x, y, z }
    }
}

/// How big an object is next to its model.  1, 1, 1 is as it was made.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scale {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Scale {
    pub fn new(x: f32, y: f32, z: f32) -> Scale {
        Scale { x, y, z }
    }
}

// Rust note: `Default` is written out by hand here, because the one Rust
// would make for us starts every number at 0, and a scale of 0 is an
// object nobody can see.
impl Default for Scale {
    fn default() -> Scale {
        Scale { x: 1.0, y: 1.0, z: 1.0 }
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
}

/// A kind of component, by name, without its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Position,
    Rotation,
    Scale,
    ShortName,
    LongName,
    Titles,
    Health,
    Endurance,
    Mana,
}

impl Kind {
    /// Every kind, in the order they're listed here.
    pub const ALL: [Kind; 9] = [
        Kind::Position,
        Kind::Rotation,
        Kind::Scale,
        Kind::ShortName,
        Kind::LongName,
        Kind::Titles,
        Kind::Health,
        Kind::Endurance,
        Kind::Mana,
    ];

    /// The kind's name, as a script or the log would write it.
    pub fn name(&self) -> &'static str {
        match self {
            Kind::Position => "Position",
            Kind::Rotation => "Rotation",
            Kind::Scale => "Scale",
            Kind::ShortName => "ShortName",
            Kind::LongName => "LongName",
            Kind::Titles => "Titles",
            Kind::Health => "Health",
            Kind::Endurance => "Endurance",
            Kind::Mana => "Mana",
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
    Position(Position),
    Rotation(Rotation),
    Scale(Scale),
    ShortName(ShortName),
    LongName(LongName),
    Titles(Titles),
    Health(Pool),
    Endurance(Pool),
    Mana(Pool),
}

impl Component {
    /// Which kind this is.
    pub fn kind(&self) -> Kind {
        match self {
            Component::Position(_) => Kind::Position,
            Component::Rotation(_) => Kind::Rotation,
            Component::Scale(_) => Kind::Scale,
            Component::ShortName(_) => Kind::ShortName,
            Component::LongName(_) => Kind::LongName,
            Component::Titles(_) => Kind::Titles,
            Component::Health(_) => Kind::Health,
            Component::Endurance(_) => Kind::Endurance,
            Component::Mana(_) => Kind::Mana,
        }
    }

    /// The kind with its default value: 0, 0, 0 for a position or a
    /// rotation, 1, 1, 1 for a scale, empty names and titles, and pools of
    /// 0.  A template sets its own where these won't do.
    pub fn default_of(kind: Kind) -> Component {
        match kind {
            Kind::Position => Component::Position(Position::default()),
            Kind::Rotation => Component::Rotation(Rotation::default()),
            Kind::Scale => Component::Scale(Scale::default()),
            Kind::ShortName => Component::ShortName(ShortName::default()),
            Kind::LongName => Component::LongName(LongName::default()),
            Kind::Titles => Component::Titles(Titles::default()),
            Kind::Health => Component::Health(Pool::default()),
            Kind::Endurance => Component::Endurance(Pool::default()),
            Kind::Mana => Component::Mana(Pool::default()),
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
    fn scale_starts_at_one() {
        assert_eq!(Scale::default(), Scale::new(1.0, 1.0, 1.0));
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
