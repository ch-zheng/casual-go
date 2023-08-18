use crate::model::{Game, Stone, Turn, Scoring};
use tokio::sync::{mpsc, oneshot};
use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex}
};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Frame {
    //Game settings
    pub board_size: usize,
    pub handicap: u32,
    //Board state
    pub board: Vec<u8>,
    pub moves: Vec<bool>,
    pub turn: String,
    //Score
    pub black_score: u32,
    pub white_score: u32
}

impl Frame {
    fn from_game(game: &Game) -> Frame {
        Frame {
            board_size: game.board_size,
            handicap: game.handicap,
            board: game.board.iter().map(|x| match x {
                Stone::Empty => 0,
                Stone::Black => 1,
                Stone::White => 2
            }).collect(),
            moves: game.valid_moves.clone(),
            turn: match game.turn {
                Turn::Handicap => "handicap".to_string(),
                Turn::Black => "black".to_string(),
                Turn::White => "white".to_string(),
                Turn::End => "end".to_string()
            },
            black_score: game.black_score,
            white_score: game.white_score
        }
    }
}

pub enum Message {
    //Lobby
    Join(Stone, mpsc::UnboundedSender<serde_json::Value>),
    Occupancy(Stone),
    Leave(Stone),
    //Game
    Handicap(Stone, Vec<usize>),
    Play(Stone, usize),
    Pass(Stone),
    Resign(Stone)
}

pub enum Response {
    Success,
    Occupancy(bool),
    BoardSize(usize),
    Frame(serde_json::Value)
}

pub enum Error {
    Lobby,
    Game
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Lobby => writeln!(f, "Lobby error"),
            Self::Game => writeln!(f, "Game error"),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Lobby => writeln!(f, "Lobby error"),
            Self::Game => writeln!(f, "Game error"),
        }
    }
}

impl std::error::Error for Error {}

pub type Request = (
    Message, 
    oneshot::Sender<Result<Response, Error>>
);

pub type Sessions = HashMap<usize, mpsc::UnboundedSender<Request>>;

fn broadcast(
    game: &Game,
    clients: &[Option<mpsc::UnboundedSender<serde_json::Value>>]
) {
    for client in clients {
        if let Some(client) = client {
            let frame = serde_json::to_value(Frame::from_game(&game)).unwrap();
            let _ = client.send(frame);
        }
    }
}

pub async fn session(
    id: usize,
    sessions: Arc<Mutex<Sessions>>,
    board_size: usize,
    handicap: u32,
    scoring: Scoring,
    mut receiver: mpsc::UnboundedReceiver<Request>
) {
    //Create state
    let mut clients: [Option<mpsc::UnboundedSender<serde_json::Value>>; 2] = [None, None];
    let mut game = Game::new(board_size, handicap, scoring);
    //Listen for requests
    while let Some((message, reply)) = receiver.recv().await {
        let response = match message {
            //Lobby actions
            Message::Join(stone, client) => 'a: {
                let index = match stone {
                    Stone::Black => 0,
                    Stone::White => 1,
                    _ => break 'a Err(Error::Lobby)
                };
                if clients[index].is_none() {
                    let frame = serde_json::to_value(Frame::from_game(&game)).unwrap();
                    if client.send(frame).is_ok() {
                        clients[index] = Some(client);
                        break 'a Ok(Response::Success)
                    }
                }
                Err(Error::Lobby)
            },
            Message::Occupancy(stone) => 'a: {
                let index = match stone {
                    Stone::Black => 0,
                    Stone::White => 1,
                    _ => break 'a Err(Error::Lobby)
                };
                if clients[index].is_some() {
                    Ok(Response::Occupancy(true))
                } else {
                    Ok(Response::Occupancy(false))
                }
            },
            Message::Leave(stone) => match stone {
                Stone::Black => {
                    clients[0] = None;
                    Ok(Response::Success)
                },
                Stone::White => {
                    clients[1] = None;
                    Ok(Response::Success)
                },
                _ => Err(Error::Lobby)
            }
            //Game actions
            Message::Handicap(stone, positions) => if stone == Stone::Black
                && game.play_handicap(&positions).is_ok() {
                broadcast(&game, &clients);
                Ok(Response::Success)
            } else {
                Err(Error::Game)
            }
            Message::Play(stone, position) => if game.play(stone, position).is_ok() {
                broadcast(&game, &clients);
                Ok(Response::Success)
            } else {
                Err(Error::Game)
            },
            Message::Pass(stone) => if game.pass(stone).is_ok() {
                broadcast(&game, &clients);
                Ok(Response::Success)
            } else {
                Err(Error::Game)
            },
            Message::Resign(stone) => if game.resign(stone).is_ok() {
                broadcast(&game, &clients);
                Ok(Response::Success)
            } else {
                Err(Error::Game)
            },
        };
        let _ = reply.send(response);
        if game.turn == Turn::End {
            break
        }
    }
    //Remove session
    let mut sessions = sessions.lock().unwrap();
    sessions.remove(&id);
}
