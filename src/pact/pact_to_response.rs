use pact_models::v4::http_parts::HttpResponse;
use tiny_http::{Response};
use std::error::Error;
use std::io::{Cursor, };
use bytes::Bytes;
use tracing::debug;
use tiny_http::Header;
#[cfg(feature = "flame_it")]
use flamer::flame;
use pact_models::bodies::OptionalBody;

#[cfg_attr(feature = "flame_it", flame)]
pub fn pact_response_to_http_response(pact_response: &HttpResponse) -> Result<Response<Cursor<Vec<u8>>>, Box<dyn Error>>{
    debug!("pact_response: {:?}", pact_response);
    let body_bytes = match pact_response.body {
        OptionalBody::Present(ref body, ..) => body.clone(),
        OptionalBody::Empty => Bytes::new(),
        OptionalBody::Missing => Bytes::new(),
        OptionalBody::Null => Bytes::new(),
    };

    debug!("json body: {:?}", body_bytes);
    // to vec u8
    let mut headers = vec![];
    for hashmap in pact_response.headers.iter() {
        for (key, values) in hashmap {
            for value in values {
                headers.push(Header::from_bytes(key.as_bytes(), value.as_bytes()).unwrap());
            }
        }
    };

    let response = Response::new(
        tiny_http::StatusCode(pact_response.status),
        headers,
        Cursor::new(body_bytes.into()),
        None,
        None,
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::*;

    #[test]
    fn test_pact_response_to_http_response() {
        let test_json_body = r#"{"test": "test"}"#;
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), vec!["application/json".to_string()]);
        let pact_response = HttpResponse {
            status: 200,
            headers: Some(headers),
            body: OptionalBody::Present(test_json_body.as_bytes().to_vec().into(), None, None),
            ..Default::default()
        };
        let http_response = pact_response_to_http_response(&pact_response);
        assert!(http_response.is_ok());
        assert!(http_response.as_ref().unwrap().status_code() == 200);
        assert!(http_response.as_ref().unwrap().headers().len() == 1);
    }
}