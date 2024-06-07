use axum::{routing::get, Router};
use std::net::SocketAddr;

pub async fn run() {
    let app = Router::new().route("/", get(hello));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening API on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn hello() -> String {
    "Hello World".into()
}
