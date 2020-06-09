use crate::{Error, StatusCode};

use std::fmt::{self, Debug, Display};
use std::str::FromStr;

/// The representation of a single HTTP byte range.
///
/// # Specifications
///
/// - [RFC 7233, section 2.1: Range](https://tools.ietf.org/html/rfc7233#section-2.1)
/// - [RFC 7233, Appendix D: Collected ABNF](https://tools.ietf.org/html/rfc7233#appendix-D)
#[derive(Default, Debug, Clone, Eq, PartialEq, Copy)]
pub struct ByteRange {
    /// The range start.
    ///
    /// If empty the ends indicates a relative start
    /// from the end of the document.
    pub start: Option<u64>,
    /// The range end.
    ///
    /// If empty the range goes through the end
    /// of the document.
    pub end: Option<u64>,
}

impl ByteRange {
    /// Create a new instance with start and end.
    pub fn new<S, E>(start: S, end: E) -> Self
    where
        S: Into<Option<u64>>,
        E: Into<Option<u64>>,
    {
        Self {
            start: start.into(),
            end: end.into(),
        }
    }

    /// Returns true if the range's bounds match the given document size.
    pub fn match_size(&self, size: u64) -> bool {
        if let Some(start) = self.start {
            if start > size - 1 {
                return false;
            }
        }
        if let Some(end) = self.end {
            if end > size - 1 {
                return false;
            }
        }
        true
    }
}

impl Display for ByteRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}-{}",
            self.start
                .map(|v| v.to_string())
                .unwrap_or_else(String::new),
            self.end.map(|v| v.to_string()).unwrap_or_else(String::new),
        )
    }
}

impl FromStr for ByteRange {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fn_err = || {
            Err(Error::from_str(
                StatusCode::RequestedRangeNotSatisfiable,
                "Invalid Range header for byte ranges",
            ))
        };

        let mut s = s.trim().splitn(2, '-');
        let start = str_to_bound(s.next())?;
        let end = str_to_bound(s.next())?;

        if start.is_none() && end.is_none() {
            return fn_err();
        }

        if let Some(start) = start {
            if let Some(end) = end {
                if end <= start {
                    return fn_err();
                }
            }
        }

        Ok(ByteRange::new(start, end))
    }
}

fn str_to_bound(s: Option<&str>) -> crate::Result<Option<u64>> {
    s.and_then(|s| if s.is_empty() { None } else { Some(s) })
        .map(|s| {
            u64::from_str(s).map_err(|_| {
                Error::from_str(
                    StatusCode::RequestedRangeNotSatisfiable,
                    "Invalid Range header for byte ranges",
                )
            })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::ByteRange;
    use std::str::FromStr;

    #[test]
    fn byte_range_start_end() -> crate::Result<()> {
        let range = ByteRange::from_str("1-5")?;
        assert_eq!(range, ByteRange::new(1, 5));
        Ok(())
    }

    #[test]
    fn byte_range_start_no_end() -> crate::Result<()> {
        let range = ByteRange::from_str("1-")?;
        assert_eq!(range, ByteRange::new(1, None));
        Ok(())
    }

    #[test]
    fn byte_range_no_start_end() -> crate::Result<()> {
        let range = ByteRange::from_str("-5")?;
        assert_eq!(range, ByteRange::new(None, 5));
        Ok(())
    }

    #[test]
    #[should_panic(expected = "Invalid Range header for byte ranges")]
    fn byte_range_no_start_no_end() {
        ByteRange::from_str("-").unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid Range header for byte ranges")]
    fn byte_range_start_after_end() {
        ByteRange::from_str("3-1").unwrap();
    }

    #[test]
    #[should_panic(expected = "Invalid Range header for byte ranges")]
    fn byte_range_invalid_integer() {
        ByteRange::from_str("abc-5").unwrap();
    }

    #[test]
    fn byte_range_match_size() {
        let range = ByteRange::new(0, 4);
        assert_eq!(range.match_size(5), true);
    }

    #[test]
    fn byte_range_not_match_size() {
        let range = ByteRange::new(0, 4);
        assert_eq!(range.match_size(3), false);
    }

    #[test]
    fn byte_range_not_match_size_start() {
        let range = ByteRange::new(4, None);
        assert_eq!(range.match_size(4), false);
    }

    #[test]
    fn byte_range_not_match_size_end() {
        let range = ByteRange::new(None, 5);
        assert_eq!(range.match_size(5), false);
    }
}
