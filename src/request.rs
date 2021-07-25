use http::{header::InvalidHeaderValue, uri::PathAndQuery, HeaderMap, Method, Response, Uri};
use hyper::{Body, Client, Request};
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

pub async fn send(
    uri: &Uri,
    method: &Method,
    headers: &HeaderMap,
    addr: SocketAddr,
) -> Result<Response<Body>, RequestError> {
    let authority = format!("{}:{}", TARGET_HOST.to_string(), TARGET_PORT.to_string());
    let mut headers = headers.clone();

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

#[derive(Error, Debug)]
pub enum RequestError {
    #[error(transparent)]
    HyperError(#[from] hyper::Error),
    #[error(transparent)]
    HttpError(#[from] http::Error),
    #[error(transparent)]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
}
