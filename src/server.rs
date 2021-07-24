use h2::server;
use tokio::net::TcpListener;

pub async fn start() {
    let acceptor = crate::tls::create_acceptor();
    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();

    loop {
        if let Ok((socket, peer_addr)) = listener.accept().await {
            let acceptor = acceptor.clone();

            tokio::spawn(async move {
                let socket = acceptor.accept(socket).await.unwrap();
                let mut h2 = server::handshake(socket).await.unwrap();
                while let Some(request) = h2.accept().await {
                    let (request, respond) = request.unwrap();

                    crate::request::proxy(
                        respond,
                        request.uri(),
                        request.method(),
                        request.headers().clone(),
                        peer_addr,
                    )
                    .await;
                }
            });
        }
    }
}
