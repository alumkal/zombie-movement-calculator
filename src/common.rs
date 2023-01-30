pub use std::cmp::{min, max};
pub use std::collections::{HashMap, HashSet};
pub use std::str::FromStr;
pub use strum::IntoEnumIterator;

#[derive(strum::EnumString, strum::EnumIter, PartialEq, Eq, Hash, Copy, Clone, Debug)]
#[strum(serialize_all = "PascalCase", ascii_case_insensitive)]
pub enum ZombieType {
    Regular,
    DCFast,
    DCSlow,
    Flag,
    Conehead,
    PoleVaulting,
    Buckethead,
    Newspaper,
    ScreenDoor,
    Football,
    Dancing,
    Snorkel,
    Zomboni,
    DolphinRider,
    #[strum(serialize = "JackInTheBox", serialize = "Jack-in-the-box")]
    JackInTheBox,
    Balloon,
    Digger,
    Pogo,
    Ladder,
    Catapult,
    Gargantuar,
    #[strum(serialize = "Giga", serialize = "GigaGargantuar")]
    GigaGargantuar,
}

pub type Num = num_rational::Rational64;

pub enum MovementType {
    Constant,
    Animation(Vec<Num>),
    Regular(Vec<Num>, Vec<Num>),
    DanceCheat,
    Dancing(Vec<Num>),
    Zomboni
}

pub struct ZombieData {
    pub spawn: (i32, i32),
    pub spawn_hugewave: (i32, i32),
    pub movement_type: MovementType,
    pub speed: (Num, Num),
    pub freeze_immune: bool,
    pub chill_immune: bool,
    pub def_x: (i32, i32),
    pub def_y: (i32, i32),
    pub atk: (i32, i32),
    pub hp: i32,
    pub summon_weight_normal: u32,
    pub summon_weight_hugewave: u32,
    pub if_generate_in: (bool, bool),
    pub if_generate_in_wave1to5: (bool, bool),
}
