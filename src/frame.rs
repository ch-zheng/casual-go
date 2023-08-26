use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Init {
    pub board_size: usize
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Handicap {
    pub count: u32
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Turn {
    pub turn: String,
    pub board: Vec<u8>,
    pub moves: Vec<bool>,
    pub black_time: u64,
    pub white_time: u64
}

#[derive(Clone, Serialize, Deserialize)]
pub struct End {
    pub black_score: u32,
    pub white_score: u32
}

#[derive(Clone)]
pub enum Frame {
    Init(Init),
    Turn(Turn)
}
