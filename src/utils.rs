use crate::errors::UrlParseResult;
use crate::UrlParseError;
use url::Url;

pub fn check_uri(uri: &str) -> UrlParseResult<Url> {
    let uri = Url::parse(uri.trim_end_matches("/"));

    let uri = match uri {
        Ok(res) => res,
        Err(err) => return Err(UrlParseError::Parser(err)),
    };

    if uri.scheme() != "https" {
        return Err(UrlParseError::NotHttps);
    }

    Ok(uri)
}
