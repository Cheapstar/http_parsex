use std::collections::HashMap;

use crate::parser::HttpError;

/// This struct is only meant to be read only
#[derive(Debug)]
pub struct URL {
    scheme: Option<String>,
    host: Option<String>,
    port_number: Option<u16>,
    path: String,
    query_params: Option<HashMap<String, String>>,
    fragment: Option<String>,
}

impl URL {
    pub fn parse(unparsed_url: &str) -> Result<Self, HttpError> {
        // I am thinking of parsing it in reverse that would be easy

        if unparsed_url.is_empty() {
            return Err(HttpError::InvalidURL(unparsed_url.to_string()));
        }
        let mut rest = unparsed_url;
        // search for fragment
        let fragment = if let Some((before, after)) = rest.split_once('#') {
            rest = before;
            Some(after.to_owned())
        } else {
            None
        };

        //search for query_params
        let query_params = if let Some((before, after)) = rest.split_once('?') {
            rest = before;
            Some(parse_query(after))
        } else {
            None
        };

        let scheme = if let Some((s, after)) = rest.split_once("://") {
            rest = after;
            Some(s.to_owned())
        } else {
            None
        };

        let (authority, path) = if scheme.is_some() {
            match rest.find('/') {
                Some(idx) => (&rest[..idx], &rest[idx..]),
                None => (rest, "/"),
            }
        } else {
            ("", rest)
        };

        let (host, port_number) = if authority.is_empty() {
            (None, None)
        } else {
            match authority.split_once(':') {
                Some((h, p)) => {
                    let port = p
                        .parse::<u16>()
                        .map_err(|_| HttpError::InvalidURL(unparsed_url.to_string()))?;
                    (Some(h.to_owned()), Some(port))
                }
                None => (Some(authority.to_owned()), None),
            }
        };

        Ok(Self {
            scheme,
            host,
            port_number,
            path: path.to_owned(),
            query_params,
            fragment: fragment,
        })
    }

    pub fn get_scheme(&self) -> Option<&str> {
        self.scheme.as_ref().map(|s| s.as_str())
    }
    pub fn get_host(&self) -> Option<&str> {
        self.host.as_ref().map(|s| s.as_str())
    }
    pub fn get_port_number(&self) -> Option<u16> {
        self.port_number
    }
    pub fn get_path(&self) -> &str {
        &self.path
    }
    pub fn get_query_params(&self) -> Option<&HashMap<String, String>> {
        self.query_params.as_ref()
    }
    pub fn get_fragments(&self) -> Option<&str> {
        self.fragment.as_ref().map(|s| s.as_str())
    }
}

fn parse_query(s: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if s.is_empty() {
        return map;
    }
    for pair in s.split('&') {
        if pair.is_empty() {
            continue;
        }
        match pair.split_once('=') {
            Some((k, v)) => {
                map.insert(k.to_owned(), v.to_owned());
            }
            None => {
                map.insert(pair.to_owned(), String::new());
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_url_with_everything() {
        let url =
            URL::parse("https://api.example.com:8080/v1/users?id=42&active=true#top").unwrap();
        assert_eq!(url.scheme, Some("https".into()));
        assert_eq!(url.host, Some("api.example.com".into()));
        assert_eq!(url.port_number, Some(8080));
        assert_eq!(url.path, "/v1/users");
        assert_eq!(
            url.query_params,
            Some(HashMap::from([
                ("id".to_owned(), "42".to_owned()),
                ("active".to_owned(), "true".to_owned()),
            ]))
        );
        assert_eq!(url.fragment, Some("top".into()));
    }

    #[test]
    fn url_without_port() {
        let url = URL::parse("https://example.com/path").unwrap();
        assert_eq!(url.host, Some("example.com".into()));
        assert_eq!(url.port_number, None);
    }

    #[test]
    fn url_without_query() {
        let url = URL::parse("https://example.com/path#frag").unwrap();
        assert_eq!(url.query_params, None);
        assert_eq!(url.fragment, Some("frag".into()));
    }

    #[test]
    fn url_without_fragment() {
        let url = URL::parse("https://example.com/path?x=1").unwrap();
        assert_eq!(url.fragment, None);
    }

    #[test]
    fn bare_url() {
        let url = URL::parse("https://example.com/path").unwrap();
        assert_eq!(url.query_params, None);
        assert_eq!(url.fragment, None);
    }

    #[test]
    fn origin_form_no_scheme_no_host() {
        let url = URL::parse("/foo/bar?baz=qux#section").unwrap();
        assert_eq!(url.scheme, None);
        assert_eq!(url.host, None);
        assert_eq!(url.port_number, None);
        assert_eq!(url.path, "/foo/bar");
        assert_eq!(
            url.query_params,
            Some(HashMap::from([("baz".to_owned(), "qux".to_owned())]))
        );
        assert_eq!(url.fragment, Some("section".into()));
    }

    #[test]
    fn root_path_only() {
        let url = URL::parse("/").unwrap();
        assert_eq!(url.path, "/");
        assert_eq!(url.host, None);
    }

    #[test]
    fn localhost_host() {
        let url = URL::parse("http://localhost:3000/health").unwrap();
        assert_eq!(url.host, Some("localhost".into()));
        assert_eq!(url.port_number, Some(3000));
        assert_eq!(url.path, "/health");
    }

    #[test]
    fn multi_part_tld_host() {
        let url = URL::parse("https://www.example.co.uk/path").unwrap();
        assert_eq!(url.host, Some("www.example.co.uk".into()));
    }

    #[test]
    fn ipv4_host() {
        let url = URL::parse("http://127.0.0.1:8080/status").unwrap();
        assert_eq!(url.host, Some("127.0.0.1".into()));
        assert_eq!(url.port_number, Some(8080));
    }

    #[test]
    fn no_path_defaults_to_root() {
        let url = URL::parse("https://example.com").unwrap();
        assert_eq!(url.path, "/");
    }

    #[test]
    fn multiple_query_params() {
        let url = URL::parse("https://example.com/search?q=rust&sort=asc&page=2").unwrap();
        let params = url.query_params.unwrap();
        assert_eq!(params.get("q"), Some(&"rust".to_owned()));
        assert_eq!(params.get("sort"), Some(&"asc".to_owned()));
        assert_eq!(params.get("page"), Some(&"2".to_owned()));
    }

    #[test]
    fn query_param_without_value() {
        let url = URL::parse("https://example.com/x?debug").unwrap();
        let params = url.query_params.unwrap();
        assert_eq!(params.get("debug"), Some(&"".to_owned()));
    }

    #[test]
    fn query_and_fragment_ordering() {
        let url = URL::parse("https://example.com/x?a=1#frag").unwrap();
        assert_eq!(url.fragment, Some("frag".into()));
        let params = url.query_params.unwrap();
        assert_eq!(params.get("a"), Some(&"1".to_owned()));
    }

    #[test]
    fn invalid_port_errors() {
        let result = URL::parse("https://example.com:notaport/path");
        assert!(result.is_err());
    }

    #[test]
    fn out_of_range_port_errors() {
        let result = URL::parse("https://example.com:99999/path");
        assert!(result.is_err());
    }

    #[test]
    fn empty_input_errors() {
        let result = URL::parse("");
        assert!(result.is_err());
    }
}
