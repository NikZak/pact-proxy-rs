use std::collections::HashMap;
use std::error::Error;
use std::{io, thread};
use std::io::Cursor;
use std::path::{Path};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use pact_models::prelude::v4::V4Pact;
use pact_models::v4::http_parts::{HttpRequest, HttpResponse};
use tracing::debug;
use tiny_http::{Response, Server};
use crate::{pact::request_to_pact::http_request_to_pact, web};
use crate::cli::get_rand_port;
use crate::pact::pact_to_response::pact_response_to_http_response;
#[cfg(feature = "flame_it")]
use flamer::flame;
use crate::pact::{edit};
use crate::pacts::Pacts;

pub type InteractionIndexMap = HashMap<(String, String), HashMap<String, u16>>;
pub type Port = String;

enum WrappedServer {
    Httpserver(Arc<Server>),
    Grpc,
}

pub struct PactServer {
    server: WrappedServer,
    server_thread: Option<JoinHandle<Result<(), String>>>,
    pacts: Arc<Pacts>,
}

impl PactServer {
    pub fn port(&self) -> Result<Port, Box<dyn Error>> {
        Ok(match &self.server {
            WrappedServer::Httpserver(server)
            => server.server_addr().to_ip()
                .ok_or("No port")?.port().to_string(),
            WrappedServer::Grpc => {todo!()},
        })
    }

    pub fn with_http_server(
        pacts_folder: &Path,
        pacts: Option<HashMap<(String, String), V4Pact>>,
        port: Option<Port>,
    ) -> Result<Self, Box<dyn Error>> {
        let port = match port {
            Some(port) => port,
            None => get_rand_port().to_string(),
        };
        let pacts = match pacts {
            Some(pacts) => pacts,
            None => edit::read_pacts(pacts_folder)?
        };
        let interaction_index_map = make_interaction_index_map(&pacts);
        let pacts = Arc::new(Pacts::new(
            Arc::new(Mutex::new(pacts)),
            Arc::new(Mutex::new(interaction_index_map)),
            pacts_folder.to_path_buf())
        );
        Ok(PactServer {
            server: make_http_server(port)?,
            server_thread: None,
            pacts: pacts,
        })
    }

    pub async fn start_blocking(&mut self) -> Result<(), Box<dyn Error>> {
        match &self.server {
            WrappedServer::Httpserver(server) => {
                Ok(run_http_server(server.clone().as_ref(), self.pacts.clone()).await?)
            }
            WrappedServer::Grpc => {
                unimplemented!()
            }
        }
    }

    pub async fn start_non_blocking(&mut self) -> io::Result<()> {
        let port = self.port().expect("No port");
        match &self.server {
            WrappedServer::Httpserver(server) => {
                debug!("Starting pact server on port {}", port);
                let pacts = self.pacts.clone();
                let server = server.clone();
                self.server_thread = spawn_thread_with_http_server(pacts, server);
            }
            WrappedServer::Grpc => {
                unimplemented!()
            }
        }
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.is_running() {
            debug!("Server is not running");
            return Ok(());
        }
        match &self.server {
            WrappedServer::Httpserver(server) => {
                debug!("Unblocking tiny-htpp server, sending unblock to the message queue");
                server.unblock();
                if let Some(server_thread) = self.server_thread.take() {
                    debug!("Joining thread");
                    server_thread.join().expect("Could not join server thread")?;
                    debug!("Thread joined");
                }
                Ok(())
            }
            WrappedServer::Grpc => {
                unimplemented!()
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.server_thread.is_some()
    }

}

#[cfg_attr(feature = "flame_it", flame)]
fn spawn_thread_with_http_server(pacts: Arc<Pacts>,  server: Arc<Server>) -> Option<JoinHandle<Result<(), String>>> {
    Some(thread::spawn(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                match run_http_server(
                    server.clone().as_ref(),
                    pacts,
                ).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        debug!("Error: {}", e);
                        Err("Could not run http server".to_string())
                    }
                }
            })
    }))
}

#[cfg_attr(feature = "flame_it", flame)]
async fn run_http_server(
    server: &Server,
    pacts: Arc<Pacts>,
) -> Result<(), Box<dyn Error>> {
    for mut request in server.incoming_requests() {
        debug!("Got request: {:?}", request);
        let response = get_response(&pacts,   &mut request ).await?;
        debug!("Sending back response");
        #[cfg(feature = "flame_it")]
        dump_flame_file(request.url());
        request.respond(response)?;
    }
    Ok(())
}
#[cfg_attr(feature = "flame_it", flame)]
async fn get_response(
    pacts: &Arc<Pacts>,
    request: &mut tiny_http::Request,
) -> Result<Response<Cursor<Vec<u8>>>, Box<dyn Error>> {
    let pact_request = http_request_to_pact(request)?;
    debug!("pact_request: {pact_request:?}");
    let consumer_provider = edit::get_consumer_provider(&pact_request)?;
    debug!("Checking if pact exists for consumer: {} and provider: {}", consumer_provider.0, consumer_provider.1);
    let response = match pacts.get_pact_response(&consumer_provider.0, &consumer_provider.1, &pact_request.path) {
        Some(pact_response) => {
            response_when_interaction_exists(&pact_response)?
        }
        None => {
            response_when_no_interaction(pacts, &pact_request, &consumer_provider).await?
        }
    };
    Ok(response)
}

#[cfg_attr(feature = "flame_it", flame)]
fn response_when_interaction_exists(pact_response: &HttpResponse) -> Result<Response<Cursor<Vec<u8>>>, Box<dyn Error>> {
    debug!("Match found");
    let response = pact_response_to_http_response(pact_response)?;
    debug!("pact_response: {pact_response:?}");
    Ok(response)
}

#[cfg_attr(feature = "flame_it", flame)]
async fn response_when_no_interaction(
    pacts: &Arc<Pacts>,
    pact_request: &HttpRequest,
    consumer_provider: &(String, String),
) -> Result<Response<Cursor<Vec<u8>>>, Box<dyn Error>> {
    let (pact_response, response) = web::get_response_from_web(pact_request).await?;
    pacts.add_interaction(&consumer_provider.0, &consumer_provider.1, pact_request, &pact_response)?;
    pacts.save_pact(&consumer_provider.0, &consumer_provider.1)?;
    Ok(response)
}

#[cfg_attr(feature = "flame_it", flame)]
fn make_interaction_index_map(pacts: & HashMap<(String, String), V4Pact>) -> InteractionIndexMap {
    let mut interaction_index = HashMap::new();
    for (consumer_provider, pact) in pacts.iter() {
        let mut index = HashMap::new();
        for (i, interaction) in pact.interactions.iter().enumerate() {
            index.insert(interaction.description(), i as u16);
        }
        interaction_index.insert(consumer_provider.clone(), index);
    }
    interaction_index
}

fn make_http_server(port: String) -> Result<WrappedServer, Box<dyn Error>> {
    let server = Server::http("localhost:".to_owned() + port.as_str());
    match server {
        Ok(server) => Ok(WrappedServer::Httpserver(Arc::new(server))),
        Err(e) => {
            debug!("Error starting server: {}", e);
            Err("Could not start server".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::str::FromStr;
    use serde_json::Value;
    use super::*;
    use crate::utils::set_hook_on_panic_or_signal;

    const PACTS_FOLDER: &str = "/tmp/pacts";

    #[test_log::test(tokio::test)]
    async fn test_integration_start_non_blocking() {
        set_hook_on_panic_or_signal(cleanup_pacts_folder).unwrap();
        let test_pacts_folder = PathBuf::from_str(PACTS_FOLDER).unwrap();
        assert!(!test_pacts_folder.exists());
        let mut pact_server = PactServer::with_http_server(&test_pacts_folder, None, None).unwrap();
        let port = pact_server.port().unwrap();
        pact_server.start_non_blocking().await.unwrap();
        let test_urls = vec!(
            [
                format!("http://localhost:{}/https/httpbin.org/get", port),
                format!("http://localhost:{}/https/httpbin.org/post", port)
            ]
        );
        let mut bodies: Vec<Value> = Vec::new();
        for test_url in test_urls.iter() {
            let response = reqwest::get(test_url[0].as_str()).await.unwrap();
            assert_eq!(response.status(), 200);
            bodies.push(response.json().await.unwrap());
        }
        // make second attempt, now should come from file faster
        for (index, test_url) in test_urls.iter().enumerate() {
            let response = reqwest::get(test_url[0].as_str()).await.unwrap();
            assert_eq!(response.status(), 200);
            assert_eq!(response.json::<Value>().await.unwrap(), bodies[index]);
        }
        pact_server.stop().unwrap();
        cleanup_pacts_folder()
    }

    fn cleanup_pacts_folder() {
        let pacts_folder = PathBuf::from_str(PACTS_FOLDER).unwrap();
        if pacts_folder.exists() {
            std::fs::remove_dir_all(pacts_folder).unwrap();
        }
    }
}