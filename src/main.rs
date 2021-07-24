mod request;
mod server;
mod tls;

#[tokio::main]
pub async fn main() {
    server::start().await;
}
