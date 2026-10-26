use std::fmt;

/// Errors produced by the Fleet Project model.
#[derive(Debug)]
pub enum Error {
    /// A value failed validation. The message names the offending item.
    Invalid(String),
    /// The document was written by a newer version of the software.
    SchemaTooNew { found: u32, supported: u32 },
    /// The document is not valid JSON or does not match the schema.
    Json(serde_json::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Invalid(msg) => write!(f, "invalid fleet data: {msg}"),
            Error::SchemaTooNew { found, supported } => write!(
                f,
                "project schema version {found} is newer than supported version {supported}"
            ),
            Error::Json(err) => write!(f, "malformed fleet document: {err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Json(err) => Some(err),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Json(err)
    }
}

/// Returns `Err` unless `value` is finite. `what` names the field for the error message.
pub(crate) fn require_finite(value: f64, what: &str) -> crate::Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "{what} must be a finite number, got {value}"
        )))
    }
}

/// Returns `Err` unless `text` is non-empty after trimming whitespace.
pub(crate) fn require_text(text: &str, what: &str) -> crate::Result<()> {
    if text.trim().is_empty() {
        Err(Error::Invalid(format!("{what} must not be empty")))
    } else {
        Ok(())
    }
}
