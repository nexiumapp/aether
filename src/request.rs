use h2::server::SendResponse;
use http::{HeaderMap, Method, Response, Uri};
use hyper::{
    body::{self, Bytes},
    Body, Client, Request,
};
use std::net::SocketAddr;

async fn send(
    uri: &Uri,
    method: &Method,
    mut headers: HeaderMap,
    addr: SocketAddr,
) -> Response<Body> {
    headers.insert("X-Forwarded-For", addr.ip().to_string().parse().unwrap());
    headers.insert("host", "nexiumcore.com".parse().unwrap());
    let uri = Uri::builder()
        .scheme("http")
        .authority("nexiumcore.com")
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
