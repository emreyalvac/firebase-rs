use std::collections::HashMap;
use reqwest::Url;
use crate::event_source::RegisterError;

pub struct EventSource {
    url: Url,
    events: HashMap<String, fn()>,
}

impl EventSource {
    pub fn new(url: Url) -> Self {
        Self {
            url,
            events: HashMap::new(),
        }
    }

    pub async fn register_event(&mut self, event_name: &str, event_fn: fn()) -> Result<bool, RegisterError> {
        if self.events.contains_key(event_name) {
            return Err(RegisterError::EventExist);
        }

        self.events.insert(event_name.to_string(), event_fn);

        Ok(true)
    }

    pub async fn listen(&self) {
        todo!()
    }
}
