pub type BoxedError = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, BoxedError>;

#[derive(Debug)]
pub enum Error {
    NoResponse,
    NotFound(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoResponse => write!(f, "No response from server"),
            Error::NotFound(err) => write!(f, "Not found: {}", err),
        }
    }
}

impl std::error::Error for Error {}
