use crate::headers::{HeaderName, HeaderValue, Headers, ToHeaderValues, RANGE};
use crate::range::ByteRange;
use crate::{Error, StatusCode};

use std::fmt::{self, Debug, Display};
use std::option;
use std::str::FromStr;

/// HTTP Range requests with bytes unit.
///
/// Range header in a GET request modifies the method
/// semantics to request transfer of only one or more subranges of the
/// selected representation data, rather than the entire selected
/// representation data.
///
/// # Specifications
///
/// - [RFC 7233, section 3.1: Range](https://tools.ietf.org/html/rfc7233#section-3.1)
/// - [RFC 7233, Appendix D: Collected ABNF](https://tools.ietf.org/html/rfc7233#appendix-D)
/// - [IANA HTTP parameters, range-units: HTTP Range Unit Registry](https://www.iana.org/assignments/http-parameters/http-parameters.xhtml#range-units)
///
/// # Examples
///
/// Raw Range header parsing:
///
/// ```
/// # fn main() -> http_types::Result<()> {
/// #
/// use http_types::range::{ByteRange, ByteRanges};
/// use http_types::{Method, Request, Url};
///
/// let mut ranges = ByteRanges::new();
/// ranges.push(0, 500);
///
/// let mut req = Request::new(Method::Get, Url::parse("http://example.com").unwrap());
/// ranges.apply(&mut req);
///
/// let ranges = ByteRanges::from_headers(req)?.unwrap();
/// assert_eq!(ranges.first().unwrap(), ByteRange::new(0, 500));
/// #
/// # Ok(()) }
/// ```
///
/// Most of the time applications want to validate the range set against
/// the actual size of the document, as per the RFC specification:
///
/// ```
/// # fn main() -> http_types::Result<()> {
/// #
/// use http_types::range::{ByteRange, ByteRanges};
/// use http_types::{Method, Request, StatusCode, Url};
/// use std::convert::TryInto;
///
/// let mut ranges = ByteRanges::new();
/// ranges.push(0, 500);
///
/// let mut req = Request::new(Method::Get, Url::parse("http://example.com").unwrap());
/// ranges.apply(&mut req);
///
/// let ranges = ByteRanges::from_headers(req)?.unwrap();
/// assert_eq!(ranges.first().unwrap(), ByteRange::new(0, 500));
///
/// let err = ranges.match_size(350).unwrap_err();
/// assert_eq!(err.status(), StatusCode::RequestedRangeNotSatisfiable);
/// #
/// # Ok(()) }
/// ```
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct ByteRanges {
    ranges: Vec<ByteRange>,
}

impl ByteRanges {
    const RANGE_PREFIX: &'static str = "bytes=";

    /// Create a new instance with an empty range set.
    pub fn new() -> Self {
        ByteRanges::default()
    }

    /// Pushes a new byte range at the end of the byte range set.
    pub fn push<S, E>(&mut self, start: S, end: E)
    where
        S: Into<Option<u64>>,
        E: Into<Option<u64>>,
    {
        let range = ByteRange::new(start, end);
        self.ranges.push(range);
    }

    /// Returns an `Iterator` over the byte ranges.
    pub fn iter(&self) -> impl Iterator<Item = &ByteRange> {
        self.ranges.iter()
    }

    /// Returns the first byte range in the byte ranges set.
    pub fn first(&self) -> Option<ByteRange> {
        self.ranges.get(0).copied()
    }

    /// Validates that the ranges are withing the expected document size.
    ///
    /// Returns `HTTP 416 Range Not Satisfiable` if one range is out of bounds.
    pub fn match_size(&self, size: u64) -> crate::Result<()> {
        for range in &self.ranges {
            if !range.match_size(size) {
                return Err(Error::from_str(
                    StatusCode::RequestedRangeNotSatisfiable,
                    "Invalid Range header for byte ranges",
                ));
            }
        }
        Ok(())
    }

    /// Create a new instance from a Range headers.
    ///
    /// Only a single Range per resource is assumed to exist. If multiple Range
    /// headers are found the last one is used.
    pub fn from_headers(headers: impl AsRef<Headers>) -> crate::Result<Option<Self>> {
        let headers = match headers.as_ref().get(RANGE) {
            Some(headers) => headers,
            None => return Ok(None),
        };

        // If a header is returned we can assume at least one exists.
        let s = headers.iter().last().unwrap().as_str();
        if !s.starts_with(Self::RANGE_PREFIX) {
            return Ok(None);
        }
        Self::from_str(s).map(Some)
    }

    /// Create a ByteRanges from a string.
    pub(crate) fn from_str(s: &str) -> crate::Result<Self> {
        let fn_err = || {
            Error::from_str(
                StatusCode::BadRequest,
                "Invalid Range header for byte ranges",
            )
        };

        if !s.starts_with(Self::RANGE_PREFIX) {
            return Err(fn_err());
        }

        let s = &s[Self::RANGE_PREFIX.len()..].trim_start();
        let mut ranges = Self::default();

        for range_str in s.split(',') {
            let range = ByteRange::from_str(range_str)?;
            ranges.ranges.push(range);
        }

        if ranges.ranges.is_empty() {
            return Err(fn_err());
        }

        Ok(ranges)
    }

    /// Sets the `Range` header.
    pub fn apply(&self, mut headers: impl AsMut<Headers>) {
        headers.as_mut().insert(RANGE, self.value());
    }

    /// Get the `HeaderName`.
    pub fn name(&self) -> HeaderName {
        RANGE
    }

    /// Get the `HeaderValue`.
    pub fn value(&self) -> HeaderValue {
        let s = self.to_string();
        // SAFETY: the internal string is validated to be ASCII.
        unsafe { HeaderValue::from_bytes_unchecked(s.into()) }
    }
}

impl IntoIterator for ByteRanges {
    type Item = ByteRange;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.ranges.into_iter()
    }
}

impl<'a> IntoIterator for &'a ByteRanges {
    type Item = &'a ByteRange;
    type IntoIter = std::slice::Iter<'a, ByteRange>;

    fn into_iter(self) -> Self::IntoIter {
        self.ranges.iter()
    }
}

impl Display for ByteRanges {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bytes=")?;
        for (i, range) in self.iter().enumerate() {
            if i > 0 {
                write!(f, ",")?;
            }
            write!(f, "{}", range)?;
        }
        Ok(())
    }
}

impl ToHeaderValues for ByteRanges {
    type Iter = option::IntoIter<HeaderValue>;
    fn to_header_values(&self) -> crate::Result<Self::Iter> {
        // A HeaderValue will always convert into itself.
        Ok(self.value().to_header_values().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::{ByteRange, ByteRanges};
    use crate::headers::RANGE;
    use crate::{Method, Request, Url};

    #[test]
    fn byte_ranges_single_range() -> crate::Result<()> {
        let mut req = Request::new(Method::Get, Url::parse("http://example.com").unwrap());
        req.insert_header(RANGE, "bytes=1-5");
        let mut ranges = ByteRanges::from_headers(req)?.unwrap().into_iter();
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges.next(), Some(ByteRange::new(1, 5)));
        Ok(())
    }

    #[test]
    fn byte_ranges_invalid_unit_prefix() -> crate::Result<()> {
        let mut req = Request::new(Method::Get, Url::parse("http://example.com").unwrap());
        req.insert_header(RANGE, "foo=1-5");
        let ranges = ByteRanges::from_headers(req)?;
        assert_eq!(ranges, None);
        Ok(())
    }

    #[test]
    fn byte_ranges_multi_range() -> crate::Result<()> {
        let mut req = Request::new(Method::Get, Url::parse("http://example.com").unwrap());
        req.insert_header(RANGE, "bytes=1-5, -5");
        let mut ranges = ByteRanges::from_headers(req)?.unwrap().into_iter();
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges.next(), Some(ByteRange::new(1, 5)));
        assert_eq!(ranges.next(), Some(ByteRange::new(None, 5)));
        Ok(())
    }

    #[test]
    fn byte_ranges_apply_single_range() -> crate::Result<()> {
        let mut ranges = ByteRanges::new();
        ranges.push(1, 5);
        let mut req = Request::new(Method::Get, Url::parse("http://example.com").unwrap());
        ranges.apply(&mut req);
        assert_eq!(req[RANGE], "bytes=1-5");
        Ok(())
    }

    #[test]
    fn byte_ranges_apply_multi_range() -> crate::Result<()> {
        let mut ranges = ByteRanges::new();
        ranges.push(1, 5);
        ranges.push(None, 5);
        let mut req = Request::new(Method::Get, Url::parse("http://example.com").unwrap());
        ranges.apply(&mut req);
        assert_eq!(req[RANGE], "bytes=1-5,-5");
        Ok(())
    }

    #[test]
    #[should_panic(expected = "Invalid Range header for byte ranges")]
    fn byte_ranges_no_match_size() {
        let ranges = ByteRanges::from_str("bytes=1-5, -10").unwrap();
        ranges.match_size(6).unwrap();
    }

    #[test]
    fn byte_ranges_match_size() {
        let ranges = ByteRanges::from_str("bytes=1-5, -10").unwrap();
        ranges.match_size(11).unwrap();
    }
}
