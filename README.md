# firebase-rs

Rust based firebase library.

# Full Documentation
[Documentation](https://docs.rs/firebase-rs/1.0.3/firebase_rs/)

[Firebase Struct](https://docs.rs/firebase-rs/1.0.3/firebase_rs/struct.Firebase.html)

# How to use

### Load library
````rust
extern crate firebase_rs;
use firebase_rs::*;
````

# Creating a Firebase instance

### Without Auth
````rust
let _firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap();
````

### With Auth
````rust
let _firebase = Firebase::auth("https://myfirebase.firebaseio.com", "AUTH_KEY").unwrap();
````

---

### Read Data
````rust
let users = _firebase.at("users").unwrap();
let res = users.get().unwrap();
````
### Read Data with Generic Type
````rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    username: String,
}
let users = _firebase.at("users").unwrap();
let res = users.get_generic::<User>().unwrap();

// Use this type if you use key value struct (e.g: "user1": {"username": "value"})
let res = users.get_generic::<HashMap<String, User>>().unwrap();
````
### Async
````rust
let job = users.get_async(|res| {
    println!("{:?}", res);
});
job.join();
````

---

### Set Data
````rust
let users = _firebase.at("users").unwrap();
let res = users.set("{\"username\": \"test\"}").unwrap();
````
### Set Data with Generic Type
````rust
let users = _firebase.at("users").unwrap();
let res = users.set_generic::<User>(User {username: "value".to_string()}).unwrap();
````
### Async
````rust
let job = users.set_async("{\"username\": \"test\"}", |res| {
    println!("{:?}", res);
});
job.join();
````

---

### Update Data
````rust
let users = _firebase.at("users/user1").unwrap();
let res = users.update("{\"username\": \"new_username\"}").unwrap();
````
### Async
````rust
let users = _firebase.at("users/user1").unwrap();
let job = users.update_async("{\"username\": \"new_username\"}", |res| {
    println!("{:?}", res);
});
job.join();
````

---

### Push Data
````rust
let users = _firebase.at("users").unwrap();
let res = users.push("{\"username\": \"test\"}").unwrap();
````
### Async
````rust
let job = users.push_async("{\"username\": \"test\"}", |res| {
    println!("{:?}", res);
});
job.join();
````

---

### Remove Data
````rust
let user = _firebase.at("users").unwrap();
let res = users.delete("{\"user_id\": \"1\"}").unwrap();
````
### Async
````rust
let user = _firebase.at("users").unwrap();
let job = users.delete_async("{\"user_id\": \"1\"}", |res| {
    println!("{:?}", res);
});
job.join();
````







