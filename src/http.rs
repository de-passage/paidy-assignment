use crate::errors;
use std::io::{BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

fn parse_request<T>(mut buf_reader: BufReader<T>) -> std::option::Option<Request>
where
    T: Sized + Read,
{
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);

    let mut buf = [0; 4096]; // Nothing will work if the request is larger than this, but I don't
                             // have time to solve the lifetime issues that arise if I loop to
                             // extend a buffer dynamically.

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
            // Obviously we may be dropping part of the next request. Since I'm not gonna
            // implement connection pooling this isn't too bad, but definitely
            // something to improve

            Some(Request {
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
            })
        }
        Ok(httparse::Status::Partial) => None,
        Err(_) => None,
    }
}

#[derive(Debug)]
pub struct Response {
    pub status: Option<u16>,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Response {
    pub fn ok() -> Response {
        Response {
            status: Some(200),
            headers: vec![],
            body: "".to_string(),
        }
    }
}

pub fn parse_response<T>(mut buf_reader: BufReader<T>) -> std::option::Option<Response>
where
    T: Sized + Read,
{
    // This is duplicated from above, we could probably make a somewhat generic implementation
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

fn respond(stream: &mut TcpStream, resp: Response) {
    let status = stream.write_all(
        format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n{}\r\n{}",
            resp.status.unwrap_or(500),
            from_code(resp.status.unwrap_or(500)),
            resp.body.len(),
            resp.headers
                .iter()
                .map(|(k, v)| format!["{}:{}\r\n", k, v])
                .collect::<Vec<_>>()
                .join(""),
            resp.body
        )
        .as_bytes(),
    );

    match status {
        Err(err) => eprintln!("Failed to respond {}", err),
        _ => (),
    }
}

pub struct HttpServer {
    listener: TcpListener,
}

fn from_code(code: u16) -> &'static str {
    match code {
        400 => "Bad Request",
        200 => "OK",
        500 => "Internal Server Error",
        c => panic!("Missing string for code {}", c),
    }
}

fn handle_stream<F>(mut stream: &mut TcpStream, handler: F)
where
    F: Fn(Request) -> Response,
{
    let buf_reader = BufReader::new(&mut stream);
    match parse_request(buf_reader) {
        Some(req) => respond(&mut stream, handler(req)),
        None => respond(
            &mut stream,
            Response {
                status: Some(400),
                body: "".to_string(),
                headers: vec![],
            },
        ),
    }
}

impl HttpServer {
    pub fn new(addr: &str) -> errors::Result<Self> {
        Ok(HttpServer {
            listener: TcpListener::bind(addr)?,
        })
    }

    pub fn serve<F>(&self, handler: F)
    where
        F: Fn(Request) -> Response,
    {
        for stream in self.listener.incoming() {
            let mut stream = stream.unwrap();
            handle_stream(&mut stream, &handler)
        }
    }

    pub fn serve_once<F>(&self, handler: F)
    where
        F: Fn(Request) -> Response,
    {
        let mut stream = self.listener.incoming().next().unwrap().unwrap();
        handle_stream(&mut stream, &handler);
    }
}

pub struct HttpClient {
    stream: TcpStream,
}

impl HttpClient {
    pub fn new(server: &str) -> errors::Result<Self> {
        Ok(HttpClient {
            stream: TcpStream::connect(server)?,
        })
    }

    pub fn send(&mut self, method: &str, endpoint: &str, body: &str) -> errors::Result<Response> {
        self.stream.write_all(
            format! {
                "{} {} HTTP/1.1\r\nContent-Length: {}\r\n\r\n{}",
                method, endpoint, body.len(), body
            }
            .as_bytes(),
        )?;

        let buf_reader = BufReader::new(&mut self.stream);
        parse_response(buf_reader).ok_or(Box::new(errors::Error::NoResponse))
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

        assert!(parsed_req.is_none());
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
    fn test_simple_http_request() {
        // I normally would not do this kind of tests here but time is short
        // It may fail if started several times in a row since the OS takes some time
        // to make the port available again. If I get around to it I'll extract this into
        // something cleaner
        static ADDR: &str = "127.0.0.1:1422";

        let handle = std::thread::spawn(|| {
            eprintln!("Connecting to {}", ADDR);
            let server = HttpServer::new(ADDR);
            match server {
                Ok(s) => s.serve_once(|_| Response::ok()),
                Err(err) => eprintln!("Failed to spawn server: {}", err),
            }
        });

        let mut client = (|| {
            for _ in 1..10 {
                match HttpClient::new(ADDR) {
                    Ok(c) => return Some(c),
                    Err(err) => eprintln!("Trying to connect to {}: {}", ADDR, err),
                }
            }
            None
        })()
        .expect("Failed to connect client");

        let resp = client
            .send("POST", "/", "{\"content\": \"Hello\"}")
            .expect("Failed to communicate with server");

        assert_eq!(resp.status.unwrap(), 200);

        handle.join().unwrap();
    }
}
