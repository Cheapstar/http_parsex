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
}

#[derive(Debug)]
pub struct HttpRequest {}

struct HttpParser {}
