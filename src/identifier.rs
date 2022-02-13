use std::error::Error;
use std::fmt;
use std::str::{from_utf8, FromStr};

/// Maximum value of 28-bit counter field.
pub const MAX_COUNTER: u32 = 0xFFF_FFFF;

/// Maximum value of 24-bit per_sec_random field.
pub const MAX_PER_SEC_RANDOM: u32 = 0xFF_FFFF;

/// Digit characters used in the base 32 notation.
const DIGITS: &[u8; 32] = b"0123456789ABCDEFGHIJKLMNOPQRSTUV";

/// O(1) map from ASCII code points to base 32 digit values.
const DECODE_MAP: [u8; 256] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
    0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
    0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
];

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
        if str_value.len() != 26 {
            return Err(ParseError {});
        }
        let bs = str_value.as_bytes();
        let mut int_value = DECODE_MAP[bs[0] as usize] as u128;
        if int_value > 7 {
            return Err(ParseError {});
        }
        for i in 1..26 {
            let n = DECODE_MAP[bs[i] as usize] as u128;
            if n == 0xff {
                return Err(ParseError {});
            }
            int_value = (int_value << 5) | n;
        }
        Ok(Self(int_value))
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
pub struct ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid string representation")
    }
}

impl Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::Scru128Id;
    use crate::Scru128Generator;

    const MAX_UINT44: u64 = (1 << 44) - 1;
    const MAX_UINT28: u32 = (1 << 28) - 1;
    const MAX_UINT24: u32 = (1 << 24) - 1;
    const MAX_UINT32: u32 = u32::MAX;

    /// Encodes and decodes prepared cases correctly
    #[test]
    fn it_encodes_and_decodes_prepared_cases_correctly() {
        let cases: Vec<((u64, u32, u32, u32), &str)> = vec![
            ((0, 0, 0, 0), "00000000000000000000000000"),
            ((MAX_UINT44, 0, 0, 0), "7VVVVVVVVG0000000000000000"),
            ((MAX_UINT44, 0, 0, 0), "7vvvvvvvvg0000000000000000"),
            ((0, MAX_UINT28, 0, 0), "000000000FVVVVU00000000000"),
            ((0, MAX_UINT28, 0, 0), "000000000fvvvvu00000000000"),
            ((0, 0, MAX_UINT24, 0), "000000000000001VVVVS000000"),
            ((0, 0, MAX_UINT24, 0), "000000000000001vvvvs000000"),
            ((0, 0, 0, MAX_UINT32), "00000000000000000003VVVVVV"),
            ((0, 0, 0, MAX_UINT32), "00000000000000000003vvvvvv"),
            (
                (MAX_UINT44, MAX_UINT28, MAX_UINT24, MAX_UINT32),
                "7VVVVVVVVVVVVVVVVVVVVVVVVV",
            ),
            (
                (MAX_UINT44, MAX_UINT28, MAX_UINT24, MAX_UINT32),
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
        let cases = vec![
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

    /// Has symmetric converters from/to various values
    #[test]
    fn it_has_symmetric_converters() {
        let mut cases = vec![
            Scru128Id::from_fields(0, 0, 0, 0),
            Scru128Id::from_fields(MAX_UINT44, 0, 0, 0),
            Scru128Id::from_fields(0, MAX_UINT28, 0, 0),
            Scru128Id::from_fields(0, 0, MAX_UINT24, 0),
            Scru128Id::from_fields(0, 0, 0, MAX_UINT32),
            Scru128Id::from_fields(MAX_UINT44, MAX_UINT28, MAX_UINT24, MAX_UINT32),
        ];

        let mut g = Scru128Generator::new();
        for _ in 0..1000 {
            cases.push(g.generate());
        }

        for e in cases {
            assert_eq!(e.to_string().parse::<Scru128Id>(), Ok(e));
            assert_eq!(Scru128Id::from_u128(e.as_u128()), e);
            assert_eq!(
                Scru128Id::from_fields(
                    e.timestamp(),
                    e.counter(),
                    e.per_sec_random(),
                    e.per_gen_random()
                ),
                e
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
            Scru128Id::from_fields(0, 0, 0, MAX_UINT32),
            Scru128Id::from_fields(0, 0, 1, 0),
            Scru128Id::from_fields(0, 0, MAX_UINT24, 0),
            Scru128Id::from_fields(0, 1, 0, 0),
            Scru128Id::from_fields(0, MAX_UINT28, 0, 0),
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
