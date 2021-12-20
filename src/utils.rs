use crate::UrlParseError;
use url::Url;

pub fn check_uri(uri: &String) -> Result<Url, UrlParseError> {
    let uri = Url::parse(uri.as_str());

    let uri = match uri {
        Ok(res) => res,
        Err(err) => return Err(UrlParseError::Parser(err)),
    };

    if uri.scheme() != "https" {
        return Err(UrlParseError::NotHttps);
    }

    Ok(uri)
}
