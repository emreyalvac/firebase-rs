extern crate firebase_rs;
extern crate serde;
extern crate serde_json;

use firebase_rs::Firebase;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    username: String,
}

fn main() {
    let mut _firebase = Firebase::new("https://fir-rs-7af57.firebaseio.com/").unwrap();
    _firebase = _firebase.at("users").unwrap();
    // let x = _firebase.get_generic::<HashMap<String, User>>().unwrap();
    // let x = _firebase.set("{\"username\": \"asdas\"}").unwrap();
    let x = _firebase.set_generic::<User>(User { username: "Emre".to_string() }).unwrap();
    println!("{:?}", x);
}
