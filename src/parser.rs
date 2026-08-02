use std::collections::HashMap;

use crate::{
    headers::{HeaderError, HeaderName, HeaderValue},
    url::URL,
};

#[derive(Debug)]
pub enum METHOD {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    OPTIONS,
    CONNECT,
    TRACE,
}

impl METHOD {
    fn from_str(s: &str) -> Result<METHOD, HttpError> {
        match s.to_ascii_lowercase().as_str() {
            "get" => Ok(METHOD::GET),
            "post" => Ok(METHOD::POST),
            "put" => Ok(METHOD::PUT),
            "patch" => Ok(METHOD::PATCH),
            "delete" => Ok(METHOD::DELETE),
            "options" => Ok(METHOD::OPTIONS),
            "connect" => Ok(METHOD::CONNECT),
            "trace" => Ok(METHOD::TRACE),
            _ => Err(HttpError::InvalidMethod),
        }
    }
}

#[derive(Debug)]

pub enum HttpError {
    InvalidMethod,
    InvalidURL(String),
    HeaderError,
}

/// This struct is only meant to be read only
#[derive(Debug)]
pub struct HttpRequest {
    pub method: METHOD,
    pub url: URL,
    pub version: String, // change to enum
    pub headers: HashMap<HeaderName, HeaderValue>,
}

struct HttpParser {}
