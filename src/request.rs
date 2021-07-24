use h2::server::SendResponse;
use http::{HeaderMap, Method, Response, Uri};
use hyper::{
    body::{self, Bytes},
    Body, Client, Request,
};
use std::{env, net::SocketAddr};

lazy_static! {
    static ref TARGET_HOST: String = match env::var("TARGET_HOST") {
        Ok(val) => val,
        Err(_) => "server".to_string(),
    };
    static ref TARGET_PORT: u32 = match env::var("TARGET_PORT") {
        Ok(val) => val.parse().unwrap(),
        Err(_) => 80,
    };
    static ref TARGET_SCHEME: String = match env::var("TARGET_SCHEME") {
        Ok(val) => val,
        Err(_) => "http".to_string(),
    };
}

async fn send(
    uri: &Uri,
    method: &Method,
    mut headers: HeaderMap,
    addr: SocketAddr,
) -> Response<Body> {
    let authority = format!("{}:{}", TARGET_HOST.to_string(), TARGET_PORT.to_string());

    headers.insert("X-Forwarded-For", addr.ip().to_string().parse().unwrap());
    headers.insert("host", TARGET_HOST.parse().unwrap());

    let uri = Uri::builder()
        .scheme(TARGET_SCHEME.as_str())
        .authority(authority.as_str())
        .path_and_query(uri.path_and_query().unwrap().to_string())
        .build()
        .unwrap();

    let body = Body::empty();

    let mut req = Request::builder()
        .uri(uri)
        .method(method)
        .body(body)
        .unwrap();
    *req.headers_mut() = headers;

    let client = Client::new();

    client.request(req).await.unwrap()
}

pub async fn proxy(
    mut responder: SendResponse<Bytes>,
    uri: &Uri,
    method: &Method,
    headers: HeaderMap,
    addr: SocketAddr,
) {
    let mut res = send(uri, method, headers, addr).await;

    let mut meta_builder = Response::builder().status(res.status());

    *meta_builder.headers_mut().unwrap() = map_headers(res.headers());
    let meta = meta_builder.body(()).unwrap();

    let mut stream = responder.send_response(meta, false).unwrap();
    stream
        .send_data(body::to_bytes(res.body_mut()).await.unwrap(), true)
        .unwrap();
}

fn map_headers(old_headers: &HeaderMap) -> HeaderMap {
    let mut new_headers = HeaderMap::new();

    for (key, value) in old_headers {
        let include = !matches!(
            key.as_str(),
            "connection" | "keep-alive" | "upgrade" | "transfer-encoding"
        );

        if include {
            new_headers.insert(key, value.clone());
        }
    }

    new_headers
}
