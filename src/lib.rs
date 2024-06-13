use constants::{Method, Response, AUTH};
use errors::{RequestResult, UrlParseResult};
use params::Params;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::fmt::Debug;
use url::Url;
use utils::check_uri;

use crate::sse::ServerEvents;

pub use errors::{RequestError, ServerEventError, UrlParseError};

mod constants;
mod errors;
mod params;
mod sse;
mod utils;

#[derive(Debug)]
pub struct Firebase {
    uri: Url,
}

impl Firebase {
    /// ```rust
    /// use firebase_rs::Firebase;
    ///
    /// let firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap();
    /// ```
    pub fn new(uri: &str) -> UrlParseResult<Self>
    where
        Self: Sized,
    {
        match check_uri(&uri) {
            Ok(uri) => Ok(Self { uri }),
            Err(err) => Err(err),
        }
    }

    /// ```rust
    /// const URI: &str = "...";
    /// const AUTH_KEY: &str = "...";
    ///
    /// use firebase_rs::Firebase;
    ///
    /// let firebase = Firebase::auth("https://myfirebase.firebaseio.com", AUTH_KEY).unwrap();
    /// ```
    pub fn auth(uri: &str, auth_key: &str) -> UrlParseResult<Self>
    where
        Self: Sized,
    {
        match check_uri(&uri) {
            Ok(mut uri) => {
                uri.set_query(Some(&format!("{}={}", AUTH, auth_key)));
                Ok(Self { uri })
            }
            Err(err) => Err(err),
        }
    }

    /// ```rust
    /// use firebase_rs::Firebase;
    ///
    /// # async fn run() {
    /// let firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap().with_params().start_at(1).order_by("name").equal_to(5).finish();
    /// let result = firebase.get::<String>().await;
    /// # }
    /// ```
    pub fn with_params(&self) -> Params {
        let uri = self.uri.clone();
        Params::new(uri)
    }

    /// To use simple interface with synchronous callbacks, pair with `.listen()`:
    /// ```rust
    /// use firebase_rs::Firebase;
    /// # async fn run() {
    /// let firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap().at("users");
    /// let stream = firebase.with_realtime_events().unwrap();
    /// stream.listen(|event_type, data| {
    ///                     println!("{:?} {:?}" ,event_type, data);
    ///                 }, |err| println!("{:?}" ,err), false).await;
    /// # }
    /// ```
    ///
    /// To use streaming interface for async code, pair with `.stream()`:
    /// ```rust
    /// use firebase_rs::Firebase;
    /// use futures_util::StreamExt;
    ///
    /// # async fn run() {
    /// let firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap().at("users");
    /// let stream = firebase.with_realtime_events()
    ///              .unwrap()
    ///              .stream(true);
    /// stream.for_each(|event| {
    ///           match event {
    ///               Ok((event_type, maybe_data)) => println!("{:?} {:?}" ,event_type, maybe_data),
    ///               Err(err) => println!("{:?}" ,err),
    ///           }
    ///           futures_util::future::ready(())
    ///        }).await;
    /// # }
    /// ```
    pub fn with_realtime_events(&self) -> Option<ServerEvents> {
        ServerEvents::new(self.uri.as_str())
    }

    /// ```rust
    /// use firebase_rs::Firebase;
    ///
    /// let firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap().at("users").at("USER_ID").at("f69111a8a5258c15286d3d0bd4688c55");
    /// ```
    pub fn at(&self, path: &str) -> Self {
        let mut new_path = String::default();

        let paths = self.uri.path_segments().map(|p| p.collect::<Vec<_>>());
        for mut path in paths.unwrap() {
            if path.find(".json").is_some() {
                path = path.trim_end_matches(".json");
            }
            new_path += format!("{}/", path).as_str();
        }

        new_path += path;

        if new_path.find(".json").is_some() {
            new_path = new_path.trim_end_matches(".json").to_string();
        }

        let mut uri = self.uri.clone();
        uri.set_path(&format!("{}.json", new_path));
        Self { uri }
    }

    /// ```rust
    /// use firebase_rs::Firebase;
    ///
    /// let firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap().at("users");
    /// let uri = firebase.get_uri();
    /// ```
    pub fn get_uri(&self) -> String {
        self.uri.to_string()
    }

    async fn request(&self, method: Method, data: Option<Value>) -> RequestResult<Response> {
        let client = reqwest::Client::new();

        return match method {
            Method::GET => {
                let request = client.get(self.uri.to_string()).send().await;
                match request {
                    Ok(response) => {
                        if response.status() == StatusCode::from_u16(200).unwrap() {
                            return match response.text().await {
                                Ok(data) => {
                                    if data.as_str() == "null" {
                                        return Err(RequestError::NotFoundOrNullBody);
                                    }
                                    return Ok(Response { data });
                                }
                                Err(_) => Err(RequestError::NotJSON),
                            };
                        } else {
                            Err(RequestError::NetworkError)
                        }
                    }
                    Err(_) => return Err(RequestError::NetworkError),
                }
            }
            Method::POST => {
                if !data.is_some() {
                    return Err(RequestError::SerializeError);
                }

                let request = client.post(self.uri.to_string()).json(&data).send().await;
                match request {
                    Ok(response) => {
                        let data = response.text().await.unwrap();
                        Ok(Response { data })
                    }
                    Err(_) => Err(RequestError::NetworkError),
                }
            }
            Method::PATCH => {
                if !data.is_some() {
                    return Err(RequestError::SerializeError);
                }

                let request = client.patch(self.uri.to_string()).json(&data).send().await;
                match request {
                    Ok(response) => {
                        let data = response.text().await.unwrap();
                        Ok(Response { data })
                    }
                    Err(_) => Err(RequestError::NetworkError),
                }
            }
            Method::DELETE => {
                let request = client.delete(self.uri.to_string()).send().await;
                match request {
                    Ok(_) => Ok(Response {
                        data: String::default(),
                    }),
                    Err(_) => Err(RequestError::NetworkError),
                }
            }
        };
    }

    async fn request_generic<T>(&self, method: Method) -> RequestResult<T>
    where
        T: Serialize + DeserializeOwned + Debug,
    {
        let request = self.request(method, None).await;

        match request {
            Ok(response) => {
                let data: T = serde_json::from_str(response.data.as_str()).unwrap();

                Ok(data)
            }
            Err(err) => Err(err),
        }
    }

    /// ```rust
    /// use firebase_rs::Firebase;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize, Debug)]
    /// struct User {
    ///    name: String
    /// }
    ///
    /// # async fn run() {
    /// let user = User { name: String::default() };
    /// let firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap().at("users");
    /// let users = firebase.set(&user).await;
    /// # }
    /// ```
    pub async fn set<T>(&self, data: &T) -> RequestResult<Response>
    where
        T: Serialize + DeserializeOwned + Debug,
    {
        let data = serde_json::to_value(&data).unwrap();
        self.request(Method::POST, Some(data)).await
    }

    /// ```rust
    /// use std::collections::HashMap;
    /// use firebase_rs::Firebase;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize, Debug)]
    /// struct User {
    ///    name: String
    /// }
    ///
    /// # async fn run() {
    /// let firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap().at("users");
    /// let users = firebase.get::<HashMap<String, User>>().await;
    /// # }
    /// ```
    pub async fn get_as_string(&self) -> RequestResult<Response> {
        self.request(Method::GET, None).await
    }

    /// ```rust
    /// use std::collections::HashMap;
    /// use firebase_rs::Firebase;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize, Debug)]
    /// struct User {
    ///     name: String
    /// }
    ///
    /// # async fn run() {
    /// let firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap().at("users").at("USER_ID");
    /// let user = firebase.get::<User>().await;
    ///
    ///  // OR
    ///
    /// let firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap().at("users");
    /// let user = firebase.get::<HashMap<String, User>>().await;
    /// # }
    /// ```
    pub async fn get<T>(&self) -> RequestResult<T>
    where
        T: Serialize + DeserializeOwned + Debug,
    {
        self.request_generic::<T>(Method::GET).await
    }

    /// ```rust
    /// use firebase_rs::Firebase;
    ///
    /// # async fn run() {
    /// let firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap().at("users").at("USER_ID");
    /// firebase.delete().await;
    /// # }
    /// ```
    pub async fn delete(&self) -> RequestResult<Response> {
        self.request(Method::DELETE, None).await
    }

    /// ```rust
    /// use firebase_rs::Firebase;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize, Debug)]
    /// struct User {
    ///     name: String
    /// }
    ///
    /// # async fn run() {
    /// let user = User { name: String::default() };
    /// let firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap().at("users").at("USER_ID");
    /// let users = firebase.update(&user).await;
    /// # }
    /// ```
    pub async fn update<T>(&self, data: &T) -> RequestResult<Response>
    where
        T: DeserializeOwned + Serialize + Debug,
    {
        let value = serde_json::to_value(&data).unwrap();
        self.request(Method::PATCH, Some(value)).await
    }
}

#[cfg(test)]
mod tests {
    use crate::{Firebase, UrlParseError};

    const URI: &str = "https://firebase_id.firebaseio.com";
    const URI_WITH_SLASH: &str = "https://firebase_id.firebaseio.com/";
    const URI_NON_HTTPS: &str = "http://firebase_id.firebaseio.com/";

    #[tokio::test]
    async fn simple() {
        let firebase = Firebase::new(URI).unwrap();
        assert_eq!(URI_WITH_SLASH.to_string(), firebase.get_uri());
    }

    #[tokio::test]
    async fn non_https() {
        let firebase = Firebase::new(URI_NON_HTTPS).map_err(|e| e.to_string());
        assert_eq!(
            firebase.err(),
            Some(String::from(UrlParseError::NotHttps.to_string()))
        );
    }

    #[tokio::test]
    async fn with_auth() {
        let firebase = Firebase::auth(URI, "auth_key").unwrap();
        assert_eq!(
            format!("{}/?auth=auth_key", URI.to_string()),
            firebase.get_uri()
        );
    }

    #[tokio::test]
    async fn with_sse_events() {
        // TODO: SSE Events Test
    }
}
