use http::{header::InvalidHeaderValue, uri::PathAndQuery, HeaderMap, Method, Response, Uri};
use hyper::{
    body::{self, Bytes},
    Body, Client, Request,
};
use std::{env, net::SocketAddr};
use thiserror::Error;

lazy_static! {
    static ref TARGET_HOST: String = match env::var("TARGET_HOST") {
        Ok(val) => val,
        Err(_) => "server".to_string(),
    };
    static ref TARGET_PORT: u32 = match env::var("TARGET_PORT") {
        Ok(val) => val.parse().expect("TARGET_PORT is not an valid number!"),
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
) -> Result<Response<Body>, RequestError> {
    let authority = format!("{}:{}", TARGET_HOST.to_string(), TARGET_PORT.to_string());

    headers.insert("X-Forwarded-For", addr.ip().to_string().parse()?);
    headers.insert("host", TARGET_HOST.parse()?);

    let part_query = uri
        .path_and_query()
        .unwrap_or(&PathAndQuery::from_static("/"))
        .to_owned();

    let uri = Uri::builder()
        .scheme(TARGET_SCHEME.as_str())
        .authority(authority.as_str())
        .path_and_query(part_query)
        .build()?;

    let body = Body::empty();

    let mut req = Request::builder().uri(uri).method(method).body(body)?;
    *req.headers_mut() = headers;

    let client = Client::new();

    Ok(client.request(req).await?)
}

pub async fn proxy(
    uri: &Uri,
    method: &Method,
    headers: HeaderMap,
    addr: SocketAddr,
) -> Result<(Response<()>, Bytes), RequestError> {
    let mut res = send(uri, method, headers, addr).await?;

    let mut meta_builder = Response::builder().status(res.status());

    let headers = meta_builder.headers_mut().unwrap();
    map_headers(headers, res.headers());

    let meta = meta_builder.body(())?;

    Ok((meta, body::to_bytes(res.body_mut()).await?))
}

fn map_headers(headers: &mut HeaderMap, old_headers: &HeaderMap) {
    for (key, value) in old_headers {
        let include = !matches!(
            key.as_str(),
            "connection" | "keep-alive" | "upgrade" | "transfer-encoding"
        );

        if include {
            headers.insert(key, value.clone());
        }
    }
}

#[derive(Error, Debug)]
pub enum RequestError {
    #[error(transparent)]
    HyperError(#[from] hyper::Error),
    #[error(transparent)]
    HttpError(#[from] http::Error),
    #[error(transparent)]
    H2Error(#[from] h2::Error),
    #[error(transparent)]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
}
