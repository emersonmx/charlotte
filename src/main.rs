mod api;
#[tokio::main]
async fn main() {
    api::run().await;
}
