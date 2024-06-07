mod api;
mod proxy;

#[tokio::main]
async fn main() {
    tokio::join!(proxy::run(), api::run());
}
