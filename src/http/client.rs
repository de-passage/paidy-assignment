use crate::errors;
use std::io::{BufReader, Write};
use std::net::TcpStream;
use crate::http::{parse_response, Response};

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
        parse_response(buf_reader)
    }
}

