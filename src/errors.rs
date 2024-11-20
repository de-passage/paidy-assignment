/// Type alias to simplify conversions where necessary
pub type BoxedError = Box<dyn std::error::Error>;

/// The general Result type used throughout the application
pub type Result<T> = std::result::Result<T, BoxedError>;

/// Application error types
///
/// This is mixing server-side and client-side errors, which is not ideal.
#[derive(Debug)]
pub enum Error {
    /// An HTTP request didn't get a response from the server
    NoResponse,
    /// The requested resource (path or object) doesn't exist
    NotFound(String),
    /// Incoming request is malformed or incoherent with the server's expectations
    BadRequest(String),
    /// Something went wrong server-side
    InternalServerError(String),
    /// A TCP stream was closed unexpectedly
    ConnectionReset,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoResponse => write!(f, "No response from server"),
            Error::NotFound(err) => write!(f, "Not found: {}", err),
            Error::BadRequest(err) => write!(f, "Bad Request: {}", err),
            Error::InternalServerError(err) => write!(f, "InternalServerError: {}", err),
            Error::ConnectionReset => write!(f, "ConnectionReset"),
        }
    }
}

impl std::error::Error for Error {}
