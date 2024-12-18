use crate::{errors, threadpool::ThreadPool};
use std::io::{BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

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

/// Writes an HTTP response to a stream
fn respond(stream: &mut TcpStream, resp: Response) {
    let status = stream.write_all(
        format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n{}\r\n{}",
            resp.status.unwrap_or(500),
            code_to_string(resp.status.unwrap_or(500)),
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

/// This is the main server.
///
/// It listens for incomming connections on a TCP socket, parses the requests and dispatches them
/// to a handler. Whatever the handler produces is then converted in an HTTP response and sent
/// back to the client.
pub struct HttpServer {
    listener: TcpListener,
}

/// Turn an HTTP error code into its string representation
///
/// TODO: look up the standard representations and complete the list
pub fn code_to_string(code: u16) -> &'static str {
    match code {
        400 => "Bad Request",
        404 => "Not Found",
        200 => "OK",
        204 => "No Content",
        500 => "Internal Server Error",
        c => panic!("Missing string for code {}", c),
    }
}

/// Parse an HTTP request from a TCP stream, calls the handler and write back the answer
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
    /// Create a new server listening on the given address
    pub fn new(addr: &str) -> errors::Result<Self> {
        Ok(HttpServer {
            listener: TcpListener::bind(addr)?,
        })
    }

    /// Start the server
    ///
    /// Calls the handler with the incoming requests. Uses a threadpool internally to handle the
    /// requests concurrently on as many threads as the system can handle.
    ///
    /// This function is blocking, with no real way of stopping it (except the socket being
    /// forcefully closed by the OS or the program being killed)
    pub fn serve<F>(&self, handler: F)
    where
        F: Fn(Request) -> Response + Send + Sync + 'static + Clone,
    {
        let threadpool = ThreadPool::new(
            std::thread::available_parallelism()
                .map(|x| x.into())
                .unwrap_or(4),
        );
        for stream in self.listener.incoming() {
            let mut stream = stream.unwrap();
            let handler = handler.clone();
            threadpool.execute(move || handle_stream(&mut stream, &handler))
        }
    }

    /// Utility function for one-shot servers.
    ///
    /// This is mostly for testing, it listens to a single connection, processes the
    /// request and exit.
    pub fn serve_once<F>(&self, handler: F)
    where
        F: Fn(Request) -> Response,
    {
        let mut stream = self.listener.incoming().next().unwrap().unwrap();
        handle_stream(&mut stream, &handler);
    }
}

/// Simple HTTP client
///
/// It sends HTTP requests from a set of parameters, then parses and yields the server response.
pub struct HttpClient {
    stream: TcpStream,
}

impl HttpClient {
    /// Create a new client connected to the given server.
    ///
    /// An error is returned if the connection cannot be made for whatever reason
    pub fn new(server: &str) -> errors::Result<Self> {
        Ok(HttpClient {
            stream: TcpStream::connect(server)?,
        })
    }

    /// Send an HTTP request on the open connection.
    ///
    /// While I believe that it is technically possible to send multiple requests on the same
    /// connection with this, connection keep-alive is not implemented server side.
    /// Drop the object after the response is retrieved.
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
        // It may fail if started several times in a row since the OS may takes some time
        // to make the port available again (or if it is already in use by something else)
        // . If I get around to it I'll extract this into something cleaner
        static ADDR: &str = "127.0.0.1:18422";

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
                    Err(err) => {
                        eprintln!("Trying to connect to {}: {}", ADDR, err);
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            }
            None
        })()
        .expect("Failed to connect client");

        let resp = client
            .send("POST", "/", "{\"content\": \"Hello\"}")
            .expect("Failed to communicate with server");

        assert_eq!(resp.status.unwrap(), 204);

        handle.join().unwrap();
    }
}
