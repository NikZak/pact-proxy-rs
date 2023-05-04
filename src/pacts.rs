use crate::pact::edit;
use crate::server::InteractionIndexMap;
use pact_models::prelude::v4::V4Pact;
use pact_models::v4::http_parts::{HttpRequest, HttpResponse};
use pact_models::{Consumer, Provider};
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct Pacts {
    pacts: Arc<Mutex<HashMap<(String, String), V4Pact>>>,
    interaction_index_map: Arc<Mutex<InteractionIndexMap>>,
    pacts_folder: PathBuf,
}

impl Pacts {
    pub fn new(
        pacts: Arc<Mutex<HashMap<(String, String), V4Pact>>>,
        interaction_index_map: Arc<Mutex<InteractionIndexMap>>,
        pacts_folder: PathBuf,
    ) -> Pacts {
        Pacts {
            pacts,
            interaction_index_map,
            pacts_folder,
        }
    }

    #[cfg_attr(feature = "flame_it", flame)]
    pub fn add_interaction(
        &self,
        consumer: &str,
        provider: &str,
        request: &HttpRequest,
        response: &HttpResponse,
    ) -> Result<(), Box<dyn Error>> {
        let mut pacts = self.pacts.lock().unwrap();
        let mut interaction_index_map = self.interaction_index_map.lock().unwrap();
        let pact = pacts
            .entry((consumer.to_string(), provider.to_string()))
            .or_insert(Self::default_empty_pact(consumer, provider));
        interaction_index_map
            .entry((consumer.to_string(), provider.to_string()))
            .or_insert(HashMap::new());
        edit::add_interaction_to_pact(request, response, pact, &mut interaction_index_map)?;
        Ok(())
    }

    #[cfg_attr(feature = "flame_it", flame)]
    fn default_empty_pact(consumer: &str, provider: &str) -> V4Pact {
        let consumer = Consumer {
            name: consumer.to_string(),
        };
        let provider = Provider {
            name: provider.to_string(),
        };
        V4Pact {
            consumer,
            provider,
            interactions: vec![],
            metadata: Default::default(),
            plugin_data: vec![],
        }
    }

    #[cfg_attr(feature = "flame_it", flame)]
    pub fn get_pact_response(
        &self,
        consumer: &str,
        provider: &str,
        interaction_descr: &str,
    ) -> Option<HttpResponse> {
        let pacts = self.pacts.lock().unwrap();
        let interaction_index_map = self.interaction_index_map.lock().unwrap();
        let pact = pacts.get(&(consumer.to_string(), provider.to_string()))?;
        let interaction_index = interaction_index_map
            .get(&(consumer.to_string(), provider.to_string()))?
            .get(interaction_descr)?;
        let interaction = pact.interactions.get(*interaction_index as usize)?;
        let interaction_json = interaction.to_json();
        let pact_response = HttpResponse::from_json(interaction_json.get("response")?).ok()?;
        Some(pact_response)
    }

    pub fn get_folder(&self) -> PathBuf {
        self.pacts_folder.clone()
    }

    #[cfg_attr(feature = "flame_it", flame)]
    pub fn save_pact(&self, consumer: &str, provider: &str) -> Result<(), Box<dyn Error>> {
        let pacts = self.pacts.lock().unwrap();
        let pact = pacts
            .get(&(consumer.to_string(), provider.to_string()))
            .unwrap();
        edit::save_pact(pact, &self.get_folder())?;
        Ok(())
    }
}
