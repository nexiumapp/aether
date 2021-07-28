#[macro_use]
extern crate log;

use std::sync::Arc;

mod kube;
mod request;
mod server;
mod tls;

#[tokio::main]
pub async fn main() {
    env_logger::init();
    let kube = Arc::new(kube::Kube::connect().await.unwrap());

    let kube_clone = kube.clone();
    tokio::spawn(async move {
        kube_clone.watch_ingress().await;
    });

    let kube_clone = kube.clone();
    tokio::spawn(async move {
        kube_clone.watch_certificates().await;
    });

    server::start(kube.clone()).await;
}
