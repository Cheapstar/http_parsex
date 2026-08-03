use std::collections::HashMap;

use crate::{
    headers::{HeaderName, HeaderValue, HttpRequest, METHOD, parse_header_line},
    url::URL,
};

/// Error That May Occur During Parsing
#[derive(Debug, PartialEq)]
pub enum HttpParserError {
    InvalidMethod,
    InvalidURL(String),
    HeaderError,
    InvalidStartLine,
    InvalidCharacterDetected,
    InvalidHttpVersion,
    InvalidRequestFormat,
    InvalidHeader,
    AlreadyComplete,
}

#[derive(PartialEq)]
enum HttpParserState {
    STARTLINE,
    HEADER,
    /// these kind of states becomes necessary due to uneven chunks
    AFTERLINE,
    DONE,
}

/// HttpParser is a incrementally fed parser for HTTP request.
/// It parses the "start line" and "headers" and returns the body if included in chunk.
/// It uses FINITE STATE MACHINE , rather than requiring the entire body to be loaded into memory up front.
/// Small parts are buffered in memory, and as soon as any data is processed
/// it can be retrieved via get_processed_chunk continuously.
///
///
/// Parse is done by continuously calling `[parse]` until all the bytes are given to be parsed.
/// Parsing is finished when vector is returned (could be empty as well).
///
/// request can be accessed at when either vector is returned using `[get_request]`.
pub struct HttpParser {
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
    ///     `HttpParserError`
    pub fn parse(&mut self, chunk: &[u8]) -> Result<Option<Vec<u8>>, HttpParserError> {
        if self.state == HttpParserState::DONE {
            // Unexpected Bytes Detected at the end of parse
            return Err(HttpParserError::AlreadyComplete);
        }

        let owned_buf;
        let new_chunk: &[u8] = if self.inner.is_empty() {
            chunk
        } else {
            self.inner.extend_from_slice(chunk);
            owned_buf = std::mem::take(&mut self.inner);
            &owned_buf
        };

        let mut index: usize = 0;

        loop {
            // this will be the starting state
            if self.state == HttpParserState::STARTLINE {
                let newline_exists = find_crlf(&new_chunk);

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
                    return Err(HttpParserError::InvalidRequestFormat);
                }

                index += 2;
                self.state = HttpParserState::HEADER;
            }

            if self.state == HttpParserState::HEADER {
                let newline_exists = find_crlf(&new_chunk[index..]);

                if newline_exists.is_none() {
                    self.inner.extend_from_slice(&new_chunk[index..]);
                    break;
                }

                // this will be relative to the headers
                let newline_index = newline_exists.unwrap();

                if !new_chunk[index..index + newline_index].is_ascii() {
                    return Err(HttpParserError::InvalidCharacterDetected);
                }

                if newline_index == 0 {
                    // we reached the empty line
                    // so return the body bytes
                    self.state = HttpParserState::DONE;
                    return Ok(Some(new_chunk[index + newline_index + 2..].to_vec()));
                }

                let header_str = str::from_utf8(&new_chunk[index..index + newline_index]).unwrap();
                let (key, value) =
                    parse_header_line(header_str).map_err(|_| HttpParserError::InvalidHeader)?;

                if let Some(ref mut req) = self.req {
                    req.get_headers_mut().insert(key, value);
                }

                index += newline_index;
                self.state = HttpParserState::AFTERLINE;
            }
        }
        Ok(None)
    }

    /// Returns the Parsed `HttpRequest` instance
    pub fn get_request(&mut self) -> HttpRequest {
        self.req.take().unwrap()
    }
}

fn find_crlf(chunk: &[u8]) -> Option<usize> {
    chunk.windows(2).position(|item| item == b"\r\n")
}

fn parse_startline(chunk: &[u8]) -> Result<HttpRequest, HttpParserError> {
    if !chunk.is_ascii() {
        return Err(HttpParserError::InvalidCharacterDetected);
    }

    let mut unparsed_chunk = str::from_utf8(chunk).unwrap();

    let method_str = if let Some((before, after)) = unparsed_chunk.split_once(' ') {
        unparsed_chunk = after;
        before
    } else {
        return Err(HttpParserError::InvalidStartLine);
    };

    let url_str = if let Some((before, after)) = unparsed_chunk.split_once(' ') {
        unparsed_chunk = after;
        before
    } else {
        return Err(HttpParserError::InvalidStartLine);
    };

    if !unparsed_chunk.eq("HTTP/1.1") {
        return Err(HttpParserError::InvalidHttpVersion);
    }

    let version = unparsed_chunk.to_owned();

    let method = METHOD::from_str(method_str)?;
    let url = URL::parse(url_str)?;

    let headers: HashMap<HeaderName, HeaderValue> = HashMap::new();

    Ok(HttpRequest::new(method, url, version, headers))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;

    const RAW_REQ: &str = "POST /api/users?active=true HTTP/1.1\r\nHost: example.com:8080\r\nContent-Type: application/json\r\nContent-Length: 25\r\nConnection: keep-alive\r\nUser-Agent: test-client/1.0\r\n\r\n{\"name\":\"Alice\",\"age\":30}";
    const EXPECTED_BODY: &str = "{\"name\":\"Alice\",\"age\":30}";

    fn feed_in_chunks(parser: &mut HttpParser, raw: &[u8], chunk_sizes: &[usize]) -> Vec<u8> {
        let mut offset = 0;
        for (i, &size) in chunk_sizes.iter().enumerate() {
            let end = (offset + size).min(raw.len());
            let chunk = &raw[offset..end];
            offset = end;

            let result = parser
                .parse(chunk)
                .unwrap_or_else(|e| panic!("parser errored on chunk {i} ({chunk:?}): {e:?}"));

            let is_last_chunk = offset >= raw.len();
            match result {
                None => {
                    assert!(
                        !is_last_chunk,
                        "parser returned None after consuming all input on chunk {i} \
                         (should have returned Some(leftover) once headers were fully fed)"
                    );
                }
                Some(leftover) => {
                    assert!(
                        is_last_chunk,
                        "parser finished early on chunk {i}, before all input was fed \
                         (headers must have completed mid-stream — use a bespoke test, \
                         not this helper, for that case)"
                    );
                    return leftover;
                }
            }
        }
        panic!("parser never returned Some(leftover) after feeding all chunks");
    }

    #[test]
    fn single_shot_whole_request() {
        let mut parser = HttpParser::new();
        let result = parser.parse(RAW_REQ.as_bytes()).unwrap();

        let body = result.expect("expected Some body once full request is fed");
        assert_eq!(str::from_utf8(&body).unwrap(), EXPECTED_BODY);

        let req = parser.get_request();
        println!("Request: {:?}", req);
    }

    #[test]
    fn byte_by_byte_feed() {
        let mut parser = HttpParser::new();
        let raw = RAW_REQ.as_bytes();
        let boundary = RAW_REQ.find("\r\n\r\n").unwrap() + 4; // end of blank line

        for i in 0..boundary {
            let result = parser.parse(&raw[i..i + 1]).unwrap();
            if i < boundary - 1 {
                assert!(
                    result.is_none(),
                    "should still be parsing headers at byte {i}"
                );
            } else {
                let leftover = result.expect("headers complete on the final CRLF byte");
                assert!(
                    leftover.is_empty(),
                    "no body bytes had arrived yet, expected empty leftover, got {leftover:?}"
                );
            }
        }

        for i in boundary..raw.len() {
            let result = parser.parse(&raw[i..i + 1]);
            assert_eq!(
                result.expect_err("Already Complete Error is Expected"),
                HttpParserError::AlreadyComplete
            );
        }
    }

    #[test]
    fn split_exactly_at_header_body_boundary() {
        let raw = RAW_REQ.as_bytes();
        let boundary = RAW_REQ.find("\r\n\r\n").unwrap() + 4;
        let mut parser = HttpParser::new();

        let first = parser.parse(&raw[..boundary]).unwrap();
        let leftover = first.expect("headers complete as soon as the blank line is seen");
        assert!(
            leftover.is_empty(),
            "no body bytes arrived in this call yet"
        );

        let second = parser.parse(&raw[boundary..]);
        assert_eq!(
            second.expect_err("Already Complete Error is Expected"),
            HttpParserError::AlreadyComplete
        );
    }

    #[test]
    fn split_one_byte_before_boundary() {
        let raw = RAW_REQ.as_bytes();
        let boundary = RAW_REQ.find("\r\n\r\n").unwrap() + 4;
        let mut parser = HttpParser::new();

        let first = parser.parse(&raw[..boundary - 1]).unwrap();
        assert!(first.is_none());

        let second = parser.parse(&raw[boundary - 1..]).unwrap();
        let body = second.expect("should complete once the rest of the blank line arrives");
        assert_eq!(str::from_utf8(&body).unwrap(), EXPECTED_BODY);
    }

    #[test]
    fn split_mid_header_name() {
        let raw = RAW_REQ.as_bytes();
        let idx = RAW_REQ.find("Content-Length").unwrap() + 7;
        let mut parser = HttpParser::new();

        let first = parser.parse(&raw[..idx]).unwrap();
        assert!(first.is_none());

        let body = feed_in_chunks(&mut parser, &raw[idx..], &[raw.len() - idx]);
        assert_eq!(str::from_utf8(&body).unwrap(), EXPECTED_BODY);
    }

    #[test]
    fn split_mid_content_length_digits() {
        let raw = RAW_REQ.as_bytes();
        let idx = RAW_REQ.find("Content-Length: 2").unwrap() + 18;
        let mut parser = HttpParser::new();

        let first = parser.parse(&raw[..idx]).unwrap();
        assert!(first.is_none());

        let body = feed_in_chunks(&mut parser, &raw[idx..], &[raw.len() - idx]);
        assert_eq!(str::from_utf8(&body).unwrap(), EXPECTED_BODY);
        assert_eq!(body.len(), 25);
    }

    #[test]
    fn body_arrives_in_multiple_chunks() {
        let raw = RAW_REQ.as_bytes();
        let boundary = RAW_REQ.find("\r\n\r\n").unwrap() + 4;
        let mut parser = HttpParser::new();

        let r1 = parser.parse(&raw[..boundary]).unwrap();
        let leftover = r1.expect("headers complete as soon as blank line is seen");
        assert!(
            leftover.is_empty(),
            "no body bytes arrived with the headers"
        );

        let body_bytes = &raw[boundary..];
        assert_eq!(body_bytes.len(), 25);
    }

    #[test]
    fn partial_body_bytes_in_same_chunk_as_header_completion_are_returned() {
        let raw = RAW_REQ.as_bytes();
        let boundary = RAW_REQ.find("\r\n\r\n").unwrap() + 4;
        let mut parser = HttpParser::new();

        let chunk = &raw[..boundary + 10];
        let result = parser.parse(chunk).unwrap();
        let leftover = result.expect("headers complete; leftover should include the 10 body bytes");
        assert_eq!(leftover, &raw[boundary..boundary + 10]);
    }

    #[test]
    fn no_body_request_completes_immediately_after_headers() {
        let raw = "GET /health HTTP/1.1\r\nHost: localhost:3000\r\nConnection: close\r\n\r\n";
        let mut parser = HttpParser::new();
        let result = parser.parse(raw.as_bytes()).unwrap();
        let body = result.expect("should complete right after the blank line, no body to wait for");
        assert!(body.is_empty(), "expected empty body, got {body:?}");
    }

    #[test]
    fn malformed_request_line_errors() {
        let raw = "NOT_A_VALID_REQUEST_LINE\r\n\r\n";
        let mut parser = HttpParser::new();
        let result = parser.parse(raw.as_bytes());
        assert!(
            result.is_err(),
            "expected an error for a malformed request line"
        );
    }

    #[test]
    fn fragmented_headers_produce_correct_request_fields() {
        let raw = RAW_REQ.as_bytes();

        let chunk_sizes = vec![5, 3, 17, 1, 40, 9999];
        let mut parser = HttpParser::new();
        let body = feed_in_chunks(&mut parser, raw, &chunk_sizes);
        assert_eq!(str::from_utf8(&body).unwrap(), EXPECTED_BODY);

        let req = parser.get_request();
        println!("Request after fragmented feed: {:?}", req);
        assert_eq!(*req.get_method(), METHOD::POST);
        assert_eq!(req.get_url().get_path(), "/api/users");
        assert_eq!(
            req.get_headers().get(&HeaderName::ContentLength),
            Some(&HeaderValue::ContentLength(25))
        );
    }
}
