use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

type ServerBuilder = hyper::server::conn::http1::Builder;

use charlotte::proxy;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server_addr = "127.0.0.1:8888";
    let listener = TcpListener::bind(server_addr).await?;

    println!("Listening on http://{}", server_addr);
    loop {
        let (socket, socket_addr) = listener.accept().await?;
        println!("Client '{}' connected", socket_addr);

        let io = TokioIo::new(socket);
        tokio::spawn(async move {
            if let Err(err) = ServerBuilder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(io, service_fn(proxy))
                .with_upgrades()
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
