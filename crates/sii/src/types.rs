//! Base types: `Rut`, `Monto`, and core domain identifiers.

use crate::error::Error;
use std::fmt;
use std::str::FromStr;

pub use crate::codes::{Ambiente, TipoDte};

/// Computes the check digit (DV) for a Chilean RUT number using modulo 11.
///
/// Returns `'0'`..=`'9'` or `'K'`.
///
/// # Example
///
/// ```
/// use sii::types::compute_dv;
///
/// assert_eq!(compute_dv(76_086_428), '5');
/// assert_eq!(compute_dv(12_345_678), '5');
/// assert_eq!(compute_dv(6), 'K');
/// ```
#[must_use]
pub fn compute_dv(mut number: u32) -> char {
    let mut sum: u32 = 0;
    let mut multiplier: u32 = 2;

    while number > 0 {
        let digit = number % 10;
        sum += digit * multiplier;
        multiplier = if multiplier == 7 { 2 } else { multiplier + 1 };
        number /= 10;
    }

    let remainder = sum % 11;
    match 11 - remainder {
        11 => '0',
        10 => 'K',
        dv => {
            let offset = u8::try_from(dv).unwrap_or(0);
            char::from(b'0' + offset)
        }
    }
}

/// Chilean taxpayer identifier (Rol Único Tributario / Rol Único Nacional).
///
/// Under Chilean law and SII specifications, a RUT consists of a body number
/// (up to 8 digits) and a check digit calculated using modulo 11 (DTE-04).
///
/// Canonical display representation is `########-D` (without dots, with hyphen,
/// and uppercase `'K'` when applicable).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rut {
    number: u32,
    dv: char,
}

impl Rut {
    /// Creates a new `Rut` by validating that the number and check digit match modulo 11.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidRut`] if `number` is 0, exceeds 99,999,999, or if
    /// `dv` does not match the computed check digit.
    pub fn new(number: u32, dv: char) -> Result<Self, Error> {
        if number == 0 || number > 99_999_999 {
            return Err(Error::InvalidRut);
        }
        let normalized_dv = match dv {
            '0'..='9' => dv,
            'k' | 'K' => 'K',
            _ => return Err(Error::InvalidRut),
        };
        if compute_dv(number) != normalized_dv {
            return Err(Error::InvalidRut);
        }
        Ok(Self {
            number,
            dv: normalized_dv,
        })
    }

    /// Creates a valid `Rut` from a body number, computing its verification digit.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidRut`] if `number` is 0 or exceeds 99,999,999.
    pub fn from_number(number: u32) -> Result<Self, Error> {
        if number == 0 || number > 99_999_999 {
            return Err(Error::InvalidRut);
        }
        Ok(Self {
            number,
            dv: compute_dv(number),
        })
    }

    /// Parses a RUT string in canonical format (`"12345678-5"`), formatted with dots
    /// (`"12.345.678-5"`), or with lower/uppercase `'k'`.
    ///
    /// A single hyphen `-` separating the body and verification digit is required.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidRut`] if the string is malformed or the check digit is invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use sii::types::Rut;
    ///
    /// let rut = Rut::parse("76.086.428-5")?;
    /// assert_eq!(rut.number(), 76_086_428);
    /// assert_eq!(rut.dv(), '5');
    /// assert_eq!(rut.to_string(), "76086428-5");
    /// # Ok::<(), sii::Error>(())
    /// ```
    pub fn parse(s: &str) -> Result<Self, Error> {
        let trimmed = s.trim();
        if trimmed.bytes().filter(|&b| b == b'-').count() != 1 {
            return Err(Error::InvalidRut);
        }
        let (body_part, dv_part) = trimmed.rsplit_once('-').ok_or(Error::InvalidRut)?;
        let dv_trimmed = dv_part.trim();
        if dv_trimmed.len() != 1 {
            return Err(Error::InvalidRut);
        }
        let dv_char = dv_trimmed.chars().next().ok_or(Error::InvalidRut)?;
        let normalized_dv = match dv_char {
            '0'..='9' => dv_char,
            'k' | 'K' => 'K',
            _ => return Err(Error::InvalidRut),
        };

        let number = parse_body(body_part.trim())?;
        if compute_dv(number) != normalized_dv {
            return Err(Error::InvalidRut);
        }

        Ok(Self {
            number,
            dv: normalized_dv,
        })
    }

    /// Returns the body number of the RUT.
    #[inline]
    #[must_use]
    pub const fn number(&self) -> u32 {
        self.number
    }

    /// Alias for [`Rut::number`].
    #[inline]
    #[must_use]
    pub const fn body(&self) -> u32 {
        self.number
    }

    /// Returns the verification check digit (`'0'`..=`'9'` or `'K'`).
    #[inline]
    #[must_use]
    pub const fn dv(&self) -> char {
        self.dv
    }

    /// Formats the RUT with thousands separator dots and hyphen (e.g. `"12.345.678-5"`).
    #[must_use]
    pub fn format_with_dots(&self) -> String {
        let num_str = self.number.to_string();
        let mut res = String::with_capacity(num_str.len() + 4);
        let first_group_len = match num_str.len() % 3 {
            0 => 3,
            r => r,
        };

        for (count, c) in num_str.chars().enumerate() {
            if count == first_group_len
                || (count > first_group_len && (count - first_group_len) % 3 == 0)
            {
                res.push('.');
            }
            res.push(c);
        }
        res.push('-');
        res.push(self.dv);
        res
    }
}

fn parse_body(body_str: &str) -> Result<u32, Error> {
    if body_str.is_empty() {
        return Err(Error::InvalidRut);
    }

    for c in body_str.chars() {
        if !c.is_ascii_digit() && c != '.' {
            return Err(Error::InvalidRut);
        }
    }

    if body_str.contains('.') {
        if body_str.starts_with('.') || body_str.ends_with('.') || body_str.contains("..") {
            return Err(Error::InvalidRut);
        }
        let parts: Vec<&str> = body_str.split('.').collect();
        let first = parts.first().ok_or(Error::InvalidRut)?;
        if first.is_empty() || first.len() > 3 {
            return Err(Error::InvalidRut);
        }
        for part in parts.iter().skip(1) {
            if part.len() != 3 {
                return Err(Error::InvalidRut);
            }
        }
    }

    let mut number: u32 = 0;
    let mut digit_count: usize = 0;
    for c in body_str.chars() {
        if c.is_ascii_digit() {
            let digit = c.to_digit(10).ok_or(Error::InvalidRut)?;
            number = number
                .checked_mul(10)
                .and_then(|n| n.checked_add(digit))
                .ok_or(Error::InvalidRut)?;
            digit_count += 1;
        }
    }

    if digit_count == 0 || digit_count > 8 || number == 0 {
        return Err(Error::InvalidRut);
    }

    if digit_count > 1 && body_str.starts_with('0') {
        return Err(Error::InvalidRut);
    }

    Ok(number)
}

impl fmt::Display for Rut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.number, self.dv)
    }
}

impl FromStr for Rut {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for Rut {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<String> for Rut {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s)
    }
}

/// Amount in Chilean Pesos (CLP).
///
/// In Chilean tax law (SII), all primary monetary values on electronic tax documents
/// (net amount, IVA, exempt amount, total) are represented as whole integer pesos
/// without decimal fractions (DTE-03).
///
/// Floating-point math (`f64`) is strictly forbidden for currency. All arithmetic
/// is checked for overflow to guarantee safe calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Monto(i64);

impl Monto {
    /// Zero pesos.
    pub const ZERO: Self = Self(0);

    /// Creates a new `Monto` from an integer amount of Chilean Pesos.
    ///
    /// # Example
    ///
    /// ```
    /// use sii::types::Monto;
    ///
    /// let monto = Monto::from_pesos(100_000);
    /// assert_eq!(monto.as_pesos(), 100_000);
    /// ```
    #[inline]
    #[must_use]
    pub const fn from_pesos(pesos: i64) -> Self {
        Self(pesos)
    }

    /// Returns the amount as integer pesos (`i64`).
    #[inline]
    #[must_use]
    pub const fn as_pesos(&self) -> i64 {
        self.0
    }

    /// Returns `true` if the amount is zero.
    #[inline]
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Returns `true` if the amount is positive.
    #[inline]
    #[must_use]
    pub const fn is_positive(&self) -> bool {
        self.0 > 0
    }

    /// Returns `true` if the amount is negative.
    #[inline]
    #[must_use]
    pub const fn is_negative(&self) -> bool {
        self.0 < 0
    }

    /// Parses a string representation of an integer peso amount.
    ///
    /// Accepts optional thousands separator dots `.` and leading sign `+` or `-`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidMonto`] if the string contains non-numeric characters
    /// (other than allowed signs and dots) or overflows `i64`.
    pub fn parse(s: &str) -> Result<Self, Error> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(Error::InvalidMonto("empty amount string".to_string()));
        }

        let clean: String = trimmed.chars().filter(|&c| c != '.').collect();
        clean
            .parse::<i64>()
            .map(Self)
            .map_err(|e| Error::InvalidMonto(e.to_string()))
    }

    /// Checked integer addition. Computes `self + rhs`, returning `None` if overflow occurred.
    #[must_use]
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        match self.0.checked_add(rhs.0) {
            Some(res) => Some(Self(res)),
            None => None,
        }
    }

    /// Checked integer subtraction. Computes `self - rhs`, returning `None` if overflow occurred.
    #[must_use]
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        match self.0.checked_sub(rhs.0) {
            Some(res) => Some(Self(res)),
            None => None,
        }
    }

    /// Checked integer multiplication by a scalar factor.
    #[must_use]
    pub const fn checked_mul(self, factor: i64) -> Option<Self> {
        match self.0.checked_mul(factor) {
            Some(res) => Some(Self(res)),
            None => None,
        }
    }

    /// Checked integer division by a scalar divisor.
    #[must_use]
    pub const fn checked_div(self, divisor: i64) -> Option<Self> {
        if divisor == 0 {
            None
        } else {
            match self.0.checked_div(divisor) {
                Some(res) => Some(Self(res)),
                None => None,
            }
        }
    }

    /// Adds another `Monto`, returning [`Error::InvalidMonto`] on overflow.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidMonto`] if the addition overflows.
    pub fn add_monto(self, rhs: Self) -> Result<Self, Error> {
        self.checked_add(rhs)
            .ok_or_else(|| Error::InvalidMonto("overflow in addition".to_string()))
    }

    /// Subtracts another `Monto`, returning [`Error::InvalidMonto`] on overflow.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidMonto`] if the subtraction overflows.
    pub fn sub_monto(self, rhs: Self) -> Result<Self, Error> {
        self.checked_sub(rhs)
            .ok_or_else(|| Error::InvalidMonto("overflow in subtraction".to_string()))
    }

    /// Multiplies by a scalar factor, returning [`Error::InvalidMonto`] on overflow.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidMonto`] if the multiplication overflows.
    pub fn mul_scalar(self, factor: i64) -> Result<Self, Error> {
        self.checked_mul(factor)
            .ok_or_else(|| Error::InvalidMonto("overflow in multiplication".to_string()))
    }

    /// Computes 19 % IVA on this amount using half-up integer rounding.
    ///
    /// According to SII regulations (DTE-02, DTE-03), IVA is 19 % rounded to the
    /// nearest integer peso (`round(neto * 0.19)`).
    ///
    /// Returns `None` if the multiplication or rounding overflows.
    #[must_use]
    pub const fn checked_iva_19(self) -> Option<Self> {
        let Some(product) = self.0.checked_mul(19) else {
            return None;
        };

        if product >= 0 {
            let Some(rounded) = product.checked_add(50) else {
                return None;
            };
            Some(Self(rounded / 100))
        } else {
            let Some(rounded) = product.checked_sub(50) else {
                return None;
            };
            Some(Self(rounded / 100))
        }
    }

    /// Computes 19 % IVA on this amount, returning an error on overflow.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidMonto`] if the arithmetic overflows.
    ///
    /// # Example
    ///
    /// ```
    /// use sii::types::Monto;
    ///
    /// let neto = Monto::from_pesos(100_000);
    /// let iva = neto.iva_19()?;
    /// assert_eq!(iva, Monto::from_pesos(19_000));
    /// # Ok::<(), sii::Error>(())
    /// ```
    pub fn iva_19(self) -> Result<Self, Error> {
        self.checked_iva_19()
            .ok_or_else(|| Error::InvalidMonto("overflow calculating 19% IVA".to_string()))
    }

    /// Computes the gross total including 19 % IVA (`neto + iva_19(neto)`).
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidMonto`] if calculating IVA or adding it overflows.
    pub fn total_con_iva(self) -> Result<Self, Error> {
        let iva = self.iva_19()?;
        self.add_monto(iva)
    }
}

impl fmt::Display for Monto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Monto {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl From<i64> for Monto {
    #[inline]
    fn from(pesos: i64) -> Self {
        Self(pesos)
    }
}

impl From<u32> for Monto {
    #[inline]
    fn from(pesos: u32) -> Self {
        Self(i64::from(pesos))
    }
}

impl TryFrom<u64> for Monto {
    type Error = Error;

    fn try_from(pesos: u64) -> Result<Self, Self::Error> {
        i64::try_from(pesos)
            .map(Self)
            .map_err(|_| Error::InvalidMonto("amount exceeds i64::MAX".to_string()))
    }
}

impl TryFrom<usize> for Monto {
    type Error = Error;

    fn try_from(pesos: usize) -> Result<Self, Self::Error> {
        i64::try_from(pesos)
            .map(Self)
            .map_err(|_| Error::InvalidMonto("amount exceeds i64::MAX".to_string()))
    }
}

/// Clock trait for providing date and time without system I/O (ADR-01, QA-03).
///
/// Implemented by types providing temporal context, enabling deterministic testing.
pub trait Clock: Send + Sync {
    /// Returns the current date and time with UTC offset.
    fn now(&self) -> time::OffsetDateTime;
}

/// A manual, fixed clock useful for deterministic testing and reproducible stamps (QA-03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualClock {
    time: time::OffsetDateTime,
}

impl ManualClock {
    /// Creates a new `ManualClock` initialized to the given date and time.
    #[must_use]
    pub const fn new(time: time::OffsetDateTime) -> Self {
        Self { time }
    }

    /// Sets or advances the clock to a new date and time.
    pub fn set_time(&mut self, time: time::OffsetDateTime) {
        self.time = time;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> time::OffsetDateTime {
        self.time
    }
}

impl Clock for time::OffsetDateTime {
    fn now(&self) -> time::OffsetDateTime {
        *self
    }
}

impl Clock for time::PrimitiveDateTime {
    fn now(&self) -> time::OffsetDateTime {
        self.assume_utc()
    }
}

impl<T: Clock + ?Sized> Clock for &T {
    fn now(&self) -> time::OffsetDateTime {
        (*self).now()
    }
}

/// Formats a `time::OffsetDateTime` as `YYYY-MM-DDTHH:MM:SS`.
#[must_use]
pub fn format_timestamp(dt: &time::OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        dt.year(),
        dt.month() as u8,
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second()
    )
}

/// Parses a `YYYY-MM-DDTHH:MM:SS` string into a `time::OffsetDateTime` (UTC).
///
/// # Errors
///
/// Returns [`Error::InvalidDate`] if the timestamp string cannot be parsed.
pub fn parse_timestamp(s: &str) -> Result<time::OffsetDateTime, Error> {
    let trimmed = s.trim();
    let parts: Vec<&str> = trimmed.split('T').collect();
    if parts.len() != 2 {
        return Err(Error::InvalidDate(format!(
            "expected YYYY-MM-DDTHH:MM:SS format, got: {s}"
        )));
    }
    let date_str = parts
        .first()
        .ok_or_else(|| Error::InvalidDate(s.to_string()))?;
    let time_str = parts
        .get(1)
        .ok_or_else(|| Error::InvalidDate(s.to_string()))?;

    let date = crate::dte::parse_date(date_str)?;

    let time_parts: Vec<&str> = time_str.split(':').collect();
    if time_parts.len() != 3 {
        return Err(Error::InvalidDate(format!(
            "expected HH:MM:SS time format in timestamp, got: {time_str}"
        )));
    }
    let h: u8 = time_parts
        .first()
        .ok_or_else(|| Error::InvalidDate(s.to_string()))?
        .parse()
        .map_err(|e| Error::InvalidDate(format!("invalid hour in {time_str}: {e}")))?;
    let m: u8 = time_parts
        .get(1)
        .ok_or_else(|| Error::InvalidDate(s.to_string()))?
        .parse()
        .map_err(|e| Error::InvalidDate(format!("invalid minute in {time_str}: {e}")))?;
    let sec: u8 = time_parts
        .get(2)
        .ok_or_else(|| Error::InvalidDate(s.to_string()))?
        .parse()
        .map_err(|e| Error::InvalidDate(format!("invalid second in {time_str}: {e}")))?;

    let time = time::Time::from_hms(h, m, sec)
        .map_err(|e| Error::InvalidDate(format!("invalid time {time_str}: {e}")))?;

    Ok(date.with_time(time).assume_utc())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_rut_valid_cases() {
        let cases = [
            ("76.086.428-5", 76_086_428, '5'),
            ("76086428-5", 76_086_428, '5'),
            ("11.111.111-1", 11_111_111, '1'),
            ("11111111-1", 11_111_111, '1'),
            ("12.345.678-5", 12_345_678, '5'),
            ("12345678-5", 12_345_678, '5'),
            ("5.126.663-3", 5_126_663, '3'),
            ("5126663-3", 5_126_663, '3'),
            ("6-k", 6, 'K'),
            ("6-K", 6, 'K'),
            ("1-9", 1, '9'),
            ("  76.086.428-5  ", 76_086_428, '5'),
        ];

        for (input, expected_num, expected_dv) in cases {
            let rut = Rut::parse(input).unwrap_or_else(|_| panic!("failed on {input}"));
            assert_eq!(rut.number(), expected_num);
            assert_eq!(rut.dv(), expected_dv);
            assert_eq!(rut.body(), expected_num);
        }
    }

    #[test]
    fn test_rut_canonical_display_and_dots() {
        let rut = Rut::parse("76.086.428-5").unwrap();
        assert_eq!(rut.to_string(), "76086428-5");
        assert_eq!(rut.format_with_dots(), "76.086.428-5");

        let rut_k = Rut::parse("6-k").unwrap();
        assert_eq!(rut_k.to_string(), "6-K");
        assert_eq!(rut_k.format_with_dots(), "6-K");
    }

    #[test]
    fn test_rut_invalid_cases() {
        let invalid = [
            "",
            "   ",
            "abc",
            "12a45678-5",
            "76.123.456-7", // Wrong DV
            "12.345.678-9", // Wrong DV
            "11.111.111-2", // Wrong DV
            "01234567-8",   // Leading zero
            "12345678",     // Missing hyphen
            "12-34-5",      // Multiple hyphens
            "0-0",          // Zero body
            "-5",           // Empty body
            "12345678-",    // Empty DV
            "12345678-55",  // Multi-char DV
            "12.3456.78-5", // Misplaced dots
            "1234567890-1", // Too many digits
        ];

        for input in invalid {
            assert!(
                Rut::parse(input).is_err(),
                "should have failed for: {input}"
            );
        }
    }

    #[test]
    fn test_monto_iva_19_exact() {
        let cases = [
            (0, 0),
            (1, 0),
            (3, 1),
            (8, 2),
            (15, 3),
            (25, 5),
            (100, 19),
            (1_000, 190),
            (5_000, 950),
            (123_456, 23_457),
            (-100, -19),
            (-123_456, -23_457),
        ];

        for (neto, expected_iva) in cases {
            let m = Monto::from_pesos(neto);
            let iva = m.iva_19().unwrap();
            assert_eq!(iva.as_pesos(), expected_iva, "IVA mismatch for neto {neto}");

            let total = m.total_con_iva().unwrap();
            assert_eq!(
                total.as_pesos(),
                neto + expected_iva,
                "Total mismatch for neto {neto}"
            );
        }
    }

    #[test]
    fn test_monto_overflow_safety() {
        let max = Monto::from_pesos(i64::MAX);
        let min = Monto::from_pesos(i64::MIN);
        let one = Monto::from_pesos(1);

        assert!(max.add_monto(one).is_err());
        assert!(min.sub_monto(one).is_err());
        assert!(max.mul_scalar(2).is_err());
        assert!(max.iva_19().is_err());

        assert_eq!(max.checked_add(one), None);
        assert_eq!(min.checked_sub(one), None);
        assert_eq!(max.checked_mul(2), None);
        assert_eq!(one.checked_div(0), None);
    }

    #[test]
    fn test_monto_parsing() {
        assert_eq!(Monto::parse("12345").unwrap(), Monto::from_pesos(12345));
        assert_eq!(Monto::parse("12.345").unwrap(), Monto::from_pesos(12345));
        assert_eq!(Monto::parse("-12.345").unwrap(), Monto::from_pesos(-12345));
        assert_eq!(Monto::parse("0").unwrap(), Monto::ZERO);
        assert!(Monto::parse("abc").is_err());
        assert!(Monto::parse("").is_err());
    }

    proptest! {
        #[test]
        fn test_rut_modulo_11_proptest(body in 1u32..=99_999_999u32) {
            let dv = compute_dv(body);
            let rut = Rut::new(body, dv).expect("Rut::new with computed DV must succeed");
            prop_assert_eq!(rut.number(), body);
            prop_assert_eq!(rut.dv(), dv);

            // Canonical round-trip
            let canonical = rut.to_string();
            let parsed = Rut::parse(&canonical).expect("Rut::parse canonical must succeed");
            prop_assert_eq!(rut, parsed);

            // Formatted with dots round-trip
            let with_dots = rut.format_with_dots();
            let parsed_dots = Rut::parse(&with_dots).expect("Rut::parse with dots must succeed");
            prop_assert_eq!(rut, parsed_dots);
        }

        #[test]
        fn test_rut_invalid_dv_rejected_proptest(body in 1u32..=99_999_999u32, wrong_dv in "[0-9K]") {
            let correct_dv = compute_dv(body);
            let wrong_char = wrong_dv.chars().next().unwrap();
            if wrong_char != correct_dv {
                prop_assert!(Rut::new(body, wrong_char).is_err());
                let raw = format!("{body}-{wrong_char}");
                prop_assert!(Rut::parse(&raw).is_err());
            }
        }
    }
}
