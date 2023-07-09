use std::fmt::Debug;
use eventsource_client::*;
use futures_util::TryStreamExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::ServerEventError;

pub struct ServerEvents {
    client: ClientBuilder,
}

impl ServerEvents {
    pub fn new(url: &str) -> Option<Self> {
        let mut client = ClientBuilder::for_url(url);

        match client {
            Ok(stream_connection) => {
                Some(ServerEvents { client: stream_connection })
            }
            Err(_) => {
                None
            }
        }
    }

    pub async fn listen(self, stream_event: fn(String, Option<String>), stream_err: fn(Error), keep_alive_friendly: bool) {
        let mut stream =
            self.client
                .build()
                .stream()
                .map_ok(|event| {
                    match event {
                        SSE::Event(ev) => {
                            if ev.event_type == "keep-alive" && !keep_alive_friendly {
                                return;
                            }

                            if ev.data == "null" {
                                stream_event(ev.event_type, None);
                                return;
                            }

                            stream_event(ev.event_type, Some(ev.data));
                        }
                        SSE::Comment(_) => {}
                    }
                })
                .map_err(|err| {
                    stream_err(err)
                });

        while let Ok(Some(_)) = stream.try_next().await {}
    }
}