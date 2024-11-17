use regex::Regex;

pub const DEFAULT_ADDRESS: &str = "127.0.0.1:9898";

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

pub fn validate_url(url: &str) -> std::result::Result<&str, CLIError> {
    let re = Regex::new(r"^[a-zA-Z0-9\.\-]+:\d{1,5}$").unwrap();
    if re.is_match(&url) {
        Ok(url)
    } else {
        Err(CLIError::InvalidUrlFormat)
    }
}

