use h2::server;
use http::{Response, StatusCode};
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
                    let (request, mut respond) = request.unwrap();

                    let res = crate::request::proxy(
                        request.uri(),
                        request.method(),
                        request.headers().clone(),
                        peer_addr,
                    )
                    .await;

                    match res {
                        Ok((meta, body)) => {
                            let mut stream = respond.send_response(meta, false).unwrap();
                            stream.send_data(body, true).unwrap();
                        }
                        Err(_) => {
                            let meta = Response::builder()
                                .status(StatusCode::BAD_GATEWAY)
                                .body(())
                                .unwrap();
                            respond.send_response(meta, true).unwrap();
                        }
                    }
                }
            });
        }
    }
}
