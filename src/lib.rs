#![allow(warnings)]

use reqwest::{Error, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::fmt::Debug;
use url::Url;

use constants::{Method, AUTH};
use errors::UrlParseError;
use utils::check_uri;

use crate::constants::Response;
use crate::errors::RequestError;
use crate::params::Params;

mod constants;
mod errors;
mod utils;
mod event_source;
mod params;

#[derive(Debug)]
pub struct Firebase {
    uri: Url,
}

impl Firebase {
    pub fn new(uri: String) -> Result<Self, UrlParseError>
        where
            Self: Sized,
    {
        match check_uri(&uri) {
            Ok(uri) => Ok(Self { uri }),
            Err(err) => Err(err),
        }
    }

    pub fn auth(uri: String, auth_key: String) -> Result<Self, UrlParseError>
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

    pub fn with_params(&self) -> Params {
        let uri = self.uri.clone();
        Params::new(uri)
    }

    pub fn at(&mut self, path: &str) -> Self {
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

        self.uri
            .set_path(format!("{}.json", new_path.as_str()).as_str());

        Self {
            uri: self.uri.clone(),
        }
    }

    pub fn get_uri(&self) -> String {
        self.uri.to_string()
    }

    pub async fn request(
        &self,
        method: Method,
        data: Option<Value>,
    ) -> Result<Response, RequestError> {
        let client = reqwest::Client::new();

        match method {
            Method::GET => {
                let request = client.get(self.uri.to_string()).send().await;
                return match request {
                    Ok(response) => {
                        if response.status() == StatusCode::from_u16(200).unwrap() {
                            return match response.text().await {
                                Ok(data) => {
                                    if data == String::from("null") {
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
                };
            }
            Method::POST => {
                if !data.is_some() {
                    return Err(RequestError::SerializeError);
                }

                let request = client.post(self.uri.to_string()).json(&data).send().await;
                return match request {
                    Ok(response) => {
                        let data = response.text().await.unwrap();
                        Ok(Response { data })
                    }
                    Err(_) => Err(RequestError::NetworkError),
                };
            }
            Method::PUT => {
                if !data.is_some() {
                    return Err(RequestError::SerializeError);
                }

                let request = client.put(self.uri.to_string()).json(&data).send().await;
                return match request {
                    Ok(response) => {
                        let data = response.text().await.unwrap();
                        Ok(Response { data })
                    }
                    Err(_) => Err(RequestError::NetworkError),
                };
            }
            Method::DELETE => {
                let request = client.delete(self.uri.to_string()).send().await;
                return match request {
                    Ok(_) => Ok(Response {
                        data: String::default(),
                    }),
                    Err(_) => Err(RequestError::NetworkError),
                };
            }
            _ => {}
        }

        Err(RequestError::NetworkError)
    }

    pub async fn request_generic<T>(&self, method: Method) -> Result<T, RequestError>
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

    pub async fn set<T>(&self, data: &T) -> Result<Response, RequestError>
        where
            T: Serialize + DeserializeOwned + Debug,
    {
        let data = serde_json::to_value(&data).unwrap();
        self.request(Method::POST, Some(data)).await
    }

    pub async fn get(&self) -> Result<Response, RequestError> {
        self.request(Method::GET, None).await
    }

    pub async fn get_generic<T>(&self) -> Result<T, RequestError>
        where
            T: Serialize + DeserializeOwned + Debug,
    {
        self.request_generic::<T>(Method::GET).await
    }

    pub async fn delete(&self) -> Result<Response, RequestError> {
        self.request(Method::DELETE, None).await
    }

    pub async fn update<T>(&self, data: &T) -> Result<Response, RequestError>
        where
            T: DeserializeOwned + Serialize + Debug,
    {
        let value = serde_json::to_value(&data).unwrap();
        self.request(Method::PUT, Some(value)).await
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::num::NonZeroU16;
    use url::Url;

    use crate::{Firebase, Method, UrlParseError};
    use crate::event_source::EventSource;

    const URI: &str = "https://firebase_id.firebaseio.com";
    const URI_WITH_SLASH: &str = "https://firebase_id.firebaseio.com/";
    const URI_NON_HTTPS: &str = "http://firebase_id.firebaseio.com/";

    #[tokio::test]
    async fn simple() {
        let firebase = Firebase::new(URI.to_string()).unwrap();
        assert_eq!(URI_WITH_SLASH.to_string(), firebase.get_uri());
    }

    #[tokio::test]
    async fn non_https() {
        let firebase = Firebase::new(URI_NON_HTTPS.to_string()).map_err(|e| e.to_string());
        assert_eq!(
            firebase.err(),
            Some(String::from(UrlParseError::NotHttps.to_string()))
        );
    }

    #[tokio::test]
    async fn with_auth() {
        let firebase = Firebase::auth(URI.to_string(), String::from("auth_key")).unwrap();
        assert_eq!(
            format!("{}/?auth=auth_key", URI.to_string()),
            firebase.get_uri()
        );
    }

    #[tokio::test]
    async fn at() {
        let firebase = Firebase::new(URI.to_string())
            .unwrap()
            .at("movies/movie1.json");
        assert_eq!(format!("{}/movies/movie1.json", URI), firebase.get_uri());
    }

    #[tokio::test]
    async fn test_events() {
        let mut event_source =
            EventSource::new(Url::parse(URI).unwrap());

        event_source
            .register_event("/user", || println!("OK"))
            .await;
        event_source.listen().await;
    }
}
