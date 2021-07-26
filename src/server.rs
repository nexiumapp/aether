use crate::{kube::Kube, request};
use http::{Request, Response};
use hyper::{server::conn::Http, service::service_fn, Body};
use std::{net::SocketAddr, sync::Arc};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;

pub async fn start(kube: Arc<Kube>) {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let acceptor = crate::tls::create_acceptor();

    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind the socket!");

    loop {
        if let Ok((socket, addr)) = listener.accept().await {
            match handle_connection(acceptor.clone(), socket, addr, kube.clone()).await {
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
    kube: Arc<Kube>,
) -> Result<(), std::io::Error> {
    let socket = acceptor.accept(socket).await?;

    debug!("Connection accepted from {}.", addr);

    tokio::spawn(async move {
        if let Err(http_err) = Http::new()
            .serve_connection(socket, service_fn(|req| proxy(req, addr, kube.clone())))
            .await
        {
            error!("Error while serving HTTP connection: {}", http_err);
        }
    });

    Ok(())
}

async fn proxy(
    req: Request<Body>,
    addr: SocketAddr,
    kube: Arc<Kube>,
) -> Result<Response<Body>, request::RequestError> {
    Ok(request::send(req.uri(), req.method(), req.headers(), addr, kube).await?)
}
