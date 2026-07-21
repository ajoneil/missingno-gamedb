use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

/// A SHA-1 digest in lowercase hex; uppercase input is normalized.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
#[serde(try_from = "String", into = "String")]
pub struct Sha1(String);

impl Sha1 {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Sha1 {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value = value.to_ascii_lowercase();
        if value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit()) {
            Ok(Self(value))
        } else {
            Err(format!("not a 40-digit hex sha1: {value:?}"))
        }
    }
}

impl FromStr for Sha1 {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s.to_owned())
    }
}

impl From<Sha1> for String {
    fn from(sha1: Sha1) -> Self {
        sha1.0
    }
}

impl fmt::Display for Sha1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A game's directory name: lowercase alphanumerics, `-`, `_`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Slug(String);

impl Slug {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Slug {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
        {
            Ok(Self(s.to_owned()))
        } else {
            Err(format!(
                "invalid slug {s:?}: expected lowercase alphanumerics, '-' or '_'"
            ))
        }
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A release date at whatever precision is known: "1998", "1998-03", or "1998-03-01".
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[serde(try_from = "String", into = "String")]
pub struct ReleaseDate(String);

impl ReleaseDate {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The four-digit year prefix.
    pub fn year(&self) -> &str {
        &self.0[..4]
    }
}

fn valid_year_month_day(value: &str) -> (bool, bool, bool) {
    let b = value.as_bytes();
    let digits = |range: std::ops::Range<usize>| b[range].iter().all(u8::is_ascii_digit);
    let two = |at: usize| (b[at] - b'0') * 10 + (b[at + 1] - b'0');
    match b.len() {
        4 => (digits(0..4), false, false),
        7 => (
            digits(0..4) && b[4] == b'-' && digits(5..7) && (1..=12).contains(&two(5)),
            true,
            false,
        ),
        10 => (
            digits(0..4)
                && b[4] == b'-'
                && digits(5..7)
                && (1..=12).contains(&two(5))
                && b[7] == b'-'
                && digits(8..10)
                && (1..=31).contains(&two(8)),
            true,
            true,
        ),
        _ => (false, false, false),
    }
}

impl TryFrom<String> for ReleaseDate {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match valid_year_month_day(&value) {
            (true, ..) => Ok(Self(value)),
            _ => Err(format!(
                "invalid date {value:?}: expected YYYY, YYYY-MM, or YYYY-MM-DD"
            )),
        }
    }
}

/// A precise calendar day, "YYYY-MM-DD" — event stamps, not publication dates.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[serde(try_from = "String", into = "String")]
pub struct Date(String);

impl Date {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Date {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match valid_year_month_day(&value) {
            (true, true, true) => Ok(Self(value)),
            _ => Err(format!("invalid date {value:?}: expected YYYY-MM-DD")),
        }
    }
}

impl FromStr for Date {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s.to_owned())
    }
}

impl From<Date> for String {
    fn from(date: Date) -> Self {
        date.0
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ReleaseDate {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s.to_owned())
    }
}

impl From<ReleaseDate> for String {
    fn from(date: ReleaseDate) -> Self {
        date.0
    }
}

impl fmt::Display for ReleaseDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha1_normalizes_and_validates() {
        let sha1: Sha1 = "D960E951B18D07E79D046313DF49C18313664224".parse().unwrap();
        assert_eq!(sha1.as_str(), "d960e951b18d07e79d046313df49c18313664224");
        assert!("abc".parse::<Sha1>().is_err());
        assert!(
            "z960e951b18d07e79d046313df49c18313664224"
                .parse::<Sha1>()
                .is_err()
        );
    }

    #[test]
    fn slug_charset() {
        assert!("144p-test-suite".parse::<Slug>().is_ok());
        assert!("2brownboyz_restaurant-rumble-demo".parse::<Slug>().is_ok());
        assert!("".parse::<Slug>().is_err());
        assert!("Bad Slug".parse::<Slug>().is_err());
    }

    #[test]
    fn date_requires_full_precision() {
        assert!("2026-07-21".parse::<Date>().is_ok());
        for bad in ["2026", "2026-07", "2026-13-01", "yesterday"] {
            assert!(bad.parse::<Date>().is_err(), "{bad}");
        }
    }

    #[test]
    fn release_date_precision() {
        for ok in ["1998", "1998-03", "1998-03-01"] {
            assert!(ok.parse::<ReleaseDate>().is_ok(), "{ok}");
        }
        for bad in ["98", "1998-13", "1998-3", "1998-03-32", "1998/03/01"] {
            assert!(bad.parse::<ReleaseDate>().is_err(), "{bad}");
        }
    }
}
