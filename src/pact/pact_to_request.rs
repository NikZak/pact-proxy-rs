use pact_models::v4::http_parts::HttpRequest;
use reqwest::RequestBuilder;
use std::error::Error;

pub fn copy_pact_headers_to_request(
    pact_request: &HttpRequest,
    mut reqwest_request: RequestBuilder,
) -> Result<RequestBuilder, Box<dyn Error>> {
    for hash_map in pact_request.headers.iter() {
        for (key, values) in hash_map {
            for value in values {
                reqwest_request = reqwest_request.header(key, value);
            }
        }
    }
    Ok(reqwest_request)
}
