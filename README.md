# http-parsex

An incremental, streaming HTTP/1.x request parser written in Rust.

`HttpParser` is built around a **finite state machine (FSM)**, so it never
requires the full request to be loaded into memory before parsing begins.
You feed it bytes as they arrive (e.g. from a TCP socket), and it
incrementally parses the **start line** and **headers**, buffering only the
small, as-yet-incomplete pieces it needs. Once the headers are fully parsed,
any body bytes present in a chunk are handed back to the caller immediately
via the parser's streaming output, instead of being accumulated internally.

This makes `HttpParser` suitable for servers that want to start acting on a
request (e.g. streaming a large upload to disk) before the entire body has
arrived on the wire.

## Installation

Run:

```bash
cargo add http_parsex
```

Or add it to your `Cargo.toml` manually:

```toml
[dependencies]
http_parsex = "0.1"
```

## How it works

```
           chunk 1              chunk 2              chunk N
              │                    │                    │
              ▼                    ▼                    ▼
       ┌─────────────────────────────────────────────────────┐
       │                     HttpParser (FSM)                 │
       │                                                       │
       │   StartLine ──► Headers ──► (Body passthrough)        │
       │                                                       │
       └─────────────────────────────────────────────────────┘
              │                    │                    │
              ▼                    ▼                    ▼
         None / Err          None / Err        Some(body_bytes)
                                                (parsing finished)
```

---

## Usage

```rust
    let mut parser = HttpParser::new();

    let result = parser.parse(RAW_REQ.as_bytes()).unwrap();
    let body = result.expect("expected Some body once full request is fed");
    assert_eq!(str::from_utf8(&body).unwrap(), EXPECTED_BODY);
    let req = parser.get_request();
    println!("Request: {:?}", req);

```

---

## API Reference

### `HttpParser`

```rust
pub struct HttpParser {
    inner: Vec<u8>,
    state: HttpParserState,
    req: Option<HttpRequest>,
}
```

The parser itself. Holds an internal byte buffer (`inner`) for partially
received data, the current FSM `state`, and the in-progress/parsed
`HttpRequest` (`req`).

#### `new`

```rust
pub fn new() -> Self
```

Creates a fresh parser, ready to receive chunks via `parse`.

#### `parse`

```rust
pub fn parse(&mut self, chunk: &[u8]) -> Result<Option<Vec<u8>>, HttpParserError>
```

Feeds a chunk of bytes into the parser.

|               |                                                                                                                                                                                                              |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Arguments** | `chunk` — a slice of bytes to parse next                                                                                                                                                                     |
| **Returns**   | `Ok(Some(bytes))` when the start line and headers have been fully parsed (`bytes` is any body data included in this chunk, and may be empty). `Ok(None)` when more data is needed before parsing can finish. |
| **Errors**    | Returns `HttpParserError` if the chunk contains data that violates the HTTP request format (see [`HttpParserError`](#httpparsererror)).                                                                      |

Call `parse` repeatedly, feeding successive chunks, until it returns
`Ok(Some(_))` (finished) or `Err(_)` (malformed request).

#### `get_request`

```rust
pub fn get_request(&mut self) -> HttpRequest
```

Retrieves the parsed `HttpRequest`. Should be called once `parse` has
signaled completion (returned `Ok(Some(_))`).

> **Note:** calling this before parsing has finished will not yield a
> meaningful/complete request, since `req` is only fully populated at that
> point.

#### `get_processed_chunk`

Continuously exposes body bytes as they're processed, allowing consumers to
retrieve data that has already been handled without waiting for the entire
body to be loaded into memory.

---

### `HttpParserError`

```rust
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
```

| Variant                    | Meaning                                                                   |
| -------------------------- | ------------------------------------------------------------------------- |
| `InvalidMethod`            | The HTTP method in the start line isn't recognized.                       |
| `InvalidURL(String)`       | The request target/URL couldn't be parsed; the `String` carries details.  |
| `HeaderError`              | A general failure while processing headers.                               |
| `InvalidStartLine`         | The start line is malformed (wrong number of parts, bad separators, etc). |
| `InvalidCharacterDetected` | An illegal/unexpected byte was found where it isn't allowed.              |
| `InvalidHttpVersion`       | The HTTP version token isn't a supported/valid format.                    |
| `InvalidRequestFormat`     | The request doesn't conform to expected structure overall.                |
| `InvalidHeader`            | A specific header line is malformed.                                      |
| `AlreadyComplete`          | `parse` was called again after parsing had already finished.              |

---

### `HttpRequest`

```rust
#[derive(Debug)]
pub struct HttpRequest {
    method: METHOD,
    url: URL,
    version: String,
    headers: HashMap<HeaderName, HeaderValue>,
}
```

The fully parsed HTTP request:

- `method` — the HTTP method (`GET`, `POST`, `PUT`, `PATCH`, ...).
- `url` — the parsed [`URL`](#url).
- `version` — the HTTP version string (e.g. `"HTTP/1.1"`).
- `headers` — a map from [`HeaderName`](#headername) to its value.

This struct is **read-only** by design — it represents parsed input and
isn't meant to be mutated after the fact.

---

### `URL`

```rust
#[derive(Debug)]
pub struct URL {
    scheme: Option<String>,
    host: Option<String>,
    port_number: Option<u16>,
    path: String,
    query_params: Option<HashMap<String, String>>,
    fragment: Option<String>,
}
```

Represents the components of the request URL:

| Field          | Description                                                     |
| -------------- | --------------------------------------------------------------- |
| `scheme`       | Protocol used (`http` in our case).                             |
| `host`         | Subdomain + domain + top-level domain combined into one string. |
| `port_number`  | Port, if explicitly specified.                                  |
| `path`         | The URL path.                                                   |
| `query_params` | Parsed query string parameters, if any.                         |
| `fragment`     | The URL fragment (`#...`), if any.                              |

See [Components of a URL](https://www.geeksforgeeks.org/computer-networks/components-of-a-url/)
for background on URL anatomy.

This struct is **read-only**, same as `HttpRequest`.

---

### `HeaderName`

```rust
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
```

A set of well-known headers gets first-class enum variants (mirroring the
approach taken by Express, Remix, and Axum), with `Custom(String)` as a
catch-all for anything not explicitly modeled. This keeps common header
lookups cheap and typo-proof while still supporting arbitrary headers.

---

### `ContentType`

```rust
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
```

Typed representation of the `Content-Type` header value. `MultipartFormData`
carries an optional `boundary` parameter, since multipart requests need it to
split the body into parts.

---

### `ConnectionValue`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionValue {
    KeepAlive,
    Close,
    Other(String),
}
```

Typed representation of the `Connection` header value.

---

### `TransferEncodingValue`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TransferEncodingValue {
    Chunked,
    Other(String),
}
```

Typed representation of the `Transfer-Encoding` header value.

---

## Documentation

Full API documentation is available on [docs.rs](https://docs.rs/chunked_decoder).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

Copyright [2026] Cheapstar
