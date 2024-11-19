use crate::{errors, threadpool::ThreadPool};
use std::io::{BufReader, Write};
use std::net::{TcpListener, TcpStream};
use crate::http::{Request,Response,parse_request};


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

/// This is the main server.
///
/// It listens for incomming connections on a TCP socket, parses the requests and dispatches them
/// to a handler. Whatever the handler produces is then converted in an HTTP response and sent
/// back to the client.
pub struct HttpServer {
    listener: TcpListener,
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
