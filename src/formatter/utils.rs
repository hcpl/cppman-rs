use std::error::Error;
use std::iter::repeat;


#[derive(Debug)]
pub enum HtmlError {
    Other { inner: Box<Error> },
}

impl HtmlError {
    pub fn from_error<T: 'static + Error>(error: T) -> HtmlError {
        HtmlError::Other { inner: Box::new(error) }
    }
}

pub fn repeat_char(c: char, times: usize) -> String {
    repeat(c).take(times).collect::<String>()
}
