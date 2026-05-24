use axum::body::{to_bytes, Body};
use bytes::Bytes;
use hyper::body::Incoming;
use http_body_util::Full;
use http_body_util::BodyExt;
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::rt::TokioExecutor;
use std::error::Error;

type BoxError = Box<dyn Error + Send + Sync>;

pub async fn proxy_request(
    req: axum::http::Request<Body>,
    target_base_url: &str,
    path_prefix: &str,
) -> Result<axum::http::Response<Body>, BoxError> {
    let target_uri = build_target_uri(req.uri(), target_base_url, path_prefix)?;
    let (parts, body) = req.into_parts();

    let body_bytes = to_bytes(body, 16 * 1024 * 1024).await?;
    let mut builder = hyper::Request::builder()
        .method(parts.method)
        .uri(target_uri.clone())
        .version(parts.version);

    let headers = builder
        .headers_mut()
        .expect("request builder should expose headers");
    copy_request_headers(&parts.headers, headers);
    headers.remove(hyper::header::HOST);
    headers.remove(hyper::header::CONNECTION);
    headers.remove(hyper::header::UPGRADE);

    let upstream_request = builder.body(Full::new(Bytes::from(body_bytes)))?;
    let client = HyperClient::builder(TokioExecutor::new()).build_http::<Full<Bytes>>();

    let upstream_response = client.request(upstream_request).await?;
    let (parts, body) = upstream_response.into_parts();
    let response_bytes = collect_body(body).await?;

    let mut response_builder = axum::http::Response::builder()
        .status(parts.status)
        .version(parts.version);

    let response_headers = response_builder
        .headers_mut()
        .expect("response builder should expose headers");
    copy_response_headers(&parts.headers, response_headers);

    Ok(response_builder.body(Body::from(response_bytes))?)
}

fn build_target_uri(
    original_uri: &axum::http::Uri,
    target_base_url: &str,
    path_prefix: &str,
) -> Result<axum::http::Uri, BoxError> {
    let path_and_query = original_uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");

    let trimmed = path_and_query
        .strip_prefix(path_prefix)
        .unwrap_or(path_and_query);

    let target = if trimmed.is_empty() {
        target_base_url.to_string()
    } else {
        format!("{}{}", target_base_url, trimmed)
    };

    Ok(axum::http::Uri::try_from(target)?)
}

fn copy_request_headers(from: &axum::http::HeaderMap, to: &mut hyper::HeaderMap) {
    for (name, value) in from.iter() {
        if name == hyper::header::HOST
            || name == hyper::header::CONNECTION
            || name == hyper::header::UPGRADE
        {
            continue;
        }
        to.insert(name, value.clone());
    }
}

fn copy_response_headers(from: &hyper::HeaderMap, to: &mut axum::http::HeaderMap) {
    for (name, value) in from.iter() {
        if name == hyper::header::CONNECTION || name == hyper::header::UPGRADE {
            continue;
        }
        to.insert(name, value.clone());
    }
}

async fn collect_body( body: Incoming) -> Result<Bytes, BoxError> {
    let collected = body.collect().await.map_err(|e| -> BoxError { Box::new(e) })?;
    Ok(collected.to_bytes())
}
