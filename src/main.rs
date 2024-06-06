mod api;
mod proxy;

#[tokio::main]
async fn main() {
    let proxy_worker = tokio::spawn(proxy::run());
    let api_worker = tokio::spawn(api::run());

    // TODO: Add Command Manager

    api_worker.await.unwrap();
    proxy_worker.await.unwrap();
}
