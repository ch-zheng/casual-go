use crate::{
    model::{Scoring, Stone},
    session
};
use axum::{
    response::{self, Response},
    extract::{
        Form,
        State,
        Path,
        ws::{WebSocketUpgrade, Message}
    }
};
use http::status::StatusCode;
use serde::Deserialize;
use tokio::sync::{mpsc, oneshot};
use futures::{
    sink::SinkExt,
    stream::StreamExt
};
use handlebars::Handlebars;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex}
};

#[derive(Clone)]
pub struct AppState {
    pub templates: Handlebars<'static>,
    pub sessions: Arc<Mutex<session::Sessions>>
}

#[derive(Deserialize)]
pub struct CreateGameForm {
    board_size: usize,
    handicap: u32,
    scoring: String
}

impl CreateGameForm {
    fn valid(&self) -> bool {
        self.board_size >= 5
            && self.board_size <= 19
            && self.board_size & 1 == 1
            && self.handicap < 9
            && match self.scoring.as_str() {
                "area" => true,
                "territory" => true,
                "stones" => true,
                _ => false
            }
    }
}

pub async fn create_session(
    State(state): State<AppState>,
    Form(form): Form<CreateGameForm>
) -> response::Result<(StatusCode, response::Redirect), StatusCode> {
    //Validate form
    if !form.valid() {
        return Err(StatusCode::BAD_REQUEST)
    }
    let mut sessions = state.sessions.lock().unwrap();
    //Generate session ID
    let id: usize = loop {
        let id: usize = rand::random();
        if !sessions.contains_key(&id) {
            break id;
        }
    };
    //Spawn task
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<session::Request>();
    let scoring = match form.scoring.as_str() {
        "area" => Scoring::Area,
        "territory" => Scoring::Territory,
        "stones" => Scoring::Stones,
        _ => return Err(StatusCode::BAD_REQUEST)
    };
    tokio::spawn(session::session(
        id,
        state.sessions.clone(),
        form.board_size,
        form.handicap + 1,
        scoring,
        receiver
    ));
    //Register session 
    sessions.insert(id, sender);
    Ok((StatusCode::SEE_OTHER, response::Redirect::to(&format!("/play/{}", id))))
}

pub async fn get_session(
    Path(game): Path<usize>,
    State(state): State<AppState>
) -> Result<response::Html<String>, StatusCode> {
    if let Some(_session) = {
        let sessions = state.sessions.lock().unwrap();
        let session = sessions.get(&game);
        if let Some(session) = session {
            Some(session.clone())
        } else {
            None
        }
    } {
        let mut data = HashMap::<String, String>::new();
        data.insert("id".to_string(), game.to_string());
        let body = state.templates.render("lobby", &data).unwrap();
        Ok(response::Html(body))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn join_session(
    Path((game, side)): Path<(usize, String)>,
    State(state): State<AppState>
) -> Result<response::Html<String>, StatusCode> {
    if let Some(session) = {
        let sessions = state.sessions.lock().unwrap();
        let session = sessions.get(&game);
        if let Some(session) = session {
            Some(session.clone())
        } else {
            None
        }
    } {
        //Check occupancy
        let stone = match side.as_str() {
            "black" => Stone::Black,
            "white" => Stone::White,
            _ => return Err(StatusCode::NOT_FOUND)
        };
        let (sender, receiver) = oneshot::channel();
        let message = (session::Message::Occupancy(stone), sender);
        let available = session.send(message).is_ok() && match receiver.await {
            Ok(Ok(session::Response::Occupancy(false))) => true,
            _ => false
        };
        if available {
            Ok(response::Html(include_str!("../templates/game.html").to_string()))
        } else {
            Err(StatusCode::BAD_REQUEST)
        }
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
    socket: WebSocketUpgrade,
    Path((game, stone)): Path<(usize, String)>,
    State(state): State<AppState>,
) -> Response {
    socket.on_upgrade(move |socket| async move {
        let (mut socket_sender, mut socket_receiver) = socket.split();
        if let Some(session) = {
            let sessions = state.sessions.lock().unwrap();
            let session = sessions.get(&game);
            if let Some(session) = session {
                Some(session.clone())
            } else {
                None
            }
        } {
            let stone = match stone.as_str() {
                "black" => Stone::Black,
                "white" => Stone::White,
                _ => return
            };
            //Session listener
            let (session_sender, mut session_receiver) = mpsc::unbounded_channel::<serde_json::Value>();
            tokio::spawn(async move {
                while let Some(message) = session_receiver.recv().await {
                    let message = Message::Text(message.to_string());
                    if socket_sender.send(message).await.is_err() {
                        break;
                    }
                }
            });
            //Join session
            let (sender, receiver) = oneshot::channel();
            let message = (session::Message::Join(stone, session_sender), sender);
            if session.send(message).is_ok() && receiver.await.is_ok() {
                //Listen for messages
                while let Some(Ok(Message::Text(message))) = socket_receiver.next().await {
                    if let Some(message) = parse_message(stone, &message) {
                        let (sender, receiver) = oneshot::channel();
                        if session.send((message, sender)).is_err()
                            || receiver.await.is_err() {
                            break
                        }
                    }
                }
            }
            //Leave session 
            let (sender, _) = oneshot::channel();
            let _ = session.send((session::Message::Leave(stone), sender));
        }
    })
}
