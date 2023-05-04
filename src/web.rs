use crate::pact::pact_to_request::copy_pact_headers_to_request;
use crate::pact::pact_to_response::pact_response_to_http_response;
use crate::pact::response_to_pact::reqwest_response_to_pact;
use bytes::Bytes;
#[cfg(feature = "flame_it")]
use flame as f;
#[cfg(feature = "flame_it")]
use flamer::flame;
use http::Method;
use pact_models::bodies::OptionalBody;
use pact_models::content_types::ContentType;
use pact_models::v4::http_parts::{HttpRequest, HttpResponse};
use reqwest::{Client, RequestBuilder};
use std::error::Error;
use std::io::Cursor;
use tiny_http::Response;
use tracing::debug;

#[cfg_attr(feature = "flame_it", flame)]
pub async fn get_response_from_web(
    pact_request: &HttpRequest,
) -> Result<(HttpResponse, Response<Cursor<Vec<u8>>>), Box<dyn Error>> {
    let mut pact_response = forward_get_request(pact_request).await?;
    adjust_body_and_content_length(&mut pact_response)?;
    let response = pact_response_to_http_response(&pact_response)?;
    Ok((pact_response, response))
}

fn adjust_body_and_content_length(pact_response: &mut HttpResponse) -> Result<(), Box<dyn Error>> {
    let content_type = match pact_response.body.content_type() {
        Some(content_type) => content_type,
        None => {
            return Ok(());
        }
    };
    if content_type.main_type == "application" && content_type.sub_type == "json" {
        adjust_body_to_pact_serialization(pact_response, content_type)?;
        adjust_content_length(pact_response)?;
    }
    Ok(())
}

fn adjust_content_length(pact_response: &mut HttpResponse) -> Result<(), Box<dyn Error>> {
    let body = match &pact_response.body {
        OptionalBody::Present(body, _, _) => body,
        _ => {
            return Ok(());
        }
    };
    if pact_response.headers.is_none() {
        return Ok(());
    }
    if pact_response
        .headers
        .as_ref()
        .unwrap()
        .get("content-length")
        .is_none()
    {
        return Ok(());
    }
    let content_length = body.len();
    let _ = pact_response.headers.as_mut().unwrap().insert(
        "content-length".to_string(),
        vec![content_length.to_string()],
    );
    Ok(())
}

fn adjust_body_to_pact_serialization(
    pact_response: &mut HttpResponse,
    content_type: ContentType,
) -> Result<(), Box<dyn Error>> {
    let body = match &pact_response.body {
        OptionalBody::Present(body, _, _) => body,
        _ => {
            return Ok(());
        }
    };
    let json_body = serde_json::from_slice::<serde_json::Value>(body)?;
    let pretty_json_body = serde_json::to_string(&json_body)?;
    let pretty_json_body_bytes = Bytes::from(pretty_json_body);
    pact_response.body = OptionalBody::Present(pretty_json_body_bytes, Some(content_type), None);
    Ok(())
}

async fn forward_get_request(request: &HttpRequest) -> Result<HttpResponse, Box<dyn Error>> {
    let mut response = None;
    for _ in 0..5 {
        let reqwest_request = build_request(request)?;
        response = Some(reqwest_request.send().await?);
        if let Some(ref res) = response {
            if res.status().is_success() {
                return reqwest_response_to_pact(response.unwrap()).await;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }
    debug!("received response: {:?}", response);
    Err("Could not get correct response from server".into())
}

fn build_request(pact_request: &HttpRequest) -> Result<RequestBuilder, Box<dyn Error>> {
    let client = Client::new();
    let url = pact_request.path.clone();
    let method = pact_request.method.clone();
    let reqwest_request = client.request(Method::from_bytes(method.as_bytes())?, url);
    let reqwest_request = copy_pact_headers_to_request(pact_request, reqwest_request)?;
    Ok(reqwest_request)
}
