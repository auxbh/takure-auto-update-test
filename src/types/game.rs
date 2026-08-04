use serde::{Deserialize, Serialize};

// DDR A3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property2 {
    pub call: CallStruct2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallStruct2 {
    pub playerdata_2: PlayerData2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerData2 {
    pub data: PlayerData2Data,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerData2Data {
    pub mode: String,
    #[serde(rename = "refid")]
    pub ref_id: String,
    #[serde(default)]
    pub isgameover: bool,
    #[serde(default)]
    pub note: Vec<Note>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub stagenum: u8,
    pub mcode: u32,
    pub notetype: u8,
    pub clearkind: u8,
    pub score: u32,
    #[serde(rename = "exscore")]
    pub ex_score: u32,
    pub maxcombo: u32,
    pub fastcount: u32,
    pub slowcount: u32,
    pub judge_marvelous: u32,
    pub judge_perfect: u32,
    pub judge_great: u32,
    pub judge_good: u32,
    pub judge_miss: u32,
    pub judge_ok: u32,
    pub playstyle: u8,
}

// DDR WORLD
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property3 {
    pub call: CallStruct3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallStruct3 {
    pub playdata_3: PlayData3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayData3 {
    pub data: PlayData3Data,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayData3Data {
    #[serde(rename = "refid")]
    pub ref_id: String,
    pub savekind: u8,
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Result {
    pub stagenum: u8,
    pub mcode: u32,
    pub difficulty: u8,
    pub clearkind: u8,
    pub score: u32,
    #[serde(rename = "exscore")]
    pub ex_score: u32,
    pub maxcombo: u32,
    pub fastcount: u32,
    pub slowcount: u32,
    pub judge_marv: u32,
    pub judge_perf: u32,
    pub judge_great: u32,
    pub judge_good: u32,
    pub judge_miss: u32,
    pub judge_ok: u32,
    pub style: u8,
    pub flare_force: i8,
}