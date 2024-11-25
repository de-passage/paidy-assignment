use std::io::{BufReader, Read};

use crate::errors::{BoxedError, Error, Result};

/// An HTTP response to be sent to a client
#[derive(Debug)]
pub struct Response {
    /// Status code of the response. Optional because that's what httparse returns, but it
    /// shouldn't happen in practice since we control the responses.
    pub status: Option<u16>,
    /// Headers for the response. It is not necessary to add Content-Length to it, this is done
    /// automatically on serialization.
    pub headers: Vec<(String, String)>,
    /// Body of the response. Give an empty string for an empty body
    pub body: String,
}

impl Response {
    /// Creates an empty OK response (204)
    pub fn ok() -> Response {
        Response {
            status: Some(204),
            headers: vec![],
            body: "".to_string(),
        }
    }

    /// Creates an OK (200) response with the given body
    pub fn ok_with_body(str: String) -> Response {
        Response {
            status: Some(200),
            headers: vec![],
            body: str,
        }
    }

    /// Creates an error response with the given body.
    ///
    /// The code must be in the 4xx or 5xx range.
    ///
    /// No body is added intentionally to avoid leaking information about the server until I build
    /// some better error handling.
    pub fn error(code: u16) -> Response {
        assert!((400..600).contains(&code), "Invalid error code");
        Response {
            status: Some(code),
            headers: vec![],
            body: "".to_string(),
        }
    }

    /// Creates an Internal Server Error (500) response.
    pub fn internal_server_error() -> Response {
        Self::error(500)
    }
}

/// Parse an HTTP response from a byte stream
///
/// At the moment, this function doesn't handle responses bigger than 4096 bytes because I'm
/// struggling getting the lifetimes right around the growing buffer.
///
/// TODO: handle responses bigger than 4096 bytes
pub fn parse_response<T>(mut buf_reader: BufReader<T>) -> Result<Response>
where
    T: Sized + Read,
{
    // This is duplicated from the request implementation, we could probably make a somewhat generic
    // implementation but I don't have the time to do it right now
    let mut buf = [0; 4096];
    let mut buf_str = String::new();

    let (body_len, parsed_len, mut request) = loop {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Response::new(&mut headers);
        let bytes_read = buf_reader.read(&mut buf)?;

        if bytes_read == 0 {
            return Err(Box::new(Error::ConnectionReset)); // TODO: better error type
        }

        buf_str.push_str(&String::from_utf8_lossy(&buf[..bytes_read]));

        match req.parse(buf_str.as_bytes()) {
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
                    Response {
                        status: req.code,
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
    fn test_parse_simple_response() {
        let req_str = b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        let buf_reader = BufReader::new(&req_str[..]);

        let parsed_req = parse_response(buf_reader).unwrap();

        assert_eq!(parsed_req.status, Some(200));
        assert_eq!(parsed_req.headers.len(), 1);
        assert_eq!(parsed_req.body, "");
    }

    #[test]
    fn test_parse_response_with_body() {
        let body = "{ \"content\": \"Hello, world!\" }";
        let req_str = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );

        let buf_reader = BufReader::new(req_str.as_bytes());
        let parsed_req = parse_response(buf_reader).unwrap();

        assert_eq!(parsed_req.status, Some(200));
        assert_eq!(parsed_req.headers.len(), 1);
        assert_eq!(parsed_req.body, body);
    }

    #[test]
    fn test_parse_response_with_large_header() {
        let mut rng = rand::thread_rng();
        let mut buffer = [0; 4096];
        for c in buffer.iter_mut() {
            *c = rng.gen_range(b'a'..=b'z')
        }
        let x_test_header = String::from_utf8_lossy(&buffer);

        let resp_str = format!("HTTP/1.1 200 OK\r\nX-Test: {}\r\n\r\n", x_test_header);

        let buf_reader = BufReader::new(resp_str.as_bytes());
        let parsed_resp = parse_response(buf_reader).unwrap();

        assert_eq!(parsed_resp.headers.len(), 1);
        let x_test = parsed_resp
            .headers
            .iter()
            .find(|(k, _)| k == "X-Test")
            .unwrap();
        assert_eq!(x_test.1, x_test_header.to_string());
    }

    #[test]
    fn test_parse_response_with_large_body() {
        let mut rng = rand::thread_rng();
        let mut buffer = [0; 4096];
        for c in buffer.iter_mut() {
            *c = rng.gen_range(b'a'..=b'z')
        }
        let body = String::from_utf8_lossy(&buffer);

        let resp_str = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            buffer.len(),
            body
        );

        let buf_reader = BufReader::new(resp_str.as_bytes());
        let parsed_resp = parse_response(buf_reader).unwrap();

        assert_eq!(parsed_resp.headers.len(), 1);
        assert_eq!(parsed_resp.body, body);
    }

    #[test]
    fn test_parse_response_with_very_large_body_and_header() {
        let mut rng = rand::thread_rng();
        let mut buffer = [0; 40960];
        for c in buffer.iter_mut() {
            *c = rng.gen_range(b'a'..=b'z')
        }
        let body = String::from_utf8_lossy(&buffer);
        let mut buffer = [0; 40960];
        for c in buffer.iter_mut() {
            *c = rng.gen_range(b'a'..=b'z')
        }
        let x_test_header = String::from_utf8_lossy(&buffer);

        let resp_str = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nX-TEST: {}\r\n\r\n{}",
            buffer.len(),
            x_test_header,
            body
        );

        let buf_reader = BufReader::new(resp_str.as_bytes());
        let parsed_resp = parse_response(buf_reader).unwrap();

        assert_eq!(parsed_resp.headers.len(), 2);
        assert_eq!(parsed_resp.body, body);
        let x_test = parsed_resp
            .headers
            .iter()
            .find(|(k, _)| k == "X-TEST")
            .unwrap();

        assert_eq!(x_test.1, x_test_header);
    }
}
