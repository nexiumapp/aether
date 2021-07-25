#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

mod request;
mod server;
mod tls;

#[tokio::main]
pub async fn main() {
    env_logger::init();

    server::start().await;
}
