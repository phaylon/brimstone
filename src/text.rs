
use std::rc;
use std::ops;
use std::fmt;
use std::borrow;

pub fn parse_search(value: &str) -> Vec<&str> {
    value.trim().split_whitespace().map(|term| term.trim()).collect()
}

pub fn pluralize<'a, T>(value: T, singular: &'a str, plural: &'a str) -> &'a str
where T: Into<u64> {
    if value.into() == 1 { singular } else { plural }
}

pub fn escape<'a>(value: &'a str) -> borrow::Cow<'a, str> {

    const REPLACE: &[char] = &['&', '"', '<', '>'];

    if value.contains(REPLACE) {
        let mut escaped = String::new();
        let mut rest = value;
        loop {
            if let Some(index) = rest.find(REPLACE) {
                escaped.push_str(&rest[..index]);
                rest = &rest[index..];
                let found = rest.chars().next().expect("found char is next char");
                rest = &rest[found.len_utf8()..];
                escaped.push_str(match found {
                    '&' => "&amp;",
                    '"' => "&quot;",
                    '<' => "&lt;",
                    '>' => "&gt;",
                    other => panic!("unexpected escape char {:?}", other),
                });
            } else {
                escaped.push_str(rest);
                break;
            }
        }
        escaped.into()
    } else {
        value.into()
    }
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

impl fmt::Display for RcString {

    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, fmt)
    }
}

impl From<String> for RcString {

    fn from(value: String) -> RcString {
        RcString {
            value: rc::Rc::new(value)
        }
    }
}

impl<'a> From<borrow::Cow<'a, str>> for RcString {

    fn from(value: borrow::Cow<'a, str>) -> RcString {
        value.to_owned().into()
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
