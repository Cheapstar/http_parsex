//! HttpParser is a incrementally fed parser for HTTP request.
//! It parses the "start line" and "headers" and returns the body if included in chunk.
//! It uses FINITE STATE MACHINE , rather than requiring the entire body to be loaded into memory up front.
//! Small parts are buffered in memory, and as soon as any data is processed
//!
//! Parse is done by continuously calling `[parse]` until all the bytes are given to be parsed.
//! Parsing is finished when vector is returned (could be empty as well).
//!
//! request can be accessed at when either vector is returned using `[get_request]`.
//!
//! # Example
//!
//! ```no_run
//!     let mut parser = HttpParser::new();
//!
//!     let result = parser.parse(RAW_REQ.as_bytes()).unwrap();
//!     let body = result.expect("expected Some body once full request is fed");
//!     assert_eq!(str::from_utf8(&body).unwrap(), EXPECTED_BODY);
//!     let req = parser.get_request();
//!     println!("Request: {:?}", req);
//! ```
//!
//! # Errors
//!
//! Parsing failures are reported via [`HttpParserError`].

mod headers;
pub mod parser;
mod url;
