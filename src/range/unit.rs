use std::fmt;

const BYTES: &str = "bytes";

/// The HTTP range requests unit type.
///
/// # Specifications
///
/// - [RFC 7233, section 2: Range Units](https://tools.ietf.org/html/rfc7233#section-2)
/// - [IANA HTTP parameters, range-units: HTTP Range Unit Registry](https://www.iana.org/assignments/http-parameters/http-parameters.xhtml)
#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Unit {
    /// The *bytes* range unit is defined for expressing subranges of the data's octet sequence.
    Bytes,
    /// Range unit not yet registered with IANA.
    Other(String),
}

impl std::default::Default for Unit {
    fn default() -> Self {
        Unit::Bytes
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unit::Bytes => write!(f, "{}", BYTES),
            Unit::Other(s) => write!(f, "{}", s),
        }
    }
}

impl From<&str> for Unit {
    fn from(s: &str) -> Self {
        match s {
            BYTES => Unit::Bytes,
            _ => Unit::Other(s.to_owned()),
        }
    }
}
