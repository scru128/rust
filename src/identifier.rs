use std::convert::TryFrom;
use std::error::Error;
use std::fmt;
use std::str::{from_utf8, FromStr};

/// Maximum value of 28-bit counter field.
pub const MAX_COUNTER: u32 = 0xFFF_FFFF;

/// Maximum value of 24-bit per_sec_random field.
pub const MAX_PER_SEC_RANDOM: u32 = 0xFF_FFFF;

/// Digit characters used in the base 32 notation.
const DIGITS: &[u8; 32] = b"0123456789ABCDEFGHIJKLMNOPQRSTUV";

/// Represents a SCRU128 ID and provides various converters and comparison operators.
///
/// # Examples
///
/// ```rust
/// use scru128::Scru128Id;
///
/// let x = "00Q1D9AB6DTJNLJ80SJ42SNJ4F".parse::<Scru128Id>()?;
/// assert_eq!(x.to_string(), "00Q1D9AB6DTJNLJ80SJ42SNJ4F");
///
/// let y = Scru128Id::from_u128(0x00d05a952ccdecef5aa01c9904e5a115);
/// assert_eq!(y.as_u128(), 0x00d05a952ccdecef5aa01c9904e5a115);
/// # Ok::<(), scru128::ParseError>(())
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "String", try_from = "String")
)]
pub struct Scru128Id(u128);

impl Scru128Id {
    /// Creates an object from a 128-bit unsigned integer.
    pub const fn from_u128(int_value: u128) -> Self {
        Self(int_value)
    }

    /// Returns the 128-bit unsigned integer representation.
    pub const fn as_u128(&self) -> u128 {
        self.0
    }

    /// Creates an object from field values.
    ///
    /// # Panics
    ///
    /// Panics if any argument is out of the value range of the field.
    pub fn from_fields(
        timestamp: u64,
        counter: u32,
        per_sec_random: u32,
        per_gen_random: u32,
    ) -> Self {
        if timestamp > 0xFFF_FFFF_FFFF
            || counter > MAX_COUNTER
            || per_sec_random > MAX_PER_SEC_RANDOM
        {
            panic!("invalid field value");
        } else {
            Self(
                ((timestamp as u128) << 84)
                    | ((counter as u128) << 56)
                    | ((per_sec_random as u128) << 32)
                    | (per_gen_random as u128),
            )
        }
    }

    /// Returns the 44-bit millisecond timestamp field value.
    pub fn timestamp(&self) -> u64 {
        (self.0 >> 84) as u64
    }

    /// Returns the 28-bit per-timestamp monotonic counter field value.
    pub fn counter(&self) -> u32 {
        (self.0 >> 56) as u32 & MAX_COUNTER
    }

    /// Returns the 24-bit per-second randomness field value.
    pub fn per_sec_random(&self) -> u32 {
        (self.0 >> 32) as u32 & MAX_PER_SEC_RANDOM
    }

    /// Returns the 32-bit per-generation randomness field value.
    pub fn per_gen_random(&self) -> u32 {
        self.0 as u32 & u32::MAX
    }
}

impl FromStr for Scru128Id {
    type Err = ParseError;

    /// Creates an object from a 26-digit string representation.
    fn from_str(str_value: &str) -> Result<Self, Self::Err> {
        let mut cs = str_value.chars();
        if str_value.len() == 26
            && cs.next().map_or(false, |c| c.is_digit(8))
            && cs.all(|c| c.is_digit(32))
        {
            Ok(Self(u128::from_str_radix(str_value, 32).unwrap()))
        } else {
            Err(ParseError(str_value.into()))
        }
    }
}

impl fmt::Display for Scru128Id {
    /// Returns the 26-digit canonical string representation.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buffer = [b'0'; 26];
        let mut n = self.0;
        for i in 0..26 {
            buffer[25 - i] = DIGITS[(n & 31) as usize];
            n >>= 5;
        }
        f.write_str(from_utf8(&buffer).unwrap())
    }
}

impl From<u128> for Scru128Id {
    fn from(value: u128) -> Self {
        Self::from_u128(value)
    }
}

impl From<Scru128Id> for u128 {
    fn from(object: Scru128Id) -> Self {
        object.as_u128()
    }
}

impl TryFrom<String> for Scru128Id {
    type Error = ParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl From<Scru128Id> for String {
    fn from(object: Scru128Id) -> Self {
        object.to_string()
    }
}

/// Error parsing an invalid string representation of SCRU128 ID.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct ParseError(String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid string representation: {}", self.0)
    }
}

impl Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::Scru128Id;
    use crate::Scru128Generator;

    /// Encodes and decodes prepared cases correctly
    #[test]
    fn it_encodes_and_decodes_prepared_cases_correctly() {
        let cases: Vec<((u64, u32, u32, u32), &str)> = vec![
            ((0, 0, 0, 0), "00000000000000000000000000"),
            ((2u64.pow(44) - 1, 0, 0, 0), "7VVVVVVVVG0000000000000000"),
            ((2u64.pow(44) - 1, 0, 0, 0), "7vvvvvvvvg0000000000000000"),
            ((0, 2u32.pow(28) - 1, 0, 0), "000000000FVVVVU00000000000"),
            ((0, 2u32.pow(28) - 1, 0, 0), "000000000fvvvvu00000000000"),
            ((0, 0, 2u32.pow(24) - 1, 0), "000000000000001VVVVS000000"),
            ((0, 0, 2u32.pow(24) - 1, 0), "000000000000001vvvvs000000"),
            ((0, 0, 0, u32::MAX), "00000000000000000003VVVVVV"),
            ((0, 0, 0, u32::MAX), "00000000000000000003vvvvvv"),
            (
                (
                    2u64.pow(44) - 1,
                    2u32.pow(28) - 1,
                    2u32.pow(24) - 1,
                    u32::MAX,
                ),
                "7VVVVVVVVVVVVVVVVVVVVVVVVV",
            ),
            (
                (
                    2u64.pow(44) - 1,
                    2u32.pow(28) - 1,
                    2u32.pow(24) - 1,
                    u32::MAX,
                ),
                "7vvvvvvvvvvvvvvvvvvvvvvvvv",
            ),
        ];

        for e in cases {
            let from_fields = Scru128Id::from_fields(e.0 .0, e.0 .1, e.0 .2, e.0 .3);
            let from_string = e.1.parse::<Scru128Id>().unwrap();

            assert_eq!(from_fields, from_string);
            assert_eq!(
                from_fields.as_u128(),
                u128::from_str_radix(e.1, 32).unwrap()
            );
            assert_eq!(
                from_string.as_u128(),
                u128::from_str_radix(e.1, 32).unwrap()
            );
            assert_eq!(
                (
                    (
                        from_fields.timestamp(),
                        from_fields.counter(),
                        from_fields.per_sec_random(),
                        from_fields.per_gen_random(),
                    ),
                    from_fields.to_string(),
                ),
                (e.0, e.1.to_uppercase()),
            );
            assert_eq!(
                (
                    (
                        from_string.timestamp(),
                        from_string.counter(),
                        from_string.per_sec_random(),
                        from_string.per_gen_random(),
                    ),
                    from_string.to_string(),
                ),
                (e.0, e.1.to_uppercase()),
            );
        }
    }

    /// Returns error if an invalid string representation is supplied
    #[test]
    fn it_returns_error_if_an_invalid_string_representation_is_supplied() {
        let cases = [
            "",
            " 00SCT4FL89GQPRHN44C4LFM0OV",
            "00SCT4FL89GQPRJN44C7SQO381 ",
            " 00SCT4FL89GQPRLN44C4BGCIIO ",
            "+00SCT4FL89GQPRNN44C4F3QD24",
            "-00SCT4FL89GQPRPN44C7H4E5RC",
            "+0SCT4FL89GQPRRN44C55Q7RVC",
            "-0SCT4FL89GQPRTN44C6PN0A2R",
            "00SCT4FL89WQPRVN44C41RGVMM",
            "00SCT4FL89GQPS1N4_C54QDC5O",
            "00SCT4-L89GQPS3N44C602O0K8",
            "00SCT4FL89GQPS N44C7VHS5QJ",
            "80000000000000000000000000",
            "VVVVVVVVVVVVVVVVVVVVVVVVVV",
        ];

        for e in cases {
            assert!(e.parse::<Scru128Id>().is_err());
        }
    }

    /// Has symmetric converters from/to String, u128, and fields
    #[test]
    fn it_has_symmetric_converters() {
        let mut g = Scru128Generator::new();
        for _ in 0..1000 {
            let obj = g.generate();
            assert_eq!(obj.to_string().parse::<Scru128Id>(), Ok(obj));
            assert_eq!(Scru128Id::from_u128(obj.as_u128()), obj);
            assert_eq!(
                Scru128Id::from_fields(
                    obj.timestamp(),
                    obj.counter(),
                    obj.per_sec_random(),
                    obj.per_gen_random()
                ),
                obj
            );
        }
    }

    /// Supports comparison operators
    #[test]
    fn it_supports_comparison_operators() {
        fn hash(v: impl std::hash::Hash) -> u64 {
            use std::{collections::hash_map::DefaultHasher, hash::Hasher};
            let mut hasher = DefaultHasher::new();
            v.hash(&mut hasher);
            hasher.finish()
        }

        let mut ordered = vec![
            Scru128Id::from_fields(0, 0, 0, 0),
            Scru128Id::from_fields(0, 0, 0, 1),
            Scru128Id::from_fields(0, 0, 0, 0xFFFF_FFFF),
            Scru128Id::from_fields(0, 0, 1, 0),
            Scru128Id::from_fields(0, 0, 0xFF_FFFF, 0),
            Scru128Id::from_fields(0, 1, 0, 0),
            Scru128Id::from_fields(0, 0xFFF_FFFF, 0, 0),
            Scru128Id::from_fields(1, 0, 0, 0),
            Scru128Id::from_fields(2, 0, 0, 0),
        ];

        let mut g = Scru128Generator::new();
        for _ in 0..1000 {
            ordered.push(g.generate());
        }

        let mut prev = ordered.remove(0);
        for curr in ordered {
            assert_ne!(curr, prev);
            assert_ne!(prev, curr);
            assert_ne!(hash(curr), hash(prev));
            assert!(curr > prev);
            assert!(curr >= prev);
            assert!(prev < curr);
            assert!(prev <= curr);

            let clone = curr.clone();
            assert_eq!(curr, clone);
            assert_eq!(clone, curr);
            assert_eq!(hash(curr), hash(clone));
            assert!(curr >= clone);
            assert!(clone >= curr);
            assert!(curr <= clone);
            assert!(clone <= curr);

            prev = curr;
        }
    }

    /// Serializes and deserializes an object using the canonical string representation
    #[cfg(feature = "serde")]
    #[test]
    fn it_serializes_and_deserializes_an_object_using_the_canonical_string_representation() {
        use serde_test::{assert_tokens, Token};

        let cases = [
            "00RR040G0H5T4K50QM4KBD772B",
            "00RR040G0H5T4K70QM4LDAO4GF",
            "00RR040G0H5T4K90QM4MHJITIJ",
            "00RR040G0H5T4KB0QM4MTNQHPN",
            "00RR040G0H5T4KD0QM4L2FONUL",
            "00RR040G0H5T4KF0QM4LUGFEM5",
            "00RR040G0H5T4KH0QM4MDCVGPG",
            "00RR040G0H5T4KJ0QM4MFJ3GRS",
        ];

        for e in cases {
            let obj = e.parse::<Scru128Id>().unwrap();
            assert_tokens(&obj, &[Token::String(e)]);
        }
    }
}
