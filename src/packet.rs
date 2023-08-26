pub struct Initial {
    //Game settings
    pub board_size: u32,
    pub komi: u32,
    pub handicap: u32,
    pub fixed_time: u32,
    pub added_time: u32,
}

pub struct Handicap {
    pub count: u32,
    pub black_time: u32,
    pub white_time: u32
}

pub struct Play {
    pub color: String,
    pub board: Vec<u8>,
    pub moves: Vec<bool>
}

pub struct Occupancy {
    pub black: bool,
    pub white: bool,
    pub spectators: u32
}

pub struct End {
    pub reason: String,
    pub black_score: u32,
    pub white_score: u32,
    pub black_time: u32,
    pub white_time: u32
}

pub enum Packet {
    Initial(Initial),
    Handicap(Handicap),
    Play(Play),
    Occupancy(Occupancy),
    End(End),
    Time(Time)
}
