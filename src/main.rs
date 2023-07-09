use firebase_rs::Firebase;
use serde::{Serialize, Deserialize};
use eventsource_client::*;
use eventsource_client::SSE::Event;

#[derive(Serialize, Deserialize, Debug)]
struct Fixed {
    pub username: String,
}

#[tokio::main]
async fn main() {
    let es = Firebase::new("https://fir-rs-7af57.firebaseio.com/");

    match es {
        Ok(con) => {
            let stream = con.at("users").with_realtime_events().unwrap();
            stream
                .listen(|event_type, data| {
                    println!("Type: {:?} Data: {:?}", event_type, data);
                }, |err| println!("{:?}", err), false).await;
        }
        Err(_) => {}
    }
}