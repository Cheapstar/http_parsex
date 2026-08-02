// Went through expressjs, remix and axum , they usually store common headers so i am gonna do to that do
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HeaderName {
    Accept,
    AcceptEncoding,
    AcceptLanguage,
    Authorization,
    CacheControl,
    Connection,
    ContentEncoding,
    ContentLength,
    ContentType,
    Cookie,
    Host,
    Location,
    SetCookie,
    TransferEncoding,
    Upgrade,
    UserAgent,
    Custom(String),
}

impl HeaderName {
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
            "set-cookie" => Self::SetCookie,
            "transfer-encoding" => Self::TransferEncoding,
            "upgrade" => Self::Upgrade,
            "user-agent" => Self::UserAgent,
            other => Self::Custom(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HeaderValue {
    ContentType(ContentType),
    ContentLength(u64),
    Connection(ConnectionValue),
    TransferEncoding(TransferEncodingValue),
    Raw(String),
}

impl HeaderValue {
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
                // look for boundary=... in remaining params
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
pub enum HeaderError {
    MissingColon(String),
    EmptyName(String),
}

/// Parse a single raw header line like "Content-Type: application/json"
pub fn parse_header_line(line: &str) -> Result<(HeaderName, HeaderValue), HeaderError> {
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
