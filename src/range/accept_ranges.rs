use crate::headers::{HeaderName, HeaderValue, Headers, ToHeaderValues, ACCEPT_RANGES};
use crate::range::Unit;

use std::fmt::{self, Debug, Display};
use std::option;

/// HTTP Accept-Ranges
///
/// Accept-Ranges header indicates that the server supports
/// range requests and specifies the unit to be used by
/// clients for range requests.
///
/// The default value is to not accept range requests.
///
/// # Specifications
///
/// - [RFC 7233, section 2.3: Accept-Ranges](https://tools.ietf.org/html/rfc7233#section-2.3)
/// - [RFC 7233, section 2.1: Byte Ranges](https://tools.ietf.org/html/rfc7233#section-2.1)
/// - [IANA HTTP parameters, range-units: HTTP Range Unit Registry](https://www.iana.org/assignments/http-parameters/http-parameters.xhtml)
///
/// # Examples
///
/// Accepting ranges specified in byte unit (the widely used default):
///
/// ```
/// # fn main() -> http_types::Result<()> {
/// #
/// use http_types::range::{Unit, AcceptRanges};
/// use http_types::Response;
///
/// let accept_ranges = AcceptRanges::new(Unit::Bytes);
///
/// let mut res = Response::new(200);
/// accept_ranges.apply(&mut res);
///
/// let accept_ranges = AcceptRanges::from_headers(res)?.unwrap();
/// assert_eq!(accept_ranges.unit(), &Some(Unit::Bytes));
/// #
/// # Ok(()) }
/// ```
///
/// Other unit ranges:
///
/// ```
/// # fn main() -> http_types::Result<()> {
/// #
/// use http_types::range::{Unit, AcceptRanges};
/// use http_types::Response;
///
/// let custom_unit = Unit::from("my_custom_unit");
/// let accept_ranges = AcceptRanges::new(custom_unit);
///
/// let mut res = Response::new(200);
/// accept_ranges.apply(&mut res);
///
/// let accept_ranges = AcceptRanges::from_headers(res)?.unwrap();
/// assert_eq!(accept_ranges.unit(), &Some(custom_type));
/// #
/// # Ok(()) }
/// ```
///
/// Range requests not accepted:
///
/// ```
/// # fn main() -> http_types::Result<()> {
/// #
/// use http_types::range::{Unit, AcceptRanges};
/// use http_types::Response;
///
/// let accept_ranges = AcceptRanges::new(None);
///
/// let mut res = Response::new(200);
/// accept_ranges.apply(&mut res);
///
/// let accept_ranges = AcceptRanges::from_headers(res)?.unwrap();
/// assert_eq!(accept_ranges.unit(), &None);
/// #
/// # Ok(()) }
/// ```
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct AcceptRanges {
    unit: Option<Unit>,
}

impl AcceptRanges {
    const BYTES: &'static str = "bytes";
    const NONE: &'static str = "none";

    /// Create a new AcceptRange which does not accept range requests.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new AcceptRange accepting `byte` unit.
    pub fn with_bytes() -> Self {
        AcceptRanges {
            bytes: true,
            ..Default::default()
        }
    }

    /// Create a new AcceptRange accepting `byte` unit.
    pub fn with_other(s: impl AsRef<str>) -> Self {
        AcceptRanges {
            other: Some(s.as_ref().to_owned()),
            ..Default::default()
        }
    }

    /// Returns true if AcceptRange accepts `byte` unit.
    pub fn bytes(&self) -> bool {
        self.bytes
    }

    /// Returns the supported unit if any.
    pub fn other(&self) -> &Option<String> {
        &self.other
    }

    /// Create a new instance from headers.
    ///
    /// Only a single AcceptRanges per resource is assumed to exist. If multiple Accept-Ranges
    /// headers are found the last one is used.
    pub fn from_headers(headers: impl AsRef<Headers>) -> crate::Result<Option<Self>> {
        let headers = match headers.as_ref().get(ACCEPT_RANGES) {
            Some(headers) => headers,
            None => return Ok(None),
        };

        // If a header is returned we can assume at least one exists.
        let s = headers.iter().last().unwrap().as_str();
        Self::from_str(s).map(Some)
    }

    /// Create an AcceptRanges from a string.
    pub(crate) fn from_str(s: &str) -> crate::Result<Self> {
        let accept_range = match s {
            Self::NONE => AcceptRanges::default(),
            Self::BYTES => AcceptRanges::with_bytes(),
            other => AcceptRanges::with_other(other),
        };
        Ok(accept_range)
    }

    /// Sets the `Accept-Ranges` header.
    pub fn apply(&self, mut headers: impl AsMut<Headers>) {
        headers.as_mut().insert(ACCEPT_RANGES, self.value());
    }

    /// Get the `HeaderName`.
    pub fn name(&self) -> HeaderName {
        ACCEPT_RANGES
    }

    /// Get the `HeaderValue`.
    pub fn value(&self) -> HeaderValue {
        let s = match self.other {
            Some(ref other) => other.as_str(),
            None if self.bytes => Self::BYTES,
            None => Self::NONE,
        };
        // SAFETY: the internal string is validated to be ASCII.
        unsafe { HeaderValue::from_bytes_unchecked(s.into()) }
    }
}

impl Display for AcceptRanges {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self.other {
            Some(ref other) => other.as_str(),
            None if self.bytes => Self::BYTES,
            None => Self::NONE,
        };
        write!(f, "{}", s)
    }
}

impl ToHeaderValues for AcceptRanges {
    type Iter = option::IntoIter<HeaderValue>;
    fn to_header_values(&self) -> crate::Result<Self::Iter> {
        // A HeaderValue will always convert into itself.
        Ok(self.value().to_header_values().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::headers::Headers;

    use crate::Response;

    #[test]
    fn accept_ranges_none() -> crate::Result<()> {
        let mut headers = Headers::new();
        headers.insert(ACCEPT_RANGES, "none");
        let accept_ranges = AcceptRanges::from_headers(headers).unwrap().unwrap();
        assert_eq!(accept_ranges.other(), &None);
        assert_eq!(accept_ranges.bytes(), false);

        let accept_ranges = AcceptRanges::new();
        let mut res = Response::new(200);
        accept_ranges.apply(&mut res);

        let raw_header_value = res.header(ACCEPT_RANGES).unwrap();
        assert_eq!(raw_header_value, "none");

        Ok(())
    }

    #[test]
    fn accept_ranges_bytes() -> crate::Result<()> {
        let mut headers = Headers::new();
        headers.insert(ACCEPT_RANGES, "bytes");
        let accept_ranges = AcceptRanges::from_headers(headers).unwrap().unwrap();
        assert_eq!(accept_ranges.bytes(), true);

        let accept_ranges = AcceptRanges::with_bytes();
        let mut res = Response::new(200);
        accept_ranges.apply(&mut res);

        let raw_header_value = res.header(ACCEPT_RANGES).unwrap();
        assert_eq!(raw_header_value, "bytes");

        Ok(())
    }

    #[test]
    fn accept_ranges_other() -> crate::Result<()> {
        let mut headers = Headers::new();
        headers.insert(ACCEPT_RANGES, "foo");
        let accept_ranges = AcceptRanges::from_headers(headers).unwrap().unwrap();
        assert_eq!(accept_ranges.other(), &Some("foo".into()));

        let accept_ranges = AcceptRanges::with_other("foo");
        let mut res = Response::new(200);
        accept_ranges.apply(&mut res);

        let raw_header_value = res.header(ACCEPT_RANGES).unwrap();
        assert_eq!(raw_header_value, "foo");

        Ok(())
    }
}
