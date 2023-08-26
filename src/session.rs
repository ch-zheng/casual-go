use crate::{
    model::{Game, Stone, Turn, Settings},
    timer::Timer,
    engine
};
use tokio::sync::{mpsc, broadcast, oneshot};
use serde::{Serialize, Deserialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Packet {
    //Board state
    pub board: Vec<u8>,
    pub moves: Vec<bool>,
    pub turn: String,
    //Occupancy
    pub black_occupied: bool,
    pub white_occupied: bool,
    //Score
    pub black_score: u32,
    pub white_score: u32,
    //Time control
    pub black_time: u64,
    pub white_time: u64
}

impl Packet {
    fn new(
        game: &Game,
        timers: &[Timer],
        occupancy: &[bool]
    ) -> Packet {
        Packet {
            //Board state
            board: game.board.iter().map(|x| match x {
                Stone::Empty => 0,
                Stone::Black => 1,
                Stone::White => 2
            }).collect(),
            moves: game.valid_moves.clone(),
            turn: if timers[2].running() {
                "wait".into()
            } else {
                match game.turn {
                    Turn::Handicap => "handicap".into(),
                    Turn::Black => "black".into(),
                    Turn::White => "white".into(),
                    Turn::End => "end".into()
                }
            },
            //Occupancy
            black_occupied: occupancy[0],
            white_occupied: occupancy[1],
            //Score
            black_score: game.black_score,
            white_score: game.white_score,
            //Time control
            black_time: if timers[2].running() {
                timers[2].time().as_secs()
            } else {
                timers[0].time().as_secs()
            },
            white_time: if timers[2].running() {
                timers[2].time().as_secs()
            } else {
                timers[1].time().as_secs()
            }
        }
    }
}

pub enum Message {
    //Lobby
    Join(Stone, oneshot::Sender<bool>),
    Leave(Stone),
    Expire,
    //Game
    Handicap(Stone, Vec<usize>),
    Play(Stone, usize),
    Pass(Stone),
    Resign(Stone),
    //Utility
    Ping,
    Packet(oneshot::Sender<Packet>),
    Query(oneshot::Sender<Settings>)
}

pub type Sessions = HashMap<
    usize,
    (mpsc::UnboundedSender<Message>, broadcast::Sender<Packet>)
>;

pub async fn session(
    id: usize,
    sessions: Arc<Mutex<Sessions>>,
    sender: mpsc::UnboundedSender<Message>,
    mut receiver: mpsc::UnboundedReceiver<Message>,
    broadcast: broadcast::Sender<Packet>,
    mut game: Game,
    bots: [bool; 2],
    fixed_time: Duration,
    added_time: Duration
) {
    let mut players = bots;
    let session_timeout = Duration::from_secs(5 * 60);
    let mut timers = [
        Timer::new(fixed_time, false), //Black
        Timer::new(fixed_time, false), //White
        Timer::new(session_timeout, true) //Session
    ];
    let sender_clone = sender.clone();
    let mut handle = tokio::spawn(async move {
        tokio::time::sleep(session_timeout).await;
        let _ = sender_clone.send(Message::Expire);
    });
    //Engine
    let mut engine = None;
    if bots[0] || bots[1] {
        let stone = if bots[0] {Stone::Black} else {Stone::White};
        engine = Some(engine::engine(stone, game.board_size as u32, game.komi, sender.clone()));
    }
    //Listen for requests
    while let Some(message) = receiver.recv().await {
        match message {
            //Lobby
            Message::Join(stone, response) => {
                //Attempt to add client
                let success = match stone {
                    Stone::Black => if !players[0] {
                        players[0] = true;
                        true
                    } else {
                        false
                    },
                    Stone::White => if !players[1] {
                        players[1] = true;
                        true
                    } else {
                        false
                    },
                    Stone::Empty => false
                };
                //Both players joined => Start game
                if success {
                    if players[0] && players[1] && timers[2].running() {
                        //Timers
                        handle.abort();
                        timers[2].pause();
                        let duration = timers[0].time();
                        timers[0].resume();
                        let sender = sender.clone();
                        handle = tokio::spawn(async move {
                            tokio::time::sleep(duration).await;
                            let _ = sender.send(Message::Resign(Stone::Black));
                        });
                        //Engine
                        if bots[0] {
                            let engine = engine.clone().expect("No engine");
                            let _ = engine.send(engine::Message::Handicap(game.handicap));
                        }
                    }
                    let _ = broadcast.send(Packet::new(&game, &mut timers, &players));
                }
                let _ = response.send(success);
            },
            Message::Leave(stone) => {
                match stone {
                    Stone::Black => players[0] = false,
                    Stone::White => players[1] = false,
                    Stone::Empty => ()
                }
                let _ = broadcast.send(Packet::new(&game, &mut timers, &players));
            },
            Message::Expire => {
                game.turn = Turn::End;
                let _ = broadcast.send(Packet::new(&game, &mut timers, &players));
            },
            //Game
            Message::Handicap(stone, positions) => if game.play_handicap(stone, &positions).is_ok() {
                //Timer
                timers[0].pause();
                timers[0].add(added_time);
                handle.abort();
                let duration = timers[1].time();
                timers[1].resume();
                let sender = sender.clone();
                handle = tokio::spawn(async move {
                    tokio::time::sleep(duration).await;
                    let _ = sender.send(Message::Resign(Stone::White));
                });
                //Broadcast
                let _ = broadcast.send(Packet::new(&game, &mut timers, &players));
                //Engine
                if bots[1] {
                    let engine = engine.clone().unwrap();
                    for position in positions {
                        let _ = engine.send(engine::Message::Play(stone, position as u32));
                    }
                    let _ = engine.send(engine::Message::Genmove);
                }
            },
            Message::Play(stone, position) => if game.play(stone, position).is_ok() {
                //Timer
                let (next_stone, duration) = match stone {
                    Stone::Black => {
                        timers[0].pause();
                        timers[0].add(added_time);
                        timers[1].resume();
                        (Stone::White, timers[1].time())
                    },
                    Stone::White => {
                        timers[1].pause();
                        timers[1].add(added_time);
                        timers[0].resume();
                        (Stone::Black, timers[0].time())
                    },
                    Stone::Empty => (Stone::Empty, Duration::from_secs(0))
                };
                handle.abort();
                let sender = sender.clone();
                handle = tokio::spawn(async move {
                    tokio::time::sleep(duration).await;
                    let _ = sender.send(Message::Resign(next_stone));
                });
                //Broadcast
                let _ = broadcast.send(Packet::new(&game, &timers, &players));
                //Engine
                if next_stone == Stone::Black && bots[0]
                || next_stone == Stone::White && bots[1] {
                    let engine = engine.clone().expect("No engine");
                    let _ = engine.send(engine::Message::Play(stone, position as u32));
                    let _ = engine.send(engine::Message::Genmove);
                }
            },
            Message::Pass(stone) => if game.pass(stone).is_ok() {
                //Timer
                let (next_stone, duration) = match stone {
                    Stone::Black => {
                        timers[0].pause();
                        timers[0].add(added_time);
                        timers[1].resume();
                        (Stone::White, timers[1].time())
                    },
                    Stone::White => {
                        timers[1].pause();
                        timers[1].add(added_time);
                        timers[0].resume();
                        (Stone::Black, timers[0].time())
                    },
                    Stone::Empty => (Stone::Empty, Duration::from_secs(0))
                };
                handle.abort();
                let sender = sender.clone();
                handle = tokio::spawn(async move {
                    tokio::time::sleep(duration).await;
                    let _ = sender.send(Message::Resign(next_stone));
                });
                //Broadcast
                let _ = broadcast.send(Packet::new(&game, &timers, &players));
                //Engine
                if next_stone == Stone::Black && bots[0]
                || next_stone == Stone::White && bots[1] {
                    let engine = engine.clone().expect("No engine");
                    let _ = engine.send(engine::Message::Genmove);
                }
            },
            Message::Resign(stone) => if game.resign(stone).is_ok() {
                let _ = broadcast.send(Packet::new(&game, &timers, &players));
            },
            //Utility
            Message::Ping => {
                let _ = broadcast.send(Packet::new(&game, &timers, &players));
            },
            Message::Packet(sender) => {
                let _ = sender.send(Packet::new(&game, &timers, &players));
            },
            Message::Query(sender) => {
                let _ = sender.send(Settings {
                    board_size: game.board_size as u32,
                    komi: game.komi,
                    handicap: game.handicap,
                    fixed_time: fixed_time.as_secs() as u32,
                    added_time: added_time.as_secs() as u32
                });
            }
        }
        if game.turn == Turn::End {
            break
        }
    }
    //Remove session
    if let Some(engine) = engine {
        let _ = engine.send(engine::Message::Quit);
    }
    handle.abort();
    let mut sessions = sessions.lock().unwrap();
    sessions.remove(&id);
}
