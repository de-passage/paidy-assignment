use crate::errors::{BoxedError, Error, Result};
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
    /// Create a new DELETE request for the given path, with the given body
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
pub fn parse_request<T>(mut buf_reader: BufReader<T>) -> Result<Request>
where
    T: Sized + Read,
{
    let mut buf = [0; 4096];
    let mut buf_str = String::new();

    let (body_len, parsed_len, mut request) = loop {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        let bytes_read = buf_reader.read(&mut buf)?;

        if bytes_read == 0 {
            return Err(Box::new(Error::ConnectionReset)); // TODO: better error type
        }

        buf_str.push_str(&String::from_utf8_lossy(&buf[..bytes_read]));

        match req.parse(&buf_str.as_bytes()) {
            Ok(httparse::Status::Complete(parsed_len)) => {
                let body_len = req
                    .headers
                    .iter()
                    .find(|h| h.name == "Content-Length")
                    .and_then(|length| String::from_utf8_lossy(length.value).parse::<usize>().ok())
                    .unwrap_or(0);

                break (
                    body_len,
                    parsed_len,
                    Request {
                        method: req.method.unwrap_or("GET").to_string(),
                        path: req.path.unwrap_or("/").to_string(),
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
                        body: "".to_string(),
                    },
                );
            }
            Ok(httparse::Status::Partial) => continue,
            Err(err) => return Err(BoxedError::from(err)),
        }
    };

    // This should be fine for HTTP1.1 since requests are not meant to be sent before
    // the response from the last is received, although connection pooling + an eager
    // request would be dropped.
    // This would be problematic for HTTP2 as we may be dropping part of the next
    // request in the case of multiplexed requests
    while body_len > buf_str.len() - parsed_len {
        let bytes_read = buf_reader.read(&mut buf)?;
        if bytes_read == 0 {
            return Err(Box::new(Error::ConnectionReset));
        }

        // Do we really need that check?
        buf_str.push_str(std::str::from_utf8(&buf[..bytes_read]).unwrap_or(""));
    }
    let body = &buf_str[parsed_len..parsed_len + body_len];
    request.body = body.to_string();

    Result::Ok(request)
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::Rng;

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

    #[test]
    fn test_parse_request_with_large_header() {
        let mut rng = rand::thread_rng();
        let mut buffer = [0; 4096];
        for c in buffer.iter_mut() {
            *c = rng.gen_range('a' as u8..='z' as u8)
        }
        let x_test_header = String::from_utf8_lossy(&buffer);

        let req_str = format!(
            "GET / HTTP/1.1\r\nHost: localhost:8080\r\nUser-Agent: curl/7.68.0\r\nAccept: */*\r\nX-Test: {}\r\n\r\n",
            x_test_header
        );

        let buf_reader = BufReader::new(req_str.as_bytes());
        let parsed_req = parse_request(buf_reader).unwrap();

        assert_eq!(parsed_req.method, "GET");
        assert_eq!(parsed_req.path, "/");
        assert_eq!(parsed_req.headers.len(), 4);
        let x_test = parsed_req
            .headers
            .iter()
            .find(|(k, _)| k == "X-Test")
            .unwrap();
        assert_eq!(x_test.1, x_test_header.to_string());
    }

    #[test]
    fn test_parse_request_with_large_body() {
        let mut rng = rand::thread_rng();
        let mut buffer = [0; 4096];
        for c in buffer.iter_mut() {
            *c = rng.gen_range('a' as u8..='z' as u8)
        }
        let body = String::from_utf8_lossy(&buffer);

        let req_str = format!(
            "GET / HTTP/1.1\r\nHost: localhost:8080\r\nUser-Agent: curl/7.68.0\r\nAccept: */*\r\nContent-Length: {}\r\n\r\n{}",
            buffer.len(),
            body
        );

        let buf_reader = BufReader::new(req_str.as_bytes());
        let parsed_req = parse_request(buf_reader).unwrap();

        assert_eq!(parsed_req.method, "GET");
        assert_eq!(parsed_req.path, "/");
        assert_eq!(parsed_req.headers.len(), 4);
        assert_eq!(parsed_req.body, body);
    }

    #[test]
    fn test_parse_request_with_very_large_body_and_header() {
        let mut rng = rand::thread_rng();
        let mut buffer = [0; 40960];
        for c in buffer.iter_mut() {
            *c = rng.gen_range('a' as u8..='z' as u8)
        }
        let body = String::from_utf8_lossy(&buffer);
        let mut buffer = [0; 40960];
        for c in buffer.iter_mut() {
            *c = rng.gen_range('a' as u8..='z' as u8)
        }
        let x_test_header = String::from_utf8_lossy(&buffer);

        let req_str = format!(
            "GET / HTTP/1.1\r\nHost: localhost:8080\r\nUser-Agent: curl/7.68.0\r\nAccept: */*\r\nContent-Length: {}\r\nX-TEST: {}\r\n\r\n{}",
            buffer.len(),
            x_test_header,
            body
        );

        let buf_reader = BufReader::new(req_str.as_bytes());
        let parsed_req = parse_request(buf_reader).unwrap();

        assert_eq!(parsed_req.method, "GET");
        assert_eq!(parsed_req.path, "/");
        assert_eq!(parsed_req.headers.len(), 5);
        assert_eq!(parsed_req.body, body);
        let x_test = parsed_req
            .headers
            .iter()
            .find(|(k, _)| k == "X-TEST")
            .unwrap();

        assert_eq!(x_test.1, x_test_header);
    }
}
