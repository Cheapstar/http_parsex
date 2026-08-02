use std::collections::HashMap;

use crate::parser::HttpError;

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
