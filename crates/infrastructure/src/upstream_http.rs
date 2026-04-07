use axum::body::Body;
use axum::http::header::{CONNECTION, HOST};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, Response, StatusCode, Uri};
use reqwest::Client;
use tracing::error;
use url::Url;

pub async fn forward_request(
    client: &Client,
    method: Method,
    incoming_headers: HeaderMap,
    incoming_uri: Uri,
    upstream_url: Url,
    body: bytes::Bytes,
    extra_headers: &[(HeaderName, HeaderValue)],
) -> Result<Response<Body>, (StatusCode, String)> {
    let reqwest_method =
        reqwest::Method::from_bytes(method.as_str().as_bytes()).map_err(internal_error)?;
    let mut builder = client.request(reqwest_method, upstream_url);

    for (name, value) in incoming_headers.iter() {
        if should_skip_forward_header(name, extra_headers) {
            continue;
        }
        builder = builder.header(name, value);
    }

    for (name, value) in extra_headers {
        builder = builder.header(name, value);
    }

    let upstream_response = builder.body(body).send().await.map_err(bad_gateway_error)?;
    let status = upstream_response.status();
    let response_headers = upstream_response.headers().clone();
    let response_body = Body::from_stream(upstream_response.bytes_stream());

    let mut response = Response::builder().status(status);
    for (name, value) in response_headers.iter() {
        if name == CONNECTION {
            continue;
        }
        response = response.header(name, value);
    }

    response.body(response_body).map_err(|error| {
        error!(
            method = %method,
            uri = %incoming_uri,
            "failed to build response: {error}"
        );
        internal_error(error)
    })
}

pub fn build_upstream_url(
    base_url: &str,
    path: &str,
    query: Option<&str>,
) -> Result<Url, url::ParseError> {
    let mut url = Url::parse(base_url)?;
    url.set_path(path);
    url.set_query(query);
    Ok(url)
}

fn should_skip_forward_header(
    name: &HeaderName,
    extra_headers: &[(HeaderName, HeaderValue)],
) -> bool {
    name == HOST
        || name == CONNECTION
        || name.as_str().eq_ignore_ascii_case("x-target")
        || extra_headers
            .iter()
            .any(|(extra_name, _)| extra_name == name)
}

fn internal_error(error: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn bad_gateway_error(error: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::BAD_GATEWAY, error.to_string())
}
