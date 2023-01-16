use std::collections::HashMap;
use std::error::Error;
use pact_models::bodies::OptionalBody;
use url::Url;
use tiny_http::Request;
use pact_models::v4::http_parts::{HttpRequest, };
use tracing::debug;
#[cfg(feature = "flame_it")]
use flamer::flame;

#[cfg_attr(feature = "flame_it", flame)]
pub fn http_request_to_pact(request: &mut Request) -> Result<HttpRequest, Box<dyn Error>> {
    // we assume a post request and that all the data is in the body and that from_json will just read it
    let mut pact_request = HttpRequest::default();
    let url = get_forward_url(request);

    set_method(request, &mut pact_request)?;
    set_path(&mut pact_request, &url);
    set_query(&mut pact_request, &url);
    set_headers(request, &mut pact_request, url)?;
    set_body(request);
    debug!("pact_request: {:?}", pact_request);
    Ok(pact_request)
}

fn set_method(request: &mut Request, pact_request: &mut HttpRequest) -> Result<(), Box<dyn Error>>{
    if request.method() != &tiny_http::Method::Get {
        debug!("request: {:?}", request);
        return Err("Only GET requests are supported".into());
    }
    pact_request.method = request.method().to_string();
    Ok(())
}

fn set_path(pact_request: &mut HttpRequest, url: &Url) {
    debug!("url: {:?}", url);
    pact_request.path = url.to_string();
}

fn set_body(request: &mut Request) {
    let mut content = String::new();
    debug!("reading request body");
    request.as_reader().read_to_string(&mut content).unwrap();
    debug!("request body: {}", content);
    match content.is_empty() {
        true => OptionalBody::Empty,
        false => OptionalBody::Present(content.into_bytes().into(), None, None)
    };
}

fn set_headers(request: &mut Request, pact_request: &mut HttpRequest, url: Url) -> Result<(), Box<dyn Error>> {
    pact_request.headers = match request.headers().is_empty() {
        true => None,
        false => {
            let mut headers_map: HashMap<String, Vec<String>> = HashMap::new();
            for header in request.headers().iter() {
                headers_map.entry(header.field.to_string()).or_insert(vec![header.value.to_string()]).push(header.value.to_string());
            }
            Some(headers_map)
        }
    };
    set_host_header(pact_request, url)?;
    Ok(())
}

fn set_host_header(pact_request: &mut HttpRequest, url: Url) -> Result<(), Box<dyn Error>> {
    if let Some(headers) = &mut pact_request.headers {
        if let Some(host) = headers.get_mut("host") {
            host[0] = url.host().ok_or("url should have host")?.to_string();
            host.truncate(1);
        }
        if let Some(host) = headers.get_mut("Host") {
            host[0] = url.host().ok_or("url should have host")?.to_string();
            host.truncate(1);
        }
    }
    Ok(())
}

fn set_query(pact_request: &mut HttpRequest, url: &Url) {
    let mut query_map: HashMap<String, Vec<String>> = HashMap::new();
    for (key, value) in url.query_pairs() {
        query_map.entry(key.into()).or_insert(vec![value.clone().into()]).push(value.clone().into());
    }
    pact_request.query = match query_map.is_empty() {
        true => None,
        false => Some(query_map)
    };
}

fn get_forward_url(request: &mut Request) -> Url {
    let relative_url = request.url();
    // remove / from the beginning of the url if it exists
    // strip_prefix
    let relative_url = match relative_url.strip_prefix('/') {
        Some(url) => url,
        None => relative_url
    };
    let scheme = relative_url.split('/').next().unwrap().to_string();
    let host = relative_url.split('/').nth(1).unwrap().to_string();
    let path = relative_url.split('/').skip(2).collect::<Vec<&str>>().join("/");
    Url::parse(&(scheme + "://" + host.as_str() + "/" + path.as_str())).unwrap()
}
