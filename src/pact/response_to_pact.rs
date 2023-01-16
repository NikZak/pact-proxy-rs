use reqwest::header::{HeaderMap, };
use std::error::Error;
use std::collections::{HashMap};
use pact_models::prelude::ContentType;
use pact_models::v4::http_parts::HttpResponse;
use reqwest::Response;
use tracing::debug;

pub async fn reqwest_response_to_pact(response: Response) -> Result<HttpResponse, Box<dyn Error>> {
    let mut pact_response = HttpResponse::default();
    set_pact_response_status(&response, &mut pact_response);
    set_pact_response_headers(&response, &mut pact_response, true);
    set_pact_response_body(response, &mut pact_response).await?;
    Ok(pact_response)
}


fn set_pact_response_status(response: &Response, pact_response: &mut HttpResponse) {
    pact_response.status = response.status().as_u16();
}

fn set_pact_response_headers(response: &Response, pact_response: &mut HttpResponse, make_small_cap:bool) {
    let headers = response.headers().clone();
    pact_response.headers = reqwest_headers_to_pact_headers(&headers, make_small_cap);
}

async fn set_pact_response_body(response: Response, pact_response: &mut HttpResponse) -> Result<(), Box<dyn Error>> {
    // get content type if it is there
    let content_type = response.headers().get("content-type");
    match content_type {
        Some(header_value) => {
            // decompose content type
            let content_type = header_value.to_str()?;
            let content_type = ContentType::parse(content_type)?;
            let json_body = response.bytes().await?;
            debug!("json body: {json_body:?}");
            pact_response.body = pact_models::prelude::OptionalBody::Present(json_body, Some(content_type), None);
        },
        None => {
            debug!("no content type header");
        }
    }
    Ok(())

}

fn reqwest_headers_to_pact_headers(headers: &HeaderMap, make_small_cap:bool) -> Option<HashMap<String, Vec<String>>> {
    let mut pact_headers = HashMap::new();
    for (key, value) in headers.iter() {
        let mut key = key.as_str().to_string();
        if make_small_cap {
            key = key.to_lowercase();
        } else {
            panic!("Headers key must be small cap for fast match in map fashion. \
            If it is not then the map has to be read as vec subsequently and each \
            key has to be lowercased and then matched. This is not efficient and \
            not implemented yet");
        }
        let value = value.to_str().unwrap().to_string();
        pact_headers.entry(key).or_insert(vec![]).push(value);
    }
    if pact_headers.is_empty() {
        None
    } else {
        Some(pact_headers)
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_reqwest_response_to_pact() {
        let response = reqwest::get("https://jsonplaceholder.typicode.com/todos/1").await.unwrap();
        let pact_response = super::reqwest_response_to_pact(response).await.unwrap();
        assert_eq!(pact_response.status, 200);
        assert_eq!(pact_response.headers.unwrap().get("content-type").unwrap()[0], "application/json; charset=utf-8");
        assert_eq!(pact_response.body.content_type().unwrap().to_string(), "application/json;charset=utf-8");
    }
}

