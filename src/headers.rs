use std::collections::HashMap;

use crate::{parser::HttpParserError, url::URL};

/// Went through expressjs, remix and axum , they usually store common headers so i am gonna do that doo

/// HttpRequest represent the different components of HttpRequest.
///     1. method : GET | POST | PUT | PATCH | ...
///     2. url
///     3. version
///     4. headers
///
/// NOTE : This struct is only meant to be read only
#[derive(Debug)]
pub struct HttpRequest {
    method: METHOD,
    url: URL,
    version: String, // might change it to the enum
    headers: HashMap<HeaderName, HeaderValue>,
}

impl HttpRequest {
    pub(crate) fn new(
        method: METHOD,
        url: URL,
        version: String,
        headers: HashMap<HeaderName, HeaderValue>,
    ) -> Self {
        HttpRequest {
            method,
            url,
            version,
            headers,
        }
    }

    pub(crate) fn get_mut(&mut self) -> &mut Self {
        self
    }
    pub fn get_method(&self) -> &METHOD {
        &self.method
    }
    pub fn get_url(&self) -> &URL {
        &self.url
    }
    pub fn get_version(&self) -> &str {
        &self.version
    }
    pub fn get_headers(&self) -> &HashMap<HeaderName, HeaderValue> {
        &self.headers
    }

    pub(crate) fn get_headers_mut(&mut self) -> &mut HashMap<HeaderName, HeaderValue> {
        &mut self.headers
    }
}

#[derive(Debug, PartialEq)]
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
    pub fn from_str(s: &str) -> Result<METHOD, HttpParserError> {
        match s.to_ascii_lowercase().as_str() {
            "get" => Ok(METHOD::GET),
            "post" => Ok(METHOD::POST),
            "put" => Ok(METHOD::PUT),
            "patch" => Ok(METHOD::PATCH),
            "delete" => Ok(METHOD::DELETE),
            "options" => Ok(METHOD::OPTIONS),
            "connect" => Ok(METHOD::CONNECT),
            "trace" => Ok(METHOD::TRACE),
            _ => Err(HttpParserError::InvalidMethod(s.to_owned())),
        }
    }
}

/// Bunch of Common headers Saved (Seems like industry Practice)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HeaderName {
    Accept,
    AcceptEncoding,
    AcceptLanguage,
    CacheControl,
    Authorization,
    Connection,
    ContentEncoding,
    ContentLength,
    ContentType,
    Cookie,
    Host,
    Location,
    TransferEncoding,
    Upgrade,
    UserAgent,
    Custom(String),
}

impl HeaderName {
    /// Takes in header_name as string and returns the instance of `HeaderName`.
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "accept" => Self::Accept,
            "accept-encoding" => Self::AcceptEncoding,
            "accept-language" => Self::AcceptLanguage,
            "authorization" => Self::Authorization,
            "cache-control" => Self::CacheControl,
            "connection" => Self::Connection,
            "content-encoding" => Self::ContentEncoding,
            "content-length" => Self::ContentLength,
            "content-type" => Self::ContentType,
            "cookie" => Self::Cookie,
            "host" => Self::Host,
            "location" => Self::Location,
            "transfer-encoding" => Self::TransferEncoding,
            "upgrade" => Self::Upgrade,
            "user-agent" => Self::UserAgent,
            other => Self::Custom(other.to_string()),
        }
    }
}

/// Some Common Headers Value.
#[derive(Debug, Clone, PartialEq)]
pub enum HeaderValue {
    ContentType(ContentType),
    ContentLength(u64),
    Connection(ConnectionValue),
    TransferEncoding(TransferEncodingValue),
    Raw(String),
}

impl HeaderValue {
    /// Returns `HeaderValue` as per the `HeaderName`
    pub fn parse(name: &HeaderName, raw: &str) -> Self {
        let trimmed = raw.trim();
        match name {
            HeaderName::ContentType => HeaderValue::ContentType(ContentType::parse(trimmed)),
            HeaderName::ContentLength => match trimmed.parse::<u64>() {
                Ok(n) => HeaderValue::ContentLength(n),
                Err(_) => HeaderValue::Raw(trimmed.to_owned()),
            },
            HeaderName::Connection => HeaderValue::Connection(ConnectionValue::parse(trimmed)),
            HeaderName::TransferEncoding => {
                HeaderValue::TransferEncoding(TransferEncodingValue::parse(trimmed))
            }
            _ => HeaderValue::Raw(trimmed.to_owned()),
        }
    }
}

/// Value of header `Content-Type`
#[derive(Debug, Clone, PartialEq)]
pub enum ContentType {
    TextPlain,
    TextHtml,
    ApplicationJson,
    ApplicationOctetStream,
    ApplicationFormUrlEncoded,
    MultipartFormData { boundary: Option<String> },
    Other(String),
}

impl ContentType {
    fn parse(raw: &str) -> Self {
        let mut parts = raw.split(';');
        let media_type = parts.next().unwrap_or("").trim().to_ascii_lowercase();

        match media_type.as_str() {
            "text/plain" => Self::TextPlain,
            "text/html" => Self::TextHtml,
            "application/json" => Self::ApplicationJson,
            "application/octet-stream" => Self::ApplicationOctetStream,
            "application/x-www-form-urlencoded" => Self::ApplicationFormUrlEncoded,
            "multipart/form-data" => {
                let boundary = parts
                    .find_map(|p| p.trim().strip_prefix("boundary="))
                    .map(|b| b.trim_matches('"').to_owned());
                Self::MultipartFormData { boundary }
            }
            other => Self::Other(other.to_owned()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionValue {
    KeepAlive,
    Close,
    Other(String),
}

impl ConnectionValue {
    fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "keep-alive" => Self::KeepAlive,
            "close" => Self::Close,
            other => Self::Other(other.to_owned()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransferEncodingValue {
    Chunked,
    Other(String),
}

impl TransferEncodingValue {
    fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "chunked" => Self::Chunked,
            other => Self::Other(other.to_owned()),
        }
    }
}

#[derive(Debug)]
pub(crate) enum HeaderError {
    MissingColon(String),
    EmptyName(String),
}

/// Parse a single raw header line like "Content-Type: application/json"
pub(crate) fn parse_header_line(line: &str) -> Result<(HeaderName, HeaderValue), HeaderError> {
    let (raw_name, raw_value) = line
        .split_once(':')
        .ok_or_else(|| HeaderError::MissingColon(line.to_owned()))?;

    if raw_name.trim().is_empty() {
        return Err(HeaderError::EmptyName(line.to_owned()));
    }

    let name = HeaderName::from_str(raw_name);
    let value = HeaderValue::parse(&name, raw_value);
    Ok((name, value))
}

#[cfg(test)]
mod test {
    use std::io::BufRead;

    use crate::headers::parse_header_line;

    #[test]
    fn header_extracting_correctly() {
        let payload = b"Host: example.com\nContent-Type: application/json\nContent-Length: 42";

        for line in payload.lines() {
            let line_string = line.unwrap();

            let (key, value) = parse_header_line(&line_string).unwrap();

            println!("Key: {:?} , Value : {:?}", key, value);
        }
    }
}
