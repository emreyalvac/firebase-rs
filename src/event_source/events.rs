use reqwest::{Url};
use std::collections::HashMap;
use std::time::Duration;
use futures::stream::StreamExt;
use eventsource::reqwest::Client;
use crate::Event;
use crate::event_source::RegisterError;

pub struct EventSource {
    events: HashMap<String, fn(e: Event)>,
}

impl EventSource {
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
        }
    }

    pub async fn register_event(
        &mut self,
        event_name: &str,
        event_fn: fn(Event),
    ) -> Result<bool, RegisterError> {
        if self.events.contains_key(event_name) {
            return Err(RegisterError::EventExist);
        }

        self.events.insert(event_name.to_string(), event_fn);
        Ok(true)
    }

    pub async fn listen_all_events(&self, event_fn: fn(e: Event)) {
        let client = Client::new(Url::parse("url").unwrap());
        for event in client {
            if let Ok(e) = event {
                event_fn(Event {
                    id: e.id,
                    event_type: e.event_type,
                    data: e.data,
                })
            };
        }
    }
}