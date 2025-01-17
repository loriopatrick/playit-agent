use async_trait::async_trait;
use hyper::{Body, header, Method, Request};
use hyper::body::Buf;
use hyper::client::HttpConnector;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::api::api::{ApiResult, PlayitHttpClient};

pub struct HttpClient {
    api_base: String,
    auth_header: Option<String>,
    client: hyper::Client<HttpsConnector<HttpConnector>, Body>,
}

impl HttpClient {
    pub fn new(api_base: String, auth_header: Option<String>) -> Self {
        let connector = if api_base.starts_with("http://") {
            HttpsConnectorBuilder::new()
                .with_native_roots()
                .https_or_http()
                .enable_http1()
                .enable_http2()
                .build()
        } else {
            HttpsConnectorBuilder::new()
                .with_native_roots()
                .https_only()
                .enable_http1()
                .enable_http2()
                .build()
        };

        HttpClient {
            api_base,
            auth_header,
            client: hyper::Client::builder().build(connector),
        }
    }
}

#[async_trait]
impl PlayitHttpClient for HttpClient {
    type Error = HttpClientError;

    async fn call<Req: Serialize + Send, Res: DeserializeOwned, Err: DeserializeOwned>(&self, path: &str, req: Req) -> Result<ApiResult<Res, Err>, Self::Error> {
        let mut builder = Request::builder()
            .uri(format!("{}{}", self.api_base, path))
            .method(Method::POST);

        if let Some(auth_header) = &self.auth_header {
            builder = builder.header(
                header::AUTHORIZATION,
                auth_header,
            );
        }

        let request_str = serde_json::to_string(&req)
            .map_err(|e| HttpClientError::SerializeError(e))?;

        let request = builder
            .body(Body::from(request_str))
            .unwrap();

        let response = self.client.request(request).await
            .map_err(|e| HttpClientError::RequestError(e))?;
        let bytes = hyper::body::aggregate(response.into_body()).await
            .map_err(|e| HttpClientError::RequestError(e))?;
        let response_txt = String::from_utf8_lossy(bytes.chunk());

        let result: ApiResult<Res, Err> = serde_json::from_str(&response_txt)
            .map_err(|e| HttpClientError::ParseError(e))?;

        Ok(result)
    }
}

#[derive(Debug)]
pub enum HttpClientError {
    SerializeError(serde_json::Error),
    ParseError(serde_json::Error),
    RequestError(hyper::Error),
}