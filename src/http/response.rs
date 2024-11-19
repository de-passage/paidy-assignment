use std::io::{BufReader, Read};

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
        assert!(code >= 400 && code < 600, "Invalid error code");
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
pub fn parse_response<T>(mut buf_reader: BufReader<T>) -> std::option::Option<Response>
where
    T: Sized + Read,
{
    // This is duplicated from above, we could probably make a somewhat generic implementation
    // but I don't have the time to do it right now
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Response::new(&mut headers);

    let mut buf = [0; 4096];

    let bytes_read = buf_reader.read(&mut buf).ok()?;

    if bytes_read == 0 || bytes_read == buf.len() {
        return None;
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
                return None;
            }

            let body = &buf[parsed_len..parsed_len + length];

            Some(Response {
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
                body: String::from_utf8_lossy(body).to_string(),
            })
        }
        Ok(httparse::Status::Partial) => None,
        Err(_) => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;
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
}
