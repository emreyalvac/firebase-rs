use crate::constants::{
    END_AT, EQUAL_TO, EXPORT, FORMAT, LIMIT_TO_FIRST, LIMIT_TO_LAST, ORDER_BY, SHALLOW, START_AT,
};
use crate::Firebase;
use itertools::Itertools;
use std::collections::HashMap;
use url::Url;

#[derive(Debug)]
pub struct Params {
    pub uri: Url,
}

impl Params {
    pub fn new(uri: Url) -> Self {
        Self {
            uri,
        }
    }

    pub fn add_param<T>(&mut self, key: &str, value: T) -> &mut Self
    where
        T: ToString,
    {
        self.uri.query_pairs_mut().append_pair(key, value.to_string().as_str());
        self
    }

    pub fn order_by(&mut self, key: &str) -> &mut Params {
        self.add_param(ORDER_BY, key)
    }

    pub fn limit_to_first(&mut self, count: u32) -> &mut Params {
        self.add_param(LIMIT_TO_FIRST, count)
    }

    pub fn limit_to_last(&mut self, count: u32) -> &mut Params {
        self.add_param(LIMIT_TO_LAST, count)
    }

    pub fn start_at(&mut self, index: u32) -> &mut Params {
        self.add_param(START_AT, index)
    }

    pub fn end_at(&mut self, index: u32) -> &mut Params {
        self.add_param(END_AT, index)
    }

    pub fn equal_to(&mut self, value: u32) -> &mut Params {
        self.add_param(EQUAL_TO, value)
    }

    pub fn shallow(&mut self, flag: bool) -> &mut Params {
        self.add_param(SHALLOW, flag)
    }

    pub fn format(&mut self) -> &mut Params {
        self.add_param(FORMAT, EXPORT)
    }

    pub fn finish(&self) -> Firebase {
        Firebase::new(self.uri.as_str()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::params::Params;
    use std::collections::HashMap;
    use url::Url;

    #[test]
    fn check_params() {
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("param_1".to_owned(), "value_1".to_owned());
        params.insert("param_2".to_owned(), "value_2".to_owned());
        let mut param = Params {
            uri: Url::parse("https://github.com/emreyalvac").unwrap(),
        };
        for (key, value) in params.iter() {
            param.add_param(key, value);
        }
        assert_eq!(
            param.uri.as_str(),
            "https://github.com/emreyalvac?param_1=value_1&param_2=value_2"
        )
    }
}
