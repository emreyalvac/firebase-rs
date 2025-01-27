use constants::{Method, Response, AUTH};
use errors::{RequestResult, UrlParseResult};
use params::Params;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
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
        let uri = self.build_uri(path);

        Firebase { uri }
    }

    fn build_uri(&self, path: &str) -> Url {
        let mut new_path = String::new();

        if let Some(segments) = self.uri.path_segments() {
            for segment in segments {
                let clean_segment = segment.trim_end_matches(".json");
                new_path.push_str(clean_segment);
                new_path.push('/');
            }
        }

        new_path.push_str(path);

        let final_path = if new_path.ends_with(".json") {
            new_path.trim_end_matches(".json").to_string()
        } else {
            new_path
        };

        let mut uri = self.uri.clone();
        uri.set_path(&format!("{}.json", final_path));

        uri
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

    async fn request(&self, method: Method, data: Option<Value>, include_etag: bool, etag: Option<&str>) -> RequestResult<Response> {
        let client = reqwest::Client::new();
        let mut request_builder = match method {
            Method::GET => client.get(self.uri.to_string()),
            Method::PUT | Method::PATCH | Method::POST => {
                if data.is_none() {
                    return Err(RequestError::SerializeError);
                }
                let builder = if method == Method::PUT {
                    client.put(self.uri.to_string())
                } else if method == Method::POST {
                    client.post(self.uri.to_string())
                } else {
                    client.patch(self.uri.to_string())
                };
                builder.json(&data)
            }
            Method::DELETE => client.delete(self.uri.to_string()),
        };

        if include_etag {
            request_builder = request_builder.header("X-Firebase-ETag", "true");
        }

        if let Some(etag_value) = etag {
            request_builder = request_builder.header("if-match", etag_value);
        }

        let request = request_builder.send().await;

        match request {
            Ok(response) => {
                let etag = response.headers().get("ETag").map(|v| v.to_str().unwrap().to_string());
                match response.status() {
                    StatusCode::OK => {
                        let response_text = response.text().await.unwrap_or_default();
                        if response_text == "null" {
                            Err(RequestError::NotFoundOrNullBody)
                        } else {
                            Ok(Response { etag, data: response_text })
                        }
                    }
                    StatusCode::PRECONDITION_FAILED => {
                        // create a new response with the etag value and with a new response body
                        let response_text = response.text().await.unwrap_or_default();
                        Ok(Response {etag, data: response_text })
                    }
                    _ => Err(RequestError::NetworkError),
                }
            }
            Err(_) => Err(RequestError::NetworkError),
        }
    }

    pub async fn update_atomically_by(&self, value: i64, limit_min: Option<i64>, limit_max: Option<i64>, etag: Option<String>) -> RequestResult<Response> {
        Box::pin(async move {
            if let Some(etag) = etag {
                let data = json!(value);
                let updated_response = self.request(Method::PUT, Some(data), false, Some(etag).as_deref()).await;
                if let Ok(response) = &updated_response {
                    if response.etag.is_none() {
                        return updated_response;
                    }
                    let next_value = match Self::check_current_before_update(value, limit_min, limit_max, response.data.as_str()) {
                        Ok(value) => value,
                        Err(value) => return value,
                    };
                    return self.update_atomically_by(next_value, limit_min, limit_max, response.etag.clone()).await;
                }
                return updated_response;
            }
            let response = self.request(Method::GET, None, true, None).await;
            match response {
                Ok(response) => {
                    let new_value = match Self::check_current_before_update(value, limit_min, limit_max, response.data.as_str()) {
                        Ok(value) => value,
                        Err(value) => return value,
                    };
                    self.update_atomically_by(new_value, limit_min, limit_max, response.etag).await
                }
                Err(err) => Err(err),
            }
        }).await
    }

    fn check_current_before_update(value: i64, limit_min: Option<i64>, limit_max: Option<i64>, data: &str) -> Result<i64, RequestResult<Response>> {
        let new_value: i64 = serde_json::from_str(data).unwrap();
        if let Some(limit) = limit_max {
            if (value > 0 && new_value >= limit) || (value < 0 && new_value <= limit) {
                return Err(Err(RequestError::LimitExceeded));
            }
        }
        if let Some(limit) = limit_min {
            if new_value == limit {
                return Err(Err(RequestError::LimitExceeded));
            }
        }
        let next_value = new_value + value;
        Ok(next_value)
    }

    async fn request_generic<T>(&self, method: Method) -> RequestResult<T>
    where
        T: Serialize + DeserializeOwned + Debug,
    {
        let request = self.request(method, None, false, None).await;

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
        self.request(Method::POST, Some(data), false, None).await
    }

    pub async fn post_json(&self, data: serde_json::Value) -> RequestResult<Response>
    {
        self.request(Method::POST, Some(data), false, None).await
    }

    pub async fn put_json(&self, data: serde_json::Value) -> RequestResult<Response>
    {
        self.request(Method::PUT, Some(data), false, None).await
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
    /// let mut firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap().at("users");
    /// let users = firebase.set_with_key("myKey", &user).await;
    /// # }
    /// ```
    pub async fn set_with_key<T>(&mut self, key: &str, data: &T) -> RequestResult<Response>
    where
        T: Serialize + DeserializeOwned + Debug,
    {
        self.uri = self.build_uri(key);
        let data = serde_json::to_value(&data).unwrap();

        self.request(Method::PUT, Some(data), false, None).await
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
        self.request(Method::GET, None, false, None).await
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
        self.request(Method::DELETE, None, false, None).await
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
        self.request(Method::PATCH, Some(value), false, None).await
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
