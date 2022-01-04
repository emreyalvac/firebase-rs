use std::fmt::{Display, Formatter};

pub enum RegisterError {
    EventExist
}

impl Display for RegisterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RegisterError::EventExist => write!(f, "Event exist"),
        }
    }
}