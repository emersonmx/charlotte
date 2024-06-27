use axum::{routing::get, Router};
use std::{net::SocketAddr, str::FromStr};

pub async fn run() {
    let app = Router::new().route("/", get(hello));

    let addr = SocketAddr::from_str("127.0.0.1:3000").unwrap();
    println!("Listening API on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn hello() -> String {
    "Hello World".into()
}
