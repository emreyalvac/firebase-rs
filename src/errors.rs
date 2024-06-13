use std::{
    error::Error,
    fmt::{Display, Formatter},
};

pub type UrlParseResult<T> = Result<T, UrlParseError>;

#[derive(Debug)]
pub enum UrlParseError {
    NoPath,
    NotHttps,
    Parser(url::ParseError),
}

impl Error for UrlParseError {}

impl Display for UrlParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UrlParseError::NoPath => write!(f, "URL path is missing."),
            UrlParseError::NotHttps => write!(f, "The URL protocol should be https."),
            UrlParseError::Parser(e) => write!(f, "Error while parsing the URL: {}", e),
        }
    }
}

pub type RequestResult<T> = Result<T, RequestError>;

#[derive(Debug)]
pub enum RequestError {
    NotJSON,
    NoUTF8,
    NetworkError,
    SerializeError,
    NotFoundOrNullBody,
}

impl Error for RequestError {}

impl Display for RequestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestError::NotJSON => write!(f, "Invalid JSON"),
            RequestError::NoUTF8 => write!(f, "Utf8 error"),
            RequestError::NetworkError => write!(f, "Network error"),
            RequestError::SerializeError => write!(f, "Serialize error"),
            RequestError::NotFoundOrNullBody => write!(f, "Body is null or record is not found"),
        }
    }
}

#[derive(Debug)]
pub enum ServerEventError {
    ConnectionError,
}

impl Error for ServerEventError {}

impl Display for ServerEventError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerEventError::ConnectionError => write!(f, "Connection error for server events"),
        }
    }
}
