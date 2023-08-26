use crate::{
    model::{Stone, Game},
    session::{self, Packet, Message}
};
use axum::{
    response::{self, Response, sse},
    extract::{
        Form,
        State,
        Path,
        ws
    }
};
use http::status::StatusCode;
use tokio::sync::oneshot;
use tokio_stream::wrappers::{
    BroadcastStream,
    errors::BroadcastStreamRecvError
};
use futures::{
    sink::SinkExt,
    stream::{Stream, StreamExt}
};
use handlebars::Handlebars;
use serde::{Serialize, Deserialize};
use std::{
    sync::{Arc, Mutex},
    time::Duration
};

#[derive(Clone)]
pub struct AppState {
    pub templates: Handlebars<'static>,
    pub sessions: Arc<Mutex<session::Sessions>>
}

#[derive(Deserialize)]
pub struct CreateGameForm {
    board_size: usize,
    komi: u32,
    handicap: u32,
    fixed_time: u64,
    added_time: u64,
    black_player: String,
    white_player: String
}

pub async fn create_session(
    State(state): State<AppState>,
    Form(form): Form<CreateGameForm>
) -> response::Result<(StatusCode, response::Redirect), StatusCode> {
    //Create game
    let bots = [
        form.black_player == "bot",
        form.white_player == "bot"
    ];
    if form.fixed_time <= 3600 && form.added_time <= 60 && (!bots[0] || !bots[1]) {
        if let Ok(game) = Game::new(form.board_size, form.komi, form.handicap) {
            let mut sessions = state.sessions.lock().unwrap();
            //Generate session ID
            let id: usize = loop {
                let id: usize = rand::random();
                if !sessions.contains_key(&id) {
                    break id;
                }
            };
            //Spawn task
            let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<Message>();
            let (broadcast, _) = tokio::sync::broadcast::channel::<Packet>(8);
            tokio::spawn(session::session(
                id,
                state.sessions.clone(),
                sender.clone(),
                receiver,
                broadcast.clone(),
                game,
                bots,
                Duration::from_secs(form.fixed_time),
                Duration::from_secs(form.added_time)
            ));
            //Register session 
            sessions.insert(id, (sender, broadcast));
            Ok((StatusCode::SEE_OTHER, response::Redirect::to(&format!("/play/{}", id))))
        } else {
            Err(StatusCode::BAD_REQUEST)
        }
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

#[derive(Serialize, Deserialize)]
struct GameTemplateData {
    id: usize,
    stone: String,
    board_size: u32,
    komi: u32,
    handicap: u32,
    fixed_time: u32,
    added_time: u32
}

pub async fn get_session(
    Path(game): Path<usize>,
    State(state): State<AppState>
) -> Result<response::Html<String>, StatusCode> {
    let session = {
        let sessions = state.sessions.lock().unwrap();
        let session = sessions.get(&game);
        if let Some(session) = session {
            Some(session.clone())
        } else {
            None
        }
    };
    if let Some((session, _)) = session {
        let (sender, receiver) = oneshot::channel();
        let message = Message::Query(sender);
        if session.send(message).is_ok() {
            if let Ok(settings) = receiver.await {
                let data = GameTemplateData {
                    id: game,
                    stone: "empty".to_string(),
                    board_size: settings.board_size,
                    komi: settings.komi,
                    handicap: settings.handicap,
                    fixed_time: settings.fixed_time,
                    added_time: settings.added_time
                };
                let body = state.templates.render("lobby", &data).unwrap();
                Ok(response::Html(body))
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        } else {
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn join_session(
    Path((game, side)): Path<(usize, String)>,
    State(state): State<AppState>
) -> Result<response::Html<String>, StatusCode> {
    let session = {
        let sessions = state.sessions.lock().unwrap();
        let session = sessions.get(&game);
        if let Some(session) = session {
            Some(session.clone())
        } else {
            None
        }
    };
    if let Some((session, _)) = session {
        let stone = match side.as_str() {
            "black" => Stone::Black,
            "white" => Stone::White,
            _ => return Err(StatusCode::NOT_FOUND)
        };
        let (sender, receiver) = oneshot::channel();
        let message = Message::Packet(sender);
        if session.send(message).is_ok() {
            if let Ok(packet) = receiver.await {
                //Check occupancy
                let occupied = match stone {
                    Stone::Black => packet.black_occupied,
                    Stone::White => packet.white_occupied,
                    _ => true
                };
                if !occupied {
                    //Get game settings
                    let (sender, receiver) = oneshot::channel();
                    let message = Message::Query(sender);
                    if session.send(message).is_ok() {
                        if let Ok(settings) = receiver.await {
                            let data = GameTemplateData {
                                id: game,
                                stone: side,
                                board_size: settings.board_size,
                                komi: settings.komi,
                                handicap: settings.handicap,
                                fixed_time: settings.fixed_time,
                                added_time: settings.added_time
                            };
                            let body = state.templates.render("game", &data).unwrap();
                            Ok(response::Html(body))
                        } else {
                            Err(StatusCode::INTERNAL_SERVER_ERROR)
                        }
                    } else {
                        Err(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                } else {
                    Err(StatusCode::BAD_REQUEST)
                }
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        } else {
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn spectate(
    Path(id): Path<usize>,
    State(state): State<AppState>
) -> Result<
    sse::Sse<impl Stream<Item = Result<sse::Event, Box<BroadcastStreamRecvError>>>>,
    StatusCode
> {
    let session = {
        let sessions = state.sessions.lock().unwrap();
        let session = sessions.get(&id);
        if let Some(session) = session {
            Some(session.clone())
        } else {
            None
        }
    };
    if let Some((session, broadcast)) = session {
        let receiver = broadcast.subscribe();
        let message = Message::Ping;
        if session.send(message).is_err() {
            return Err(StatusCode::NOT_FOUND);
        }
        let stream = BroadcastStream::new(receiver).map(
            |item| match item {
                Ok(item) => {
                    let event = sse::Event::default().json_data(item).unwrap();
                    Ok(event)
                },
                Err(item) => Err(Box::new(item))
            }
        );
        Ok(sse::Sse::new(stream))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/*
    Client-side messages:
    Handicap: {
        action: 'handicap',
        positions: [1,2,3,...]
    }
    Play: {
        action: 'play',
        position: 0
    }
    Pass: {action: 'pass'}
    Resign: {action: 'resign'}
*/

fn parse_message(
    stone: Stone,
    message: &str
) -> Option<session::Message> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(message) {
        let value = value.as_object()?;
        let action = value.get("action")?.as_str()?;
        match action {
            "handicap" => {
                let mut positions = Vec::<usize>::new();
                let array = value.get("positions")?.as_array()?;
                for item in array {
                    positions.push(item.as_u64()? as usize);
                }
                Some(session::Message::Handicap(stone, positions))
            },
            "play" => {
                let position = value.get("position")?.as_u64()? as usize;
                Some(session::Message::Play(stone, position))
            },
            "pass" => Some(session::Message::Pass(stone)),
            "resign" => Some(session::Message::Resign(stone)),
            _ => None
        }
    } else {
        None
    }
}

pub async fn connection(
    socket: ws::WebSocketUpgrade,
    Path((game, stone)): Path<(usize, String)>,
    State(state): State<AppState>,
) -> Response {
    socket.on_upgrade(move |socket| async move {
        let (mut socket_sender, mut socket_receiver) = socket.split();
        let session = {
            let sessions = state.sessions.lock().unwrap();
            let session = sessions.get(&game);
            if let Some(session) = session {
                Some(session.clone())
            } else {
                None
            }
        };
        if let Some((session, broadcast)) = session {
            let stone = match stone.as_str() {
                "black" => Stone::Black,
                "white" => Stone::White,
                _ => return
            };
            //Broadcast listener
            let mut broadcast_receiver = broadcast.subscribe();
            let handle = tokio::spawn(async move {
                while let Ok(packet) = broadcast_receiver.recv().await {
                    let message = ws::Message::Text(
                        serde_json::to_value(packet).unwrap().to_string()
                    );
                    if socket_sender.send(message).await.is_err() {
                        break;
                    }
                }
            });
            //WebSocket listener
            //Attempt to join session
            let (once_sender, once_receiver) = oneshot::channel();
            let message = Message::Join(stone, once_sender);
            if session.send(message).is_ok() {
                if let Ok(true) = once_receiver.await {
                    //Listen for messages
                    while let Some(Ok(ws::Message::Text(message))) = socket_receiver.next().await {
                        if let Some(message) = parse_message(stone, &message) {
                            if session.send(message).is_err() {
                                break
                            }
                        }
                    }
                }
            }
            //Leave session 
            let _ = session.send(Message::Leave(stone));
            handle.abort();
        }
    })
}
