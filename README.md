# firebase-rs

Rust based firebase library.

# Full Documentation
[Documentation](https://docs.rs/firebase-rs/1.0.1/firebase_rs/)

# How to use

### Load library
````rust
extern crate firebase_rs;
use firebase_rs::*;
````

#Creating a Firebase instance

####Without Auth
````rust
let _firebase = Firebase::new("https://myfirebase.firebaseio.com");
````

####With Auth
````rust
let _firebase = Firebase::auth("https://myfirebase.firebaseio.com", "AUTH_KEY");
````

####Reading Data
````rust
let users = _firebase.at("users").unwrap();
let res = users.get().unwrap();
````

####Writing Data
````rust
let users = _firebase.at("users").unwrap();
let res = users.set("{\"username\": \"test\"}").unwrap();
````

####Pushing Data
````rust
let users = _firebase.at("users").unwrap();
let res = users.push("{\"username\": \"test\"}").unwrap();
````

####Remove Data
````rust
let user = _firebase.at("users/user1").unwrap();
let res = users.delete("{\"user_id\": \"1\"}").unwrap();
````







