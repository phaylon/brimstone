
use std::rc;
use std::ops;

pub fn pluralize<'a, T>(value: T, singular: &'a str, plural: &'a str) -> &'a str
where T: Into<u64> {
    if value.into() == 1 { singular } else { plural }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RcString {
    value: rc::Rc<String>,
}

impl RcString {

    pub fn new() -> RcString {
        String::new().into()
    }

    pub fn into_string(&self) -> String { (**self.value).into() }

    pub fn as_str(&self) -> &str { &self.value }
}

impl From<String> for RcString {

    fn from(value: String) -> RcString {
        RcString {
            value: rc::Rc::new(value)
        }
    }
}

impl<'a> From<&'a str> for RcString {

    fn from(value: &'a str) -> RcString {
        value.to_owned().into()
    }
}

impl ops::Deref for RcString {

    type Target = str;

    fn deref(&self) -> &str { &self.value }
}
