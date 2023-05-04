use crate::server::InteractionIndexMap;
use crate::utils;
#[cfg(feature = "flame_it")]
use flame as f;
#[cfg(feature = "flame_it")]
use flamer::flame;
use pact_models::pact::Pact;
use pact_models::pact::{read_pact, write_pact};
use pact_models::prelude::v4::SynchronousHttp;
use pact_models::v4::http_parts::{HttpRequest, HttpResponse};
use pact_models::v4::pact::V4Pact;
use pact_models::PactSpecification;
use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use tracing::debug;
use url::Url;

const CONSUMER_NAME: &str = "consumer";

pub fn save_pact_to_file(pact: &V4Pact, pact_path: &Path) -> Result<(), Box<dyn Error>> {
    write_pact(pact.boxed(), pact_path, PactSpecification::V4, true)?;
    Ok(())
}

fn read_pact_from_file(pact_path: &Path) -> Result<V4Pact, Box<dyn Error>> {
    Ok(read_pact(pact_path)?.as_v4_pact()?)
}
pub fn derive_pact_file_path(pact_files_folder: &Path, pact: &V4Pact) -> PathBuf {
    let file_name = pact.consumer.name.clone() + "-" + &pact.provider.name.clone() + ".json";
    let mut path = pact_files_folder.to_path_buf();
    path.push(file_name);
    debug!("Pact file path: {:?}", path);
    path
}

pub fn read_pacts(
    pact_files_folder: &Path,
) -> Result<HashMap<(String, String), V4Pact>, Box<dyn Error>> {
    match utils::create_folder_if_not_exists(pact_files_folder) {
        Ok(_) => {}
        Err(_) => {
            println!(
                "Error: Can't create folder {}",
                pact_files_folder.to_str().unwrap()
            );
        }
    }
    let mut pacts: HashMap<(String, String), V4Pact> = HashMap::new();
    for entry in std::fs::read_dir(pact_files_folder)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let pact = read_pact_from_file(&path)?;
            let consumer_name = pact.consumer.name.clone();
            let provider_name = pact.provider.name.clone();
            pacts.insert((consumer_name, provider_name), pact);
        }
    }
    Ok(pacts)
}

#[cfg_attr(feature = "flame_it", flame)]
pub fn add_interaction_to_pact(
    pact_request: &HttpRequest,
    pact_response: &HttpResponse,
    pact: &mut V4Pact,
    interaction_index_map: &mut InteractionIndexMap,
) -> Result<(), Box<dyn Error>> {
    let interaction = SynchronousHttp {
        id: None,
        key: None,
        description: pact_request.path.clone(),
        provider_states: Vec::new(),
        request: pact_request.clone(),
        response: pact_response.clone(),
        comments: Default::default(),
        pending: false,
        plugin_config: Default::default(),
        interaction_markup: Default::default(),
        transport: None,
    };

    add_interaction_and_amend_index(pact_request, pact, interaction_index_map, &interaction)?;

    Ok(())
}

fn add_interaction_and_amend_index(
    pact_request: &HttpRequest,
    pact: &mut V4Pact,
    interaction_index_map: &mut InteractionIndexMap,
    interaction: &SynchronousHttp,
) -> Result<(), Box<dyn Error>> {
    let new_item_index = pact.interactions.len() as u16;
    pact.add_interaction(interaction)?;
    interaction_index_map
        .get_mut(&(pact.consumer.name.clone(), pact.provider.name.clone()))
        .unwrap()
        .insert(pact_request.path.clone(), new_item_index);
    assert!(pact.interactions.len() as u16 == new_item_index + 1);
    Ok(())
}

#[cfg_attr(feature = "flame_it", flame)]
pub fn save_pact(pact: &V4Pact, pacts_folder: &Path) -> Result<(), Box<dyn Error>> {
    let pact_path = derive_pact_file_path(pacts_folder, pact);
    save_pact_to_file(pact, &pact_path)?;
    debug!("Pact saved to file: {:?}", pact_path);
    Ok(())
}

#[cfg_attr(feature = "flame_it", flame)]
pub fn get_consumer_provider(
    pact_request: &HttpRequest,
) -> Result<(String, String), Box<dyn Error>> {
    let consumer_name = CONSUMER_NAME;
    let provider_url = Url::parse(&pact_request.path)?;
    let provider_name = provider_url.host_str().unwrap();
    Ok((consumer_name.to_string(), provider_name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pact_models::bodies::OptionalBody;
    use pact_models::v4::pact::V4Pact;
    use pact_models::{Consumer, Provider};

    #[test]
    #[test_log::test(test)]
    fn test_add_interaction_to_pact() {
        let pact_request = HttpRequest {
            method: "GET".to_string(),
            path: "/test".to_string(),
            query: None,
            headers: None,
            body: OptionalBody::Empty,
            ..Default::default()
        };
        let pact_response = HttpResponse {
            status: 200,
            headers: None,
            ..Default::default()
        };
        let mut pact = V4Pact {
            consumer: Consumer {
                name: "consumer".to_string(),
            },
            provider: Provider {
                name: "provider".to_string(),
            },
            ..Default::default()
        };
        let mut interaction_index_map = InteractionIndexMap::new();
        interaction_index_map.insert(
            ("consumer".to_string(), "provider".to_string()),
            HashMap::new(),
        );
        let _ = add_interaction_to_pact(
            &pact_request,
            &pact_response,
            &mut pact,
            &mut interaction_index_map,
        );
        assert_eq!(pact.interactions().len(), 1);
    }
}
