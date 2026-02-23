use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(#[from] kiutils_sexpr::ParseError),
    #[error("validation error: {0}")]
    Validation(String),
}
