use crate::errors::{BoxedError, Result, Error};
use std::io::{BufReader, Read};

/// Represents an HTTP request.
///
/// This datastructure probably needs to be simplified/split to avoid carrying redundant
/// information around the application (typically path + params after rounting).
#[derive(Debug)]
pub struct Request {
    /// The HTTP method used in the request
    pub method: String,
    /// The full path of the request
    pub path: String,
    /// Headers of the request
    pub headers: Vec<(String, String)>,
    /// Body of the request
    pub body: String,
}

impl Request {
    /// Create a new request from scratch
    pub fn new(method: &str, path: &str, headers: Vec<(String, String)>, body: String) -> Request {
        Request {
            method: method.to_string(),
            path: path.to_string(),
            headers,
            body,
        }
    }
    /// Create a new GET request for the given path, with an empty body
    pub fn get(path: &str) -> Request {
        Request {
            method: "GET".to_string(),
            body: "".to_string(),
            headers: vec![],
            path: path.to_string(),
        }
    }
    /// Create a new POST request for the given path, with the given body
    pub fn post(path: &str, body: String) -> Request {
        Request {
            method: "POST".to_string(),
            body,
            headers: vec![],
            path: path.to_string(),
        }
    }
    /// Create a new DELEET request for the given path, with the given body
    pub fn delete(path: &str, body: String) -> Request {
        Request {
            method: "DELETE".to_string(),
            body,
            headers: vec![],
            path: path.to_string(),
        }
    }
}

/// Parse an HTTP request from a byte stream
///
/// At the moment, this function doesn't handle requests bigger than 4096 bytes because I'm
/// struggling getting the lifetimes right around the growing buffer.
///
/// TODO: handle requests bigger than 4096 bytes
pub fn parse_request<T>(mut buf_reader: BufReader<T>) -> Result<Request>
where
    T: Sized + Read,
{
    let mut buf = [0; 4096];

    loop {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        let bytes_read = buf_reader.read(&mut buf)?;

        if bytes_read == 0 {
            return Err(Box::new(Error::ConnectionReset)); // TODO: better error type
        }

        match req.parse(&buf) {
            Ok(httparse::Status::Complete(parsed_len)) => {
                let length = req
                    .headers
                    .iter()
                    .find(|h| h.name == "Content-Length")
                    .and_then(|length| String::from_utf8_lossy(length.value).parse::<usize>().ok())
                    .unwrap_or(0);

                if parsed_len + length > buf.len() {
                    return Err("Request too big".into());
                }

                let body = &buf[parsed_len..parsed_len + length];
                // Obviously we may be dropping part of the next request. Since I'm not gonna
                // implement connection pooling this isn't too bad, but definitely
                // something to improve

                return Ok(Request {
                    method: req.method.unwrap().to_string(),
                    path: req.path.unwrap().to_string(),
                    headers: req
                        .headers
                        .iter()
                        .map(|h| {
                            (
                                h.name.to_string(),
                                String::from_utf8_lossy(h.value).to_string(),
                            )
                        })
                        .collect(),
                    body: String::from_utf8_lossy(body).to_string(),
                });
            }
            Ok(httparse::Status::Partial) => continue,
            Err(err) => return Err(BoxedError::from(err)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_parse_simple_request() {
        let req_str = b"GET / HTTP/1.1\r\nHost: localhost:8080\r\nUser-Agent: curl/7.68.0\r\nAccept: */*\r\n\r\n";
        let buf_reader = BufReader::new(&req_str[..]);

        let parsed_req = parse_request(buf_reader).unwrap();

        assert_eq!(parsed_req.method, "GET");
        assert_eq!(parsed_req.path, "/");
        assert_eq!(parsed_req.headers.len(), 3);
        assert_eq!(parsed_req.body, "");
    }

    #[test]
    fn test_parse_incomplete_request() {
        let req_str =
            b"GET / HTTP/1.1\r\nHost: localhost:8080\r\nUser-Agent: curl/7.68.0\r\nAccept: */*";
        let buf_reader = BufReader::new(&req_str[..]);

        let parsed_req = parse_request(buf_reader);

        assert!(parsed_req.is_err());
    }

    #[test]
    fn test_parse_request_with_body() {
        let body = "{ \"content\": \"Hello, world!\" }";
        let req_str = format!(
            "POST / HTTP/1.1\r\nHost: localhost:8080\r\nUser-Agent: curl/7.68.0\r\nAccept: */*\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );

        let buf_reader = BufReader::new(req_str.as_bytes());

        let parsed_req = parse_request(buf_reader).unwrap();

        assert_eq!(parsed_req.method, "POST");
        assert_eq!(parsed_req.path, "/");
        assert_eq!(parsed_req.headers.len(), 4);
        assert_eq!(parsed_req.body, body);
    }
}
