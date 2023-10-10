use eventsource_client::*;
use futures_util::StreamExt;

pub struct ServerEvents {
    client: ClientBuilder,
}

impl ServerEvents {
    pub fn new(url: &str) -> Option<Self> {
        let mut client = ClientBuilder::for_url(url);

        match client {
            Ok(stream_connection) => Some(ServerEvents {
                client: stream_connection,
            }),
            Err(_) => None,
        }
    }

    pub async fn listen(
        self,
        stream_event: fn(String, Option<String>),
        stream_err: fn(Error),
        keep_alive_friendly: bool,
    ) {
        self.stream(keep_alive_friendly)
            .for_each(|event| {
                match event {
                    Ok((event_type, maybe_data)) => stream_event(event_type, maybe_data),
                    Err(x) => stream_err(x),
                }
                futures_util::future::ready(())
            })
            .await
    }

    pub fn stream(
        self,
        keep_alive_friendly: bool,
    ) -> std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<(String, Option<String>)>> + Send>>
    {
        return Box::pin(
            self.client
                .build()
                .stream()
                .filter_map(move |event| async move {
                    match event {
                        Ok(SSE::Event(ev)) => {
                            if ev.event_type == "keep-alive" && !keep_alive_friendly {
                                return None;
                            }

                            if ev.data == "null" {
                                return Some(Ok((ev.event_type, None)));
                            }

                            return Some(Ok((ev.event_type, Some(ev.data))));
                        }
                        Ok(SSE::Comment(_)) => return None,
                        Err(x) => Some(Err(x)),
                    }
                }),
        );
    }
}
