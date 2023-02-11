#[cfg(not(feature = "std"))]
use core as std;

use crate::{MAX_COUNTER_HI, MAX_COUNTER_LO, MAX_TIMESTAMP};
use fstr::FStr;
use std::{fmt, str};

/// Digit characters used in the Base36 notation.
const DIGITS: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// O(1) map from ASCII code points to Base36 digit values.
const DECODE_MAP: [u8; 256] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
    0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
    0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
];

/// Represents a SCRU128 ID and provides converters and comparison operators.
///
/// # Examples
///
/// ```rust
/// use scru128::Scru128Id;
///
/// let x = "036Z968FU2TUGY7SVKFZNEWKK".parse::<Scru128Id>()?;
/// assert_eq!(x.to_string(), "036Z968FU2TUGY7SVKFZNEWKK");
///
/// let y = Scru128Id::from(0x017fa1de51a80fd992f9e8cc2d5eb88eu128);
/// assert_eq!(y.to_u128(), 0x017fa1de51a80fd992f9e8cc2d5eb88eu128);
/// # Ok::<(), scru128::ParseError>(())
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Scru128Id(u128);

impl Scru128Id {
    /// Creates an object from a 128-bit unsigned integer.
    ///
    /// Use `Scru128Id::from(u128)` instead out of `const` context. This constructor may be
    /// deprecated in the future once [const trait impls] are stabilized.
    ///
    /// [const trait impls]: https://github.com/rust-lang/rust/issues/67792
    pub const fn from_u128(int_value: u128) -> Self {
        Self(int_value)
    }

    /// Returns the 128-bit unsigned integer representation.
    pub const fn to_u128(self) -> u128 {
        self.0
    }

    /// Returns the big-endian byte array representation.
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0.to_be_bytes()
    }

    /// Creates an object from field values.
    ///
    /// # Panics
    ///
    /// Panics if any argument is out of the value range of the field.
    pub const fn from_fields(
        timestamp: u64,
        counter_hi: u32,
        counter_lo: u32,
        entropy: u32,
    ) -> Self {
        if timestamp > MAX_TIMESTAMP || counter_hi > MAX_COUNTER_HI || counter_lo > MAX_COUNTER_LO {
            panic!("invalid field value");
        } else {
            Self(
                ((timestamp as u128) << 80)
                    | ((counter_hi as u128) << 56)
                    | ((counter_lo as u128) << 32)
                    | (entropy as u128),
            )
        }
    }

    /// Returns the 48-bit `timestamp` field value.
    pub const fn timestamp(&self) -> u64 {
        (self.0 >> 80) as u64
    }

    /// Returns the 24-bit `counter_hi` field value.
    pub const fn counter_hi(&self) -> u32 {
        (self.0 >> 56) as u32 & MAX_COUNTER_HI
    }

    /// Returns the 24-bit `counter_lo` field value.
    pub const fn counter_lo(&self) -> u32 {
        (self.0 >> 32) as u32 & MAX_COUNTER_LO
    }

    /// Returns the 32-bit `entropy` field value.
    pub const fn entropy(&self) -> u32 {
        self.0 as u32 & u32::MAX
    }

    /// Returns the 25-digit string representation stored in a stack-allocated string-like type
    /// that can be handled like [`String`] through common traits.
    ///
    /// This method is primarily for `no_std` environments where heap-allocated string types are
    /// not readily available.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scru128::Scru128Id;
    ///
    /// let x = "037D0XYE6OP48CMCE8EY4XLCF".parse::<Scru128Id>()?;
    /// let y = x.encode();
    /// assert_eq!(y, "037D0XYE6OP48CMCE8EY4XLCF");
    /// assert_eq!(format!("{y}"), "037D0XYE6OP48CMCE8EY4XLCF");
    /// # Ok::<(), scru128::ParseError>(())
    /// ```
    pub fn encode(&self) -> FStr<25> {
        let mut buffer = [0u8; 25];
        self.encode_inner(&mut buffer);
        unsafe { FStr::from_inner_unchecked(buffer) }
    }

    /// Writes the 25-digit string representation to `buffer` as an ASCII byte array and returns
    /// the subslice of `buffer` as a string slice.
    ///
    /// # Panics
    ///
    /// Panics if the length of `buffer` is smaller than 25.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scru128::Scru128Id;
    ///
    /// let x = "037D0XYE6OP48CMCE8EY4XLCF".parse::<Scru128Id>()?;
    ///
    /// let mut buffer = [b'\n'; 26];
    /// let subslice = x.encode_buf(&mut buffer);
    ///
    /// assert_eq!(subslice, "037D0XYE6OP48CMCE8EY4XLCF");
    /// assert_eq!(&buffer, b"037D0XYE6OP48CMCE8EY4XLCF\n");
    /// # Ok::<(), scru128::ParseError>(())
    /// ```
    #[deprecated(since = "2.3.0", note = "use `Scru128Id::encode()` instead")]
    pub fn encode_buf<'a>(&self, buffer: &'a mut [u8]) -> &'a str {
        let dst = buffer
            .get_mut(..25)
            .expect("length of `buffer` must be at least 25");
        dst.fill(0);
        self.encode_inner(dst);
        str::from_utf8(dst).unwrap()
    }

    fn encode_inner(&self, dst: &mut [u8]) {
        // implement Base36 using 56-bit words because Div<u128> is slow
        debug_assert_eq!(dst, &[0; 25]);
        let mut min_index: isize = 99; // any number greater than size of output array
        for shift in (0..128).step_by(56).rev() {
            let mut carry = (self.0 >> shift) as u64 & 0xff_ffff_ffff_ffff;

            // iterate over output array from right to left while carry != 0 but at least up to
            // place already filled
            let mut i = dst.len() as isize - 1;
            while carry > 0 || i > min_index {
                carry += (dst[i as usize] as u64) << 56;
                dst[i as usize] = (carry % 36) as u8;
                carry /= 36;
                i -= 1;
            }
            min_index = i;
        }

        dst.iter_mut().for_each(|e| *e = DIGITS[*e as usize]);
    }
}

impl str::FromStr for Scru128Id {
    type Err = ParseError;

    /// Creates an object from a 25-digit string representation.
    fn from_str(str_value: &str) -> Result<Self, Self::Err> {
        if str_value.len() != 25 {
            return Err(ParseError {
                debug_message: "invalid length",
            });
        }

        let mut int_value = 0u128;
        for b in str_value.as_bytes() {
            let n = DECODE_MAP[*b as usize];
            if n == 0xff {
                return Err(ParseError {
                    debug_message: "invalid digit",
                });
            }
            int_value = int_value
                .checked_mul(36)
                .ok_or(ParseError {
                    debug_message: "out of 128-bit value range",
                })?
                .checked_add(n as u128)
                .ok_or(ParseError {
                    debug_message: "out of 128-bit value range",
                })?;
        }
        Ok(Self(int_value))
    }
}

impl fmt::Display for Scru128Id {
    /// Returns the 25-digit canonical string representation.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.encode())
    }
}

impl From<u128> for Scru128Id {
    fn from(value: u128) -> Self {
        Self(value)
    }
}

impl From<Scru128Id> for u128 {
    fn from(object: Scru128Id) -> Self {
        object.to_u128()
    }
}

impl From<[u8; 16]> for Scru128Id {
    /// Creates an object from a 16-byte big-endian byte array.
    fn from(value: [u8; 16]) -> Self {
        Self(u128::from_be_bytes(value))
    }
}

impl From<Scru128Id> for [u8; 16] {
    /// Returns the big-endian byte array representation.
    fn from(object: Scru128Id) -> Self {
        object.to_bytes()
    }
}

/// Error parsing an invalid string representation of SCRU128 ID.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct ParseError {
    debug_message: &'static str,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid string representation")
    }
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
mod std_ext {
    use super::{ParseError, Scru128Id};
    use std::{error, str::FromStr};

    impl TryFrom<String> for Scru128Id {
        type Error = ParseError;

        fn try_from(value: String) -> Result<Self, Self::Error> {
            Self::from_str(&value)
        }
    }

    impl From<Scru128Id> for String {
        fn from(object: Scru128Id) -> Self {
            object.encode().into()
        }
    }

    impl error::Error for ParseError {}
}

#[cfg(test)]
mod tests {
    use super::Scru128Id;

    #[cfg(feature = "std")]
    use crate::Scru128Generator;

    const MAX_UINT48: u64 = (1 << 48) - 1;
    const MAX_UINT24: u32 = (1 << 24) - 1;
    const MAX_UINT32: u32 = u32::MAX;

    /// Encodes and decodes prepared cases correctly
    #[test]
    fn encodes_and_decodes_prepared_cases_correctly() {
        let cases: &[((u64, u32, u32, u32), &str)] = &[
            ((0, 0, 0, 0), "0000000000000000000000000"),
            ((MAX_UINT48, 0, 0, 0), "F5LXX1ZZ5K6TP71GEEH2DB7K0"),
            ((MAX_UINT48, 0, 0, 0), "f5lxx1zz5k6tp71geeh2db7k0"),
            ((0, MAX_UINT24, 0, 0), "0000000005GV2R2KJWR7N8XS0"),
            ((0, MAX_UINT24, 0, 0), "0000000005gv2r2kjwr7n8xs0"),
            ((0, 0, MAX_UINT24, 0), "00000000000000JPIA7QL4HS0"),
            ((0, 0, MAX_UINT24, 0), "00000000000000jpia7ql4hs0"),
            ((0, 0, 0, MAX_UINT32), "0000000000000000001Z141Z3"),
            ((0, 0, 0, MAX_UINT32), "0000000000000000001z141z3"),
            (
                (MAX_UINT48, MAX_UINT24, MAX_UINT24, MAX_UINT32),
                "F5LXX1ZZ5PNORYNQGLHZMSP33",
            ),
            (
                (MAX_UINT48, MAX_UINT24, MAX_UINT24, MAX_UINT32),
                "f5lxx1zz5pnorynqglhzmsp33",
            ),
        ];

        for e in cases {
            let from_fields = Scru128Id::from_fields(e.0 .0, e.0 .1, e.0 .2, e.0 .3);
            let from_string = e.1.parse::<Scru128Id>().unwrap();

            assert_eq!(from_fields, from_string);
            assert_eq!(
                from_fields.to_u128(),
                u128::from_str_radix(e.1, 36).unwrap()
            );
            assert_eq!(
                from_string.to_u128(),
                u128::from_str_radix(e.1, 36).unwrap()
            );
            assert_eq!(
                from_fields.to_bytes(),
                u128::from_str_radix(e.1, 36).unwrap().to_be_bytes()
            );
            assert_eq!(
                from_string.to_bytes(),
                u128::from_str_radix(e.1, 36).unwrap().to_be_bytes()
            );
            assert_eq!(
                (
                    (
                        from_fields.timestamp(),
                        from_fields.counter_hi(),
                        from_fields.counter_lo(),
                        from_fields.entropy(),
                    ),
                    &from_fields.encode() as &str
                ),
                (e.0, e.1.to_uppercase().as_str())
            );
            assert_eq!(
                (
                    (
                        from_string.timestamp(),
                        from_string.counter_hi(),
                        from_string.counter_lo(),
                        from_string.entropy(),
                    ),
                    &from_string.encode() as &str
                ),
                (e.0, e.1.to_uppercase().as_str())
            );
            #[cfg(feature = "std")]
            assert_eq!(from_fields.to_string(), e.1.to_uppercase());
            #[cfg(feature = "std")]
            assert_eq!(from_string.to_string(), e.1.to_uppercase());
        }
    }

    /// Returns error if an invalid string representation is supplied
    #[test]
    fn returns_error_if_an_invalid_string_representation_is_supplied() {
        let cases = [
            "",
            " 036Z8PUQ4TSXSIGK6O19Y164Q",
            "036Z8PUQ54QNY1VQ3HCBRKWEB ",
            " 036Z8PUQ54QNY1VQ3HELIVWAX ",
            "+036Z8PUQ54QNY1VQ3HFCV3SS0",
            "-036Z8PUQ54QNY1VQ3HHY8U1CH",
            "+36Z8PUQ54QNY1VQ3HJQ48D9P",
            "-36Z8PUQ5A7J0TI08OZ6ZDRDY",
            "036Z8PUQ5A7J0T_08P2CDZ28V",
            "036Z8PU-5A7J0TI08P3OL8OOL",
            "036Z8PUQ5A7J0TI08P4J 6CYA",
            "F5LXX1ZZ5PNORYNQGLHZMSP34",
            "ZZZZZZZZZZZZZZZZZZZZZZZZZ",
        ];

        for e in cases {
            assert!(e.parse::<Scru128Id>().is_err());
        }
    }

    /// Has symmetric converters from/to various values
    #[test]
    fn has_symmetric_converters_from_to_various_values() {
        let cases = [
            Scru128Id::from_fields(0, 0, 0, 0),
            Scru128Id::from_fields(MAX_UINT48, 0, 0, 0),
            Scru128Id::from_fields(0, MAX_UINT24, 0, 0),
            Scru128Id::from_fields(0, 0, MAX_UINT24, 0),
            Scru128Id::from_fields(0, 0, 0, MAX_UINT32),
            Scru128Id::from_fields(MAX_UINT48, MAX_UINT24, MAX_UINT24, MAX_UINT32),
        ];

        #[cfg(feature = "std")]
        let cases = {
            let mut v = cases.to_vec();
            let mut g = Scru128Generator::new();
            for _ in 0..1000 {
                v.push(g.generate());
            }
            v
        };

        for e in cases {
            assert_eq!(e.encode().parse::<Scru128Id>(), Ok(e));
            #[cfg(feature = "std")]
            assert_eq!(e.to_string().parse::<Scru128Id>(), Ok(e));
            #[cfg(feature = "std")]
            assert_eq!(Scru128Id::try_from(String::from(e)), Ok(e));
            assert_eq!(Scru128Id::from_u128(e.to_u128()), e);
            assert_eq!(Scru128Id::from(u128::from(e)), e);
            assert_eq!(Scru128Id::from(e.to_bytes()), e);
            assert_eq!(Scru128Id::from(<[u8; 16]>::from(e)), e);
            assert_eq!(
                Scru128Id::from_fields(e.timestamp(), e.counter_hi(), e.counter_lo(), e.entropy()),
                e
            );
        }
    }

    /// Supports comparison operators
    #[test]
    fn supports_comparison_operators() {
        #[cfg(feature = "std")]
        fn hash(v: impl std::hash::Hash) -> u64 {
            use std::{collections::hash_map, hash::Hasher};
            let mut hasher = hash_map::DefaultHasher::new();
            v.hash(&mut hasher);
            hasher.finish()
        }

        let ordered = [
            Scru128Id::from_fields(0, 0, 0, 0),
            Scru128Id::from_fields(0, 0, 0, 1),
            Scru128Id::from_fields(0, 0, 0, MAX_UINT32),
            Scru128Id::from_fields(0, 0, 1, 0),
            Scru128Id::from_fields(0, 0, MAX_UINT24, 0),
            Scru128Id::from_fields(0, 1, 0, 0),
            Scru128Id::from_fields(0, MAX_UINT24, 0, 0),
            Scru128Id::from_fields(1, 0, 0, 0),
            Scru128Id::from_fields(2, 0, 0, 0),
        ];

        #[cfg(feature = "std")]
        let ordered = {
            let mut v = ordered.to_vec();
            let mut g = Scru128Generator::new();
            for _ in 0..1000 {
                v.push(g.generate());
            }
            v
        };

        let mut prev = &ordered[0];
        for curr in &ordered[1..] {
            assert_ne!(curr, prev);
            assert_ne!(prev, curr);
            #[cfg(feature = "std")]
            assert_ne!(hash(curr), hash(prev));
            assert!(curr > prev);
            assert!(curr >= prev);
            assert!(prev < curr);
            assert!(prev <= curr);

            let clone = &curr.clone();
            assert_eq!(curr, clone);
            assert_eq!(clone, curr);
            #[cfg(feature = "std")]
            assert_eq!(hash(curr), hash(clone));
            assert!(curr >= clone);
            assert!(clone >= curr);
            assert!(curr <= clone);
            assert!(clone <= curr);

            prev = curr;
        }
    }
}

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
mod serde_support {
    use super::{fmt, str, Scru128Id};
    use serde::{de, Deserializer, Serializer};

    impl serde::Serialize for Scru128Id {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            if serializer.is_human_readable() {
                serializer.serialize_str(&self.encode())
            } else {
                serializer.serialize_bytes(&self.to_bytes())
            }
        }
    }

    impl<'de> serde::Deserialize<'de> for Scru128Id {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            if deserializer.is_human_readable() {
                deserializer.deserialize_str(VisitorImpl)
            } else {
                deserializer.deserialize_bytes(VisitorImpl)
            }
        }
    }

    struct VisitorImpl;

    impl<'de> de::Visitor<'de> for VisitorImpl {
        type Value = Scru128Id;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "a SCRU128 ID representation")
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
            value.parse::<Self::Value>().map_err(de::Error::custom)
        }

        fn visit_bytes<E: de::Error>(self, value: &[u8]) -> Result<Self::Value, E> {
            match <[u8; 16]>::try_from(value) {
                Ok(array_value) => Ok(Self::Value::from(array_value)),
                Err(err) => match str::from_utf8(value) {
                    Ok(str_value) => self.visit_str(str_value),
                    _ => Err(de::Error::custom(err)),
                },
            }
        }

        fn visit_u128<E: de::Error>(self, value: u128) -> Result<Self::Value, E> {
            Ok(Self::Value::from(value))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::Scru128Id;
        use serde_test::{Configure, Token};

        /// Serializes and deserializes prepared cases correctly
        #[test]
        fn serializes_and_deserializes_prepared_cases_correctly() {
            let cases = [
                (
                    "037ARKZBGN93KDU9H3PW2OW2L",
                    &[
                        1, 128, 178, 254, 34, 56, 72, 100, 6, 87, 159, 252, 102, 145, 202, 93,
                    ],
                ),
                (
                    "037ARKZBH94JVGJMM6JTWGZTQ",
                    &[
                        1, 128, 178, 254, 34, 60, 72, 100, 6, 194, 191, 219, 2, 6, 125, 94,
                    ],
                ),
                (
                    "037ARKZBHELEY7UNPVCJF5K4Z",
                    &[
                        1, 128, 178, 254, 34, 61, 72, 100, 6, 48, 162, 140, 185, 18, 16, 51,
                    ],
                ),
                (
                    "037ARKZBHELEY7UNPVEL8ZYP1",
                    &[
                        1, 128, 178, 254, 34, 61, 72, 100, 6, 48, 162, 141, 195, 39, 182, 101,
                    ],
                ),
                (
                    "037ARKZBHELEY7UNPVGEFDINQ",
                    &[
                        1, 128, 178, 254, 34, 61, 72, 100, 6, 48, 162, 142, 174, 14, 198, 182,
                    ],
                ),
                (
                    "037ARKZBHELEY7UNPVHSYWHO2",
                    &[
                        1, 128, 178, 254, 34, 61, 72, 100, 6, 48, 162, 143, 100, 55, 67, 114,
                    ],
                ),
                (
                    "037ARKZBHELEY7UNPVJLR4OT7",
                    &[
                        1, 128, 178, 254, 34, 61, 72, 100, 6, 48, 162, 144, 77, 179, 181, 155,
                    ],
                ),
                (
                    "037ARKZBHELEY7UNPVMFM8457",
                    &[
                        1, 128, 178, 254, 34, 61, 72, 100, 6, 48, 162, 145, 188, 211, 88, 251,
                    ],
                ),
            ];

            for (text, bytes) in cases {
                let e = text.parse::<Scru128Id>().unwrap();
                serde_test::assert_tokens(&e.readable(), &[Token::Str(text)]);
                serde_test::assert_tokens(&e.compact(), &[Token::Bytes(bytes)]);

                // deserialize the other format regardless of human-readability configuration
                serde_test::assert_de_tokens(&e.readable(), &[Token::Bytes(bytes)]);
                serde_test::assert_de_tokens(&e.compact(), &[Token::Str(text)]);

                // deserialize textual representation even if passed as byte slice
                serde_test::assert_de_tokens(&e.readable(), &[Token::Bytes(text.as_bytes())]);
                serde_test::assert_de_tokens(&e.compact(), &[Token::Bytes(text.as_bytes())]);
            }
        }
    }
}
