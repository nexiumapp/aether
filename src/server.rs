use crate::request;
use http::{Request, Response};
use hyper::{server::conn::Http, service::service_fn, Body};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;

pub async fn start() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let acceptor = crate::tls::create_acceptor();

    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind the socket!");

    loop {
        if let Ok((socket, addr)) = listener.accept().await {
            match handle_connection(acceptor.clone(), socket, addr).await {
                Err(e) => info!("Failed to handle connection: {}", e),
                Ok(()) => (),
            }
        }
    }
}

async fn handle_connection(
    acceptor: TlsAcceptor,
    socket: TcpStream,
    addr: SocketAddr,
) -> Result<(), std::io::Error> {
    let socket = acceptor.accept(socket).await?;

    tokio::spawn(async move {
        if let Err(http_err) = Http::new()
            .serve_connection(socket, service_fn(|req| proxy(req, addr)))
            .await
        {
            info!("Error while serving HTTP connection: {}", http_err);
        }
    });

    Ok(())
}

async fn proxy(
    req: Request<Body>,
    addr: SocketAddr,
) -> Result<Response<Body>, request::RequestError> {
    Ok(request::send(req.uri(), req.method(), req.headers(), addr).await?)
}
