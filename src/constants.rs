pub const AUTH: &str = "auth";
pub const ORDER_BY: &str = "orderBy";
pub const LIMIT_TO_FIRST: &str = "limitToFirst";
pub const LIMIT_TO_LAST: &str = "limitToLast";
pub const START_AT: &str = "startAt";
pub const END_AT: &str = "endAt";
pub const EQUAL_TO: &str = "equalTo";
pub const SHALLOW: &str = "shallow";
pub const FORMAT: &str = "format";
pub const EXPORT: &str = "export";

#[derive(Debug)]
pub enum Method {
    GET,
    POST,
    DELETE,
    PATCH,
}

#[derive(Debug)]
pub struct Response {
    pub data: String,
}

impl Response {
    pub fn new() -> Self {
        Self {
            data: String::default(),
        }
    }
}
