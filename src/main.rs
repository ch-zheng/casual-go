use axum::{
    Router,
    response,
    routing
};
use casual_go::handlers;
use handlebars::Handlebars;
use std::{
    env,
    collections::HashMap,
    sync::{Arc, Mutex},
    net::{Ipv6Addr, SocketAddr, IpAddr}
};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let port: u16 = if let Some(port) = args.get(1) {
        port.parse().unwrap()
    } else {
        80
    };
    let engine: String = if let Some(engine) = args.get(2) {
        engine.into()
    } else {
        "gnugo".into()
    };
    //Templates
    let mut templates = Handlebars::new();
    if templates.register_template_string("lobby", include_str!("../templates/lobby.hbs")).is_err() {
        return
    }
    if templates.register_template_string("game", include_str!("../templates/game.hbs")).is_err() {
        return
    }
    //App
    let state = handlers::AppState {
        templates,
        sessions: Arc::new(Mutex::new(HashMap::new())),
        engine
    };
    let app = Router::new()
        .route("/", routing::get(|| async {
            response::Html(include_str!("../templates/index.html").to_string())
        })).route("/create", routing::post(handlers::create_session))
        .route("/play/:game", routing::get(handlers::get_session))
        .route("/play/:game/:side", routing::get(handlers::join_session))
        .route("/ws/:game/:side", routing::get(handlers::connection))
        .route("/sse/:game", routing::get(handlers::spectate))
        .with_state(state);
    let socket = SocketAddr::new(
        IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)),
        port
    );
    axum::Server::bind(&socket)
        .serve(app.into_make_service())
        .await.unwrap();
}
