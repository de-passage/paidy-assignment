use regex::Regex;

/// Default address for both the client and the server
///
/// This is a convenience value to avoid having to provide an
/// address everytime the client or server is started. Ideally this would be drawn from a config
/// file or environment variable.
pub const DEFAULT_ADDRESS: &str = "127.0.0.1:9898";

/// Errors that can occur when parsing the command line arguments
#[derive(Debug, Clone)]
pub enum CLIError {
    InvalidUrlFormat,
    MissingParameter(&'static str),
    InvalidParameter,
}

impl std::fmt::Display for CLIError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CLIError::InvalidUrlFormat => {
                write!(f, "Invalid target format. Should be <host>:<port>")
            }
            CLIError::MissingParameter(missing) => write!(f, "Missing parameter '{}'", missing),
            CLIError::InvalidParameter => write!(f, "Invalid parameter"),
        }
    }
}

impl std::error::Error for CLIError {}

/// Validate the format of the TCP address provided by the user
///
/// Returns its input if the address is in the format <host>:<port>, otherwise InvalidUrlFormat
pub fn validate_address(url: &str) -> std::result::Result<&str, CLIError> {
    let re = Regex::new(r"^[a-zA-Z0-9\.\-]+:\d{1,5}$").unwrap();
    if re.is_match(&url) {
        Ok(url)
    } else {
        Err(CLIError::InvalidUrlFormat)
    }
}

