#[derive(Debug, Clone, Eq, PartialEq)]
pub enum JqlErrorKind {
    GenericError,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JqlError {
    pub kind: JqlErrorKind,
    pub message: String,
}

impl std::fmt::Display for JqlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            JqlErrorKind::GenericError => write!(f, "generic error:{}", self.message),
            _ => Err(std::fmt::Error {}),
        }
    }
}

impl std::error::Error for JqlError {}

impl JqlError {
    pub fn new(kind: JqlErrorKind, message: String) -> Self {
        Self { kind, message }
    }
}
