use crate::headers::{HeaderName, HeaderValue, Headers, ToHeaderValues, CONTENT_RANGE};
use crate::range::ByteRange;
use crate::{Error, StatusCode};

use std::fmt::{self, Debug, Display};
use std::option;
use std::str::FromStr;

/// HTTP ContentRange response header with bytes unit.
///
/// The "Content-Range" header field is sent in a single part 206
/// (Partial Content) response to indicate the partial range of the
/// selected representation enclosed as the message payload, sent in each
/// part of a multipart 206 response to indicate the range enclosed
/// within each body part, and sent in 416 (Range Not Satisfiable)
/// responses to provide information about the selected representation.
///
/// # Specifications
///
/// - [RFC 7233, section 4.2: Range](https://tools.ietf.org/html/rfc7233#section-4.2)
///
/// # Examples
///
/// Encoding a Content-Range header for byte range 1-5 of a 10 bytes size document:
///
/// ```
/// # fn main() -> http_types::Result<()> {
/// #
/// use http_types::range::{ByteContentRange, ByteRange};
/// use http_types::{Response, StatusCode};
///
/// let mut content_range = ByteContentRange::new().with_range(1, 5).with_size(10);
///
/// let mut res = Response::new(StatusCode::PartialContent);
/// content_range.apply(&mut res);
///
/// let content_range = ByteContentRange::from_headers(res)?.unwrap();
/// assert_eq!(content_range.range(), Some(ByteRange::new(1, 5)));
/// assert_eq!(content_range.size(), Some(10));
/// #
/// # Ok(()) }
/// ```
///
/// Encoding a Content-Range header for byte range 1-5 with unknown size:
///
/// ```
/// # fn main() -> http_types::Result<()> {
/// #
/// use http_types::range::{ByteContentRange, ByteRange};
/// use http_types::{Response, StatusCode};
///
/// let mut content_range = ByteContentRange::new().with_range(1, 5);
///
/// let mut res = Response::new(StatusCode::PartialContent);
/// content_range.apply(&mut res);
///
/// let content_range = ByteContentRange::from_headers(res)?.unwrap();
/// assert_eq!(content_range.range(), Some(ByteRange::new(1, 5)));
/// assert_eq!(content_range.size(), None);
/// #
/// # Ok(()) }
/// ```
///
/// Responding to an invalid range request for a 10 bytes document:
///
/// ```
/// # fn main() -> http_types::Result<()> {
/// #
/// use http_types::range::{ByteContentRange, ByteRange};
/// use http_types::{Response, StatusCode};
///
/// let mut content_range = ByteContentRange::new().with_size(10);
///
/// let mut res = Response::new(StatusCode::RequestedRangeNotSatisfiable);
/// content_range.apply(&mut res);
///
/// let content_range = ByteContentRange::from_headers(res)?.unwrap();
/// assert_eq!(content_range.range(), None);
/// assert_eq!(content_range.size(), Some(10));
/// #
/// # Ok(()) }
/// ```
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct ByteContentRange {
    size: Option<u64>,
    range: Option<ByteRange>,
}

impl ByteContentRange {
    const CONTENT_RANGE_PREFIX: &'static str = "bytes";

    /// Create a new instance with no range and no size.
    pub fn new() -> Self {
        ByteContentRange::default()
    }

    /// Returns a new instance with a given range defined by `start` and `end` bounds.
    pub fn with_range(mut self, start: u64, end: u64) -> Self {
        self.range = Some(ByteRange::new(start, end));
        self
    }

    /// Returns a new instance with a size.
    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    /// Returns the `ByteRange` if any.
    pub fn range(&self) -> Option<ByteRange> {
        self.range
    }

    /// Returns the size if any.
    pub fn size(&self) -> Option<u64> {
        self.size
    }

    /// Create a new instance from a Content-Range headers.
    ///
    /// Only a single Content-Range per resource is assumed to exist. If multiple Range
    /// headers are found the last one is used.
    pub fn from_headers(headers: impl AsRef<Headers>) -> crate::Result<Option<Self>> {
        let headers = match headers.as_ref().get(CONTENT_RANGE) {
            Some(headers) => headers,
            None => return Ok(None),
        };

        // If a header is returned we can assume at least one exists.
        let s = headers.iter().last().unwrap().as_str();
        if !s.starts_with(Self::CONTENT_RANGE_PREFIX) {
            return Ok(None);
        }
        Self::from_str(s).map(Some)
    }

    /// Create a ByteRanges from a string.
    pub(crate) fn from_str(s: &str) -> crate::Result<Self> {
        let fn_err = || {
            Error::from_str(
                StatusCode::RequestedRangeNotSatisfiable,
                "Invalid Content-Range value",
            )
        };

        if !s.starts_with(Self::CONTENT_RANGE_PREFIX) {
            return Err(fn_err());
        }

        let mut content_range = ByteContentRange::new();

        let s = &mut s[Self::CONTENT_RANGE_PREFIX.len()..]
            .trim_start()
            .splitn(2, '/');

        let range_s = s.next().ok_or_else(fn_err)?;
        if range_s != "*" {
            let range = ByteRange::from_str(range_s).map_err(|_| fn_err())?;
            if range.start.is_none() || range.end.is_none() {
                return Err(fn_err());
            }
            content_range.range.replace(range);
        }

        let size_s = s.next().ok_or_else(fn_err)?;
        if size_s != "*" {
            let size = u64::from_str(size_s).map_err(|_| fn_err())?;
            content_range = content_range.with_size(size);
        }

        if content_range.range.is_none() && content_range.size.is_none() {
            return Err(fn_err());
        }
        if let Some(size) = content_range.size {
            if let Some(end) = content_range.range.and_then(|r| r.end) {
                if size <= end {
                    return Err(fn_err());
                }
            }
        }

        Ok(content_range)
    }

    /// Sets the `Range` header.
    pub fn apply(&self, mut headers: impl AsMut<Headers>) {
        headers.as_mut().insert(CONTENT_RANGE, self.value());
    }

    /// Get the `HeaderName`.
    pub fn name(&self) -> HeaderName {
        CONTENT_RANGE
    }

    /// Get the `HeaderValue`.
    pub fn value(&self) -> HeaderValue {
        let s = self.to_string();
        // SAFETY: the internal string is validated to be ASCII.
        unsafe { HeaderValue::from_bytes_unchecked(s.into()) }
    }
}

impl Display for ByteContentRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "bytes {}/{}",
            self.range
                .map(|r| r.to_string())
                .unwrap_or_else(|| "*".into()),
            self.size
                .map(|s| s.to_string())
                .unwrap_or_else(|| "*".into())
        )
    }
}

impl ToHeaderValues for ByteContentRange {
    type Iter = option::IntoIter<HeaderValue>;
    fn to_header_values(&self) -> crate::Result<Self::Iter> {
        // A HeaderValue will always convert into itself.
        Ok(self.value().to_header_values().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::ByteContentRange;

    use crate::headers::CONTENT_RANGE;
    use crate::range::ByteRange;
    use crate::{Response, StatusCode};

    #[test]
    fn byte_content_range_and_size() -> crate::Result<()> {
        let mut res = Response::new(StatusCode::PartialContent);
        res.insert_header(CONTENT_RANGE, "bytes 1-5/100");
        let content_range = ByteContentRange::from_headers(res)?.unwrap();
        assert_eq!(content_range.range(), Some(ByteRange::new(1, 5)));
        assert_eq!(content_range.size(), Some(100));
        Ok(())
    }

    #[test]
    fn byte_content_range_and_unknown_size() -> crate::Result<()> {
        let mut res = Response::new(StatusCode::PartialContent);
        res.insert_header(CONTENT_RANGE, "bytes 1-5/*");
        let content_range = ByteContentRange::from_headers(res)?.unwrap();
        assert_eq!(content_range.range(), Some(ByteRange::new(1, 5)));
        assert_eq!(content_range.size(), None);
        Ok(())
    }

    #[test]
    fn byte_content_no_range_and_size() -> crate::Result<()> {
        let mut res = Response::new(StatusCode::PartialContent);
        res.insert_header(CONTENT_RANGE, "bytes */100");
        let content_range = ByteContentRange::from_headers(res)?.unwrap();
        assert_eq!(content_range.range(), None);
        assert_eq!(content_range.size(), Some(100));
        Ok(())
    }

    #[test]
    #[should_panic(expected = "Invalid Content-Range value")]
    fn byte_content_no_range_and_no_size() {
        let mut res = Response::new(StatusCode::PartialContent);
        res.insert_header(CONTENT_RANGE, "bytes */*");
        ByteContentRange::from_headers(res).unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid Content-Range value")]
    fn byte_content_invalid_range() {
        let mut res = Response::new(StatusCode::PartialContent);
        res.insert_header(CONTENT_RANGE, "bytes a-b/*");
        ByteContentRange::from_headers(res).unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid Content-Range value")]
    fn byte_content_invalid_size() {
        let mut res = Response::new(StatusCode::PartialContent);
        res.insert_header(CONTENT_RANGE, "bytes */abc");
        ByteContentRange::from_headers(res).unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid Content-Range value")]
    fn byte_content_invalid_range_end() {
        let mut res = Response::new(StatusCode::PartialContent);
        res.insert_header(CONTENT_RANGE, "bytes 5-4/*");
        ByteContentRange::from_headers(res).unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid Content-Range value")]
    fn byte_content_range_overflow_size() {
        let mut res = Response::new(StatusCode::PartialContent);
        res.insert_header(CONTENT_RANGE, "bytes 1-4/3");
        ByteContentRange::from_headers(res).unwrap();
    }

    #[test]
    fn byte_content_apply_range_and_size() {
        let content_range = ByteContentRange::new().with_range(1, 5).with_size(100);
        let mut res = Response::new(StatusCode::PartialContent);
        content_range.apply(&mut res);
        assert_eq!(res[CONTENT_RANGE], "bytes 1-5/100");
    }

    #[test]
    fn byte_content_apply_range_and_no_size() {
        let content_range = ByteContentRange::new().with_range(1, 5);
        let mut res = Response::new(StatusCode::PartialContent);
        content_range.apply(&mut res);
        assert_eq!(res[CONTENT_RANGE], "bytes 1-5/*");
    }

    #[test]
    fn byte_content_apply_no_range_and_size() {
        let content_range = ByteContentRange::new().with_size(100);
        let mut res = Response::new(StatusCode::PartialContent);
        content_range.apply(&mut res);
        assert_eq!(res[CONTENT_RANGE], "bytes */100");
    }
}
