use crate::kube::Kube;
use http::{header::InvalidHeaderValue, uri::PathAndQuery, HeaderMap, Method, Response, Uri};
use hyper::{Body, Client, Request};
use std::{net::SocketAddr, sync::Arc};
use thiserror::Error;

pub async fn send(
    uri: &Uri,
    method: &Method,
    headers: &HeaderMap,
    addr: SocketAddr,
    kube: Arc<Kube>,
) -> Result<Response<Body>, RequestError> {
    let mut headers = headers.clone();

    headers.insert("X-Forwarded-For", addr.ip().to_string().parse()?);

    let part_query = uri
        .path_and_query()
        .unwrap_or(&PathAndQuery::from_static("/"))
        .to_owned();

    let authority = kube
        .get_authority(uri.host(), uri.path())
        .await
        .or(Err(RequestError::IngressNotAvailable))?;

    let uri = Uri::builder()
        .scheme("http")
        .authority(format!("{}:{}", authority.0, authority.1).as_str())
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
    #[error("Ingress is currently not available")]
    IngressNotAvailable,
}
