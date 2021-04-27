/*
Firebase REST API
*/

extern crate curl;
extern crate serde;
extern crate serde_json;
extern crate url;

use curl::easy::{Easy2, Handler, List, WriteError};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use std::thread::JoinHandle;
use url::Url;

// Collector
pub struct Collector(Vec<u8>);

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

/**
Constants
*/
const AUTH: &str = "auth";
const ORDER_BY: &'static str = "orderBy";
const LIMIT_TO_FIRST: &'static str = "limitToFirst";
const LIMIT_TO_LAST: &'static str = "limitToLast";
const START_AT: &'static str = "startAt";
const END_AT: &'static str = "endAt";
const EQUAL_TO: &'static str = "equalTo";
const SHALLOW: &'static str = "shallow";
const FORMAT: &'static str = "format";
const EXPORT: &'static str = "export";

#[derive(Clone, Debug)]
pub struct Firebase {
    url: Arc<Url>,
}

/**
Url Parse Error
*/
#[derive(Debug)]
pub enum UrlParseError {
    NoPath,
    NotHttps,
    Parser(url::ParseError),
}

impl Display for UrlParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UrlParseError::NoPath => write!(f, "URL path is missing."),
            UrlParseError::NotHttps => write!(f, "The URL protocol should be https."),
            UrlParseError::Parser(e) => write!(f, "Error while parsing the URL: {}", e),
        }
    }
}

impl std::error::Error for UrlParseError {}

#[derive(Debug)]
pub enum RequestError {
    NotJSON,
    NoUTF8(std::str::Utf8Error),
    NetworkError(curl::Error),
}

impl Display for RequestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestError::NotJSON => write!(f, "Invalid JSON"),
            RequestError::NoUTF8(utf8_error) => write!(f, "Utf8 error: {}", utf8_error),
            RequestError::NetworkError(curl_error) => write!(f, "Curl error: {}", curl_error),
        }
    }
}

impl std::error::Error for RequestError {}

#[derive(Debug)]
enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

#[derive(Debug)]
pub struct Response {
    pub body: String,
    pub code: u32,
}

#[derive(Debug)]
pub struct ResponseGeneric<T> {
    pub data: T,
    pub code: i32,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            body: Default::default(),
            code: Default::default(),
        }
    }
}

impl Firebase {
    fn request(&self, method: Method, data: Option<&str>) -> Result<Response, RequestError> {
        Firebase::request_url(&self.url, method, data)
    }

    fn request_url(
        url: &Url,
        method: Method,
        data: Option<&str>,
    ) -> Result<Response, RequestError> {
        let mut handler = Easy2::new(Collector(Vec::new()));
        handler.url(url.clone().as_str()).unwrap();

        let _req = match method {
            Method::GET => handler.get(true).unwrap(),
            Method::POST => handler.post(true).unwrap(),
            Method::PUT => {
                handler.custom_request("PUT").unwrap();
            }
            Method::DELETE => {
                handler.custom_request("DELETE").unwrap();
            }
            Method::PATCH => {
                handler.custom_request("PATCH").unwrap();
            }
        };

        if !data.is_none() {
            // header
            let mut list = List::new();
            list.append("Content-Type: application/json").unwrap();
            let mut _data = data.unwrap().as_bytes();
            handler.post_field_size(_data.len() as u64).unwrap();
            handler.post_fields_copy(_data).unwrap();
            handler.http_headers(list).unwrap();
            let response_code = handler.response_code().unwrap();
            let contents = handler.get_ref();
            match handler.perform() {
                Ok(_) => {
                    if response_code == 0 || response_code == 200 {
                        Ok(Response {
                            body: String::from_utf8_lossy(&contents.0).to_string(),
                            code: handler.response_code().unwrap(),
                        })
                    } else {
                        Err(RequestError::NetworkError(curl::Error::new(400)))
                    }
                }
                Err(e) => Err(RequestError::NetworkError(e)),
            }
        } else {
            let contents = handler.get_ref();
            match handler.perform() {
                Ok(_) => Ok(Response {
                    body: String::from_utf8_lossy(&contents.0).to_string(),
                    code: handler.response_code().unwrap(),
                }),
                Err(e) => Err(RequestError::NetworkError(e)),
            }
        }
    }

    fn request_url_async<F>(
        url: &Arc<Url>,
        method: Method,
        data: Option<String>,
        callback: F,
    ) -> JoinHandle<()>
    where
        F: Fn(Result<Response, RequestError>) + Send + 'static,
    {
        let url = url.clone();
        std::thread::spawn(move || {
            callback(Firebase::request_url(
                &url,
                method,
                data.as_ref().map(|s| s as &str),
            ))
        })
    }

    /// Creates a Firebase reference
    /// # Failures
    /// - If a url is not HTTPS, UrlParseError::NotHttps
    /// - If a url cannot be parsed into a valid url, UrlParseError::Parser(curl::Error)
    /// # Examples
    /// ```
    /// let mut _firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap();
    pub fn new(url: &str) -> Result<Self, UrlParseError> {
        let url = match url::Url::parse(url) {
            Ok(res) => res,
            Err(e) => return Err(UrlParseError::Parser(e)),
        };
        if url.scheme() != "https" {
            return Err(UrlParseError::NotHttps);
        }
        Ok(Self { url: Arc::new(url) })
    }

    /// Creates a new authenticated Firebase reference.
    /// # Failures
    /// - If a url is not HTTPS, UrlParseError::NotHttps
    /// - If a url cannot be parsed into a valid url, UrlParseError::Parser(curl::Error)
    /// # Examples
    /// ```
    ///  let mut _firebase = Firebase::auth("https://myfirebase.firebaseio.com", "AUTH_KEY").unwrap();
    /// The url will be https://myfirebase.firebaseio.com/?auth=AUTH_KEY
    pub fn auth(url: &str, auth: &str) -> Result<Self, UrlParseError> {
        let mut url = match url::Url::parse(url) {
            Ok(res) => res,
            Err(e) => return Err(UrlParseError::Parser(e)),
        };
        if url.scheme() != "https" {
            return Err(UrlParseError::NotHttps);
        }
        url.set_query(Some(format!("{}={}", AUTH, auth).as_ref()));
        Ok(Self { url: Arc::new(url) })
    }

    /// Returns current URL
    pub fn get_url(&self) -> Url {
        (*self.url).clone()
    }

    /// Creates a new Firebase instance with path.
    /// # Examples
    /// ```
    /// let mut _firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap();
    /// // The URL will be: https://myfirebase.firebaseio.com/movies.json
    /// let movies = _firebase.at("movies").unwrap();
    /// OR
    /// let movies = _firebase.at("movies/movie1").unwrap();
    pub fn at(&self, mut add_path: &str) -> Result<Self, UrlParseError> {
        let url = (*self.url).clone();
        // Remove '.json' in new path
        add_path = add_path.trim_end_matches(".json");
        add_path = add_path.trim_matches('/');
        let mut _url = url.to_string();
        _url = _url.trim_end_matches(".json").to_string();
        _url = _url.trim_matches('/').to_string();
        _url = format!("{}/{}.json", &_url, add_path);
        Ok(Self {
            url: Arc::new(Url::parse(_url.as_str()).unwrap()),
        })
    }

    /// Sets data to Firebase
    /// # Examples
    /// ```
    /// let _firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap();
    /// let users = _firebase.at("users").unwrap();
    /// users.set("{\"username\":\"test\"}").unwrap();
    pub fn set(&self, data: &str) -> Result<Response, RequestError> {
        self.request(Method::PUT, Some(data))
    }

    /// Asynchronous method for set.
    /// Takes a callback function and returns a handle.
    /// # Examples
    /// ```
    ///  let _firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap();
    ///  let mut users = _firebase.at("users").unwrap();
    ///  users = users.at("user_1").unwrap();
    ///  let job = users.set_async("{\"username\":\"new_username\"}", |res| {
    ///      println!("{:?}", res);
    ///  });
    ///  job.join();
    pub fn set_async<S, F>(&self, data: S, callback: F) -> JoinHandle<()>
    where
        F: Fn(Result<Response, RequestError>) + Send + 'static,
        S: Into<String>,
    {
        Firebase::request_url_async(&self.url, Method::PUT, Some(data.into()), callback)
    }

    // Gets data from Firebase with generic type
    /// # Examples
    ///  ```
    /// use serde::{Serialize, Deserialize};
    /// #[derive(Debug, Serialize, Deserialize)]
    /// pub struct User {
    ///     username: String
    /// }
    /// let _firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap();
    /// let users = _firebase.at("users").unwrap();
    /// let res = users.set_generic::<User>(User {username: "value".to_string()}).unwrap();
    pub fn set_generic<T>(&self, data: T) -> Result<ResponseGeneric<T>, RequestError>
    where
        T: DeserializeOwned + Serialize + Sized + Debug,
    {
        let _data = serde_json::to_string(&data).unwrap();
        match self.set(&_data) {
            Ok(_) => Ok(ResponseGeneric {
                data: serde_json::from_str::<T>(&_data).unwrap(),
                code: 200,
            }),
            Err(e) => Err(e),
        }
    }

    /// Gets data from Firebase with generic type
    /// # Examples
    ///  ```
    /// use serde::{Serialize, Deserialize};
    /// #[derive(Debug, Serialize, Deserialize)]
    /// pub struct User {
    ///     username: String
    /// }
    /// let _firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap();
    /// let users = _firebase.at("users").unwrap();
    /// let res = users.get_generic::<User>().unwrap();
    ///
    /// // Use this type if you use key value struct (e.g: "user1": {"username": "value"})
    /// let res = users.get_generic::<HashMap<String, User>().unwrap();
    pub fn get_generic<T>(&self) -> Result<ResponseGeneric<T>, RequestError>
    where
        T: DeserializeOwned,
    {
        match self.get() {
            Ok(res) => Ok(ResponseGeneric {
                data: serde_json::from_str::<T>(res.body.as_str()).unwrap(),
                code: 200,
            }),
            Err(e) => Err(e),
        }
    }

    /// Gets data from Firebase
    /// # Examples
    /// ```
    /// let _firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap();
    /// let users = _firebase.at("users").unwrap();
    /// let res = users.get().unwrap();
    pub fn get(&self) -> Result<Response, RequestError> {
        self.request(Method::GET, None)
    }

    /// Asynchronous method for get.
    /// Takes a callback function and returns a handle.
    /// # Examples
    /// ```
    ///  let _firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap();
    ///  let mut users = _firebase.at("users").unwrap();
    ///  users = users.at("user_1").unwrap();
    ///  let job = users.get_async(|res| {
    ///      println!("{:?}", res);
    ///  });
    ///  job.join();
    pub fn get_async<F>(&self, callback: F) -> JoinHandle<()>
    where
        F: Fn(Result<Response, RequestError>) + Send + 'static,
    {
        Firebase::request_url_async(&self.url, Method::GET, None, callback)
    }

    /// Push data to Firebase
    /// # Examples
    /// ```
    /// let _firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap();
    /// let users = _firebase.at("users").unwrap();
    /// users.push("{\"username\":\"test\"}").unwrap();
    pub fn push(&self, data: &str) -> Result<Response, RequestError> {
        self.request(Method::POST, Some(data))
    }

    /// Asynchronous method for push.
    /// Takes a callback function and returns a handle.
    /// # Examples
    /// ```
    ///  let _firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap();
    ///  let mut users = _firebase.at("users").unwrap();
    ///  users = users.at("user_1").unwrap();
    ///  let job = users.push_async("{\"username\":\"new_username\"}", |res| {
    ///      println!("{:?}", res);
    ///  });
    ///  job.join();
    pub fn push_async<S, F>(&self, data: S, callback: F) -> JoinHandle<()>
    where
        F: Fn(Result<Response, RequestError>) + Send + 'static,
        S: Into<String>,
    {
        Firebase::request_url_async(&self.url, Method::POST, Some(data.into()), callback)
    }

    /// Delete data from Firebase
    /// # Examples
    /// ```
    /// let _firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap();
    /// let users = _firebase.at("users").unwrap();
    /// users.delete("{\"user_id\":\"1\"}").unwrap();
    pub fn delete(&self, data: &str) -> Result<Response, RequestError> {
        self.request(Method::DELETE, Some(data))
    }

    /// Asynchronous method for delete.
    /// Takes a callback function and returns a handle.
    /// # Examples
    /// ```
    ///  let _firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap();
    ///  let mut users = _firebase.at("users").unwrap();
    ///  let job = users.delete_async("{\"user_id\":\"1\"}", |res| {
    ///      println!("{:?}", res);
    ///  });
    ///  job.join();
    pub fn delete_async<S, F>(&self, data: S, callback: F) -> JoinHandle<()>
    where
        F: Fn(Result<Response, RequestError>) + Send + 'static,
        S: Into<String>,
    {
        Firebase::request_url_async(&self.url, Method::DELETE, Some(data.into()), callback)
    }

    /// Update data
    /// # Examples
    /// ```
    /// let _firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap();
    /// let users = _firebase.at("users/user1").unwrap();
    /// users.update("{\"username\":\"new_user_name\"}").unwrap();
    pub fn update(&self, data: &str) -> Result<Response, RequestError> {
        self.request(Method::PATCH, Some(data))
    }

    /// Asynchronous method for update.
    /// Takes a callback function and returns a handle.
    /// # Examples
    /// ```
    ///  let _firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap();
    ///  let mut users = _firebase.at("users/user1").unwrap();
    ///  let job = users.update_async("{\"username\":\"new_username\"}", |res| {
    ///      println!("{:?}", res);
    ///  });
    ///  job.join();
    pub fn update_async<S, F>(&self, data: S, callback: F) -> JoinHandle<()>
    where
        F: Fn(Result<Response, RequestError>) + Send + 'static,
        S: Into<String>,
    {
        Firebase::request_url_async(&self.url, Method::PATCH, Some(data.into()), callback)
    }

    /// Filter, Sort, Limit and Format Firebase data
    /// !That can useable with GET method
    /// # Examples
    /// ```
    ///  let _firebase = Firebase::new("https://myfirebase.firebaseio.com").unwrap();
    ///  let mut users = _firebase.at("users/user1").unwrap();
    /// let order = users.with_params().order_by("user_id").get();
    /// // Chaining can also be used.
    /// let order = users.with_params().order_by("user_id").limit_to_first(10).get();
    /// let res = order.get().unwrap();
    pub fn with_params(&self) -> FirebaseParams {
        FirebaseParams::new(&self.url)
    }
}

#[derive(Clone, Debug)]
pub struct FirebaseParams {
    pub url: Arc<Url>,
    pub params: HashMap<&'static str, String>,
}

impl FirebaseParams {
    pub fn new(url: &Url) -> Self {
        Self {
            url: Arc::new(url.clone()),
            params: Default::default(),
        }
    }

    /// Gets data from Firebase
    pub fn get(&self) -> Result<Response, RequestError> {
        Firebase::request_url(&self.url, Method::GET, None)
    }

    /// Asynchronous method for delete.
    /// Takes a callback function and returns a handle.
    pub fn get_async<F>(&self, callback: F) -> JoinHandle<()>
    where
        F: Fn(Result<Response, RequestError>) + 'static + Send,
    {
        Firebase::request_url_async(&self.url, Method::GET, None, callback)
    }

    pub fn get_url(&self) -> Result<Url, UrlParseError> {
        Ok((*self.url).clone())
    }

    pub fn add_param<T: ToString>(mut self, key: &'static str, value: T) -> Self {
        let value = value.to_string();
        self.params.insert(key, value);
        self.set_params();
        self
    }

    pub fn set_params(&mut self) {
        let mut url = (*self.url).clone();
        for (k, v) in self.params.iter() {
            url.set_query(Some(format!("{}={}", k, v).as_str()));
        }
        self.url = Arc::new(url)
    }

    pub fn order_by(self, key: &str) -> Self {
        self.add_param(ORDER_BY, key)
    }

    pub fn limit_to_first(self, count: u32) -> Self {
        self.add_param(LIMIT_TO_FIRST, count)
    }

    pub fn limit_to_last(self, count: u32) -> Self {
        self.add_param(LIMIT_TO_LAST, count)
    }

    pub fn start_at(self, index: u32) -> Self {
        self.add_param(START_AT, index)
    }

    pub fn end_at(self, index: u32) -> Self {
        self.add_param(END_AT, index)
    }

    pub fn equal_to(self, value: u32) -> Self {
        self.add_param(EQUAL_TO, value)
    }

    pub fn shallow(self, flag: bool) -> Self {
        self.add_param(SHALLOW, flag)
    }

    pub fn format(self) -> Self {
        self.add_param(FORMAT, EXPORT)
    }
}

// Test
#[cfg(test)]
mod tests {
    use Firebase;

    #[test]
    fn not_http_test() {
        let _firebase = Firebase::auth("http://firebaseio.com", "5");
        assert_eq!(_firebase.is_err(), true);
    }
}
