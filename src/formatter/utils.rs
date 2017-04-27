use std::collections::HashMap;
use std::error::Error;
use std::iter::repeat;

use select::node::{Node, Data};


#[derive(Debug)]
pub enum HtmlError {
    NotElement,
    Parse { inner: Box<Error> },
}

impl HtmlError {
    pub fn from_error<T: 'static + Error>(error: T) -> HtmlError {
        HtmlError::Parse { inner: Box::new(error) }
    }
}

pub fn repeat_char(c: char, times: usize) -> String {
    repeat(c).take(times).collect::<String>()
}

pub fn get_attrs<'a>(node: &'a Node) -> Result<HashMap<&'a str, &'a str>, HtmlError> {
    match *node.data() {
        Data::Element(_, ref attrs) => {
            let attrs = attrs
                .iter()
                .map(|&(ref key, ref value)| (key.as_ref(), value.as_ref()))
                .collect::<HashMap<_, _>>();

            Ok(attrs)
        },
        _ => Err(HtmlError::NotElement),
    }
}
