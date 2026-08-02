use std::collections::HashMap;

use crate::{
    headers::{HeaderName, HeaderValue, parse_header_line},
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
    InvalidStartLine,
    InvalidCharacterDetected,
    InvalidHttpVersion,
    InvalidRequestFormat,
    InvalidHeader,
}

/// This struct is only meant to be read only
#[derive(Debug)]
pub struct HttpRequest {
    pub method: METHOD,
    pub url: URL,
    pub version: String, // might change it to the enum
    pub headers: HashMap<HeaderName, HeaderValue>,
}

#[derive(PartialEq)]
pub enum HttpParserState {
    STARTLINE,
    HEADER,
    /// these kind of states becomes necessary due to uneven chunks
    AFTERLINE,
    DONE,
}

struct HttpParser {
    inner: Vec<u8>,
    state: HttpParserState,
    req: Option<HttpRequest>, // Option is only because we need it during parsing
}

impl HttpParser {
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            state: HttpParserState::STARTLINE,
            req: None,
        }
    }

    /// # Arguments
    ///     * `chunk` : slice of bytes
    ///
    /// # Returns
    ///     * `bytes` if parsing is finished else None
    ///
    /// # Error
    ///     `HttpError`
    pub fn parse(&mut self, chunk: &[u8]) -> Result<Option<Vec<u8>>, HttpError> {
        let mut new_chunk: Vec<u8> = Vec::new();

        if self.inner.is_empty() {
            let new_chunk_size = chunk.len();
            new_chunk.reserve_exact(new_chunk_size);
            new_chunk.extend_from_slice(chunk);
        } else {
            let new_chunk_size = self.inner.len() + chunk.len();
            new_chunk.reserve_exact(new_chunk_size);
            new_chunk.extend_from_slice(&self.inner);
            new_chunk.extend_from_slice(chunk);

            self.inner.clear();
        }

        let mut index: usize = 0;

        loop {
            // this will be the starting state
            if self.state == HttpParserState::STARTLINE {
                let newline_exists = find_cr(chunk);

                if newline_exists.is_none() {
                    self.inner.extend_from_slice(&new_chunk);
                    break;
                }

                // we got the start line
                let newline_index = newline_exists.unwrap();
                let start_line = &new_chunk[..newline_index];

                let req = parse_startline(start_line)?;

                self.req = Some(req);

                self.state = HttpParserState::AFTERLINE;
                index = newline_index;
            }

            if self.state == HttpParserState::AFTERLINE {
                if new_chunk.len() < index + 2 {
                    self.inner.extend_from_slice(&new_chunk[index..]);
                    break;
                }

                if new_chunk[index] != b'\r' || new_chunk[index + 1] != b'\n' {
                    return Err(HttpError::InvalidRequestFormat);
                }

                index += 2;
                self.state = HttpParserState::HEADER;
            }

            if self.state == HttpParserState::HEADER {
                let newline_exists = find_cr(&new_chunk[index..]);

                if newline_exists.is_none() {
                    self.inner.extend_from_slice(&new_chunk[index..]);
                    break;
                }

                // this will be relative to the headers
                let newline_index = newline_exists.unwrap();

                if !new_chunk[index..index + newline_index].is_ascii() {
                    return Err(HttpError::InvalidCharacterDetected);
                }

                if newline_index == 0 {
                    // we reached the empty line
                    // so return the body bytes
                    return Ok(Some(new_chunk[index + newline_index + 2..].to_vec()));
                }

                let header_str = str::from_utf8(&new_chunk[index..index + newline_index]).unwrap();
                let (key, value) =
                    parse_header_line(header_str).map_err(|e| HttpError::InvalidHeader)?;

                if let Some(ref mut req) = self.req {
                    req.headers.insert(key, value);
                }

                index += newline_index;
                self.state = HttpParserState::AFTERLINE;
            }
        }
        Ok(None)
    }

    pub fn get_request(&mut self) -> HttpRequest {
        self.req.take().unwrap()
    }
}

fn find_cr(chunk: &[u8]) -> Option<usize> {
    chunk.windows(1).position(|item| item == b"\r")
}

fn parse_startline(chunk: &[u8]) -> Result<HttpRequest, HttpError> {
    let mut rest = &[0u8];

    if !chunk.is_ascii() {
        return Err(HttpError::InvalidCharacterDetected);
    }

    let mut unparsed_chunk = str::from_utf8(chunk).unwrap();

    let method_str = if let Some((before, after)) = unparsed_chunk.split_once(' ') {
        unparsed_chunk = after;
        before
    } else {
        return Err(HttpError::InvalidStartLine);
    };

    let url_str = if let Some((before, after)) = unparsed_chunk.split_once(' ') {
        unparsed_chunk = after;
        before
    } else {
        return Err(HttpError::InvalidStartLine);
    };

    if !unparsed_chunk.eq("HTTP/1.1") {
        return Err(HttpError::InvalidHttpVersion);
    }

    let version = unparsed_chunk.to_owned();

    let method = METHOD::from_str(method_str)?;
    let url = URL::parse(url_str)?;

    let headers: HashMap<HeaderName, HeaderValue> = HashMap::new();

    Ok(HttpRequest {
        method,
        url,
        version,
        headers,
    })
}

#[cfg(test)]
mod tests {
    use crate::parser::HttpParser;

    #[test]
    fn working_correctly() {
        let raw_req = "POST /api/users?active=true HTTP/1.1\r\nHost: example.com:8080\r\nContent-Type: application/json\r\nContent-Length: 27\r\nConnection: keep-alive\r\nUser-Agent: test-client/1.0\r\n\r\n{\"name\":\"Alice\",\"age\":30}";

        let mut parser = HttpParser::new();

        let result = parser.parse(&raw_req.as_bytes()).unwrap();

        let req = parser.get_request();

        println!("Request \r\n {:?}\r\n", req);

        let body = result.unwrap();

        let body_str = str::from_utf8(&body).unwrap();

        println!("Body {}", body_str);
    }
}
