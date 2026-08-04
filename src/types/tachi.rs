use num_enum::FromPrimitive;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Import {
    pub meta: ImportMeta,
    pub scores: Vec<ImportScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportMeta {
    pub game: String,
    #[serde(rename = "playtype")]
    pub play_type: Playtype,
    pub service: String,
}

impl Default for ImportMeta {
    fn default() -> Self {
        Self {
            game: "ddr".to_string(),
            play_type: Playtype::SP,
            service: "Takure".to_string(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Playtype {
    SP,
    DP,
}

impl From<u8> for Playtype {
    fn from(value: u8) -> Self {
        match value {
            0 | 2 => Playtype::SP,
            1 => Playtype::DP,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportScore {
    pub score: u32,
    pub lamp: TachiLamp,
    #[serde(rename = "matchType")]
    pub match_type: String,
    pub identifier: String,
    pub difficulty: Difficulty,
    #[serde(rename = "timeAchieved")]
    pub time_achieved: u128,
    pub judgements: Judgements,
    pub optional: Optional,
}

#[derive(Debug, Clone, Eq, PartialEq, FromPrimitive, Serialize, Deserialize)]
#[repr(u8)]
pub enum TachiLamp {
    #[num_enum(default)]
    #[serde(rename = "FAILED")]
    Failed = 1,
    #[serde(rename = "ASSIST")]
    Assist = 2,
    #[serde(rename = "CLEAR")]
    Clear = 3,
    #[serde(rename = "LIFE4")]
    Life4 = 6,
    #[serde(rename = "FULL COMBO")]
    FullCombo = 7,
    #[serde(rename = "GREAT FULL COMBO")]
    GreatFullCombo = 8,
    #[serde(rename = "PERFECT FULL COMBO")]
    PerfectFullCombo = 9,
    #[serde(rename = "MARVELOUS FULL COMBO")]
    MarvelousFullCombo = 10,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Difficulty {
    #[serde(rename = "BEGINNER")]
    Beginner,
    #[serde(rename = "BASIC")]
    Basic,
    #[serde(rename = "DIFFICULT")]
    Difficult,
    #[serde(rename = "EXPERT")]
    Expert,
    #[serde(rename = "CHALLENGE")]
    Challenge,
}

impl From<u8> for Difficulty {
    fn from(value: u8) -> Self {
        match value {
            0 => Difficulty::Beginner,
            1 | 5 => Difficulty::Basic,
            2 | 6 => Difficulty::Difficult,
            3 | 7 => Difficulty::Expert,
            4 | 8 => Difficulty::Challenge,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Judgements {
    #[serde(rename = "MARVELOUS")]
    pub marvelous: u32,
    #[serde(rename = "PERFECT")]
    pub perfect: u32,
    #[serde(rename = "GREAT")]
    pub great: u32,
    #[serde(rename = "GOOD")]
    pub good: u32,
    #[serde(rename = "MISS")]
    pub miss: u32,
    #[serde(rename = "OK")]
    pub ok: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HitMeta {
    pub fast: u32,
    pub slow: u32,
    #[serde(rename = "maxCombo")]
    pub max_combo: u32,
    #[serde(rename = "exScore")]
    pub ex_score: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Optional {
    #[serde(flatten)]
    pub hit_meta: HitMeta,
    #[serde(skip_serializing_if = "Flare::is_none")]
    pub flare: Flare,
}

#[derive(Debug, Clone, Eq, PartialEq, FromPrimitive, Serialize, Deserialize)]
#[repr(u8)]
pub enum Flare {
    #[num_enum(default)]
    None = 0,
    I = 1,
    II = 2,
    III = 3,
    IV = 4,
    V = 5,
    VI = 6,
    VII = 7,
    VIII = 8,
    IX = 9,
    EX = 10,
}

impl Default for Flare {
    fn default() -> Self {
        Flare::None
    }
}

impl Flare {
    pub fn is_none(flare: &Flare) -> bool {
        *flare == Flare::None
    }
}