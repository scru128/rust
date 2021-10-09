use std::error;
use std::fmt;
use std::str::FromStr;

/// Maximum value of 28-bit counter field.
pub const MAX_COUNTER: u32 = 0xFFF_FFFF;

/// Maximum value of 24-bit per_sec_random field.
pub const MAX_PER_SEC_RANDOM: u32 = 0xFF_FFFF;

/// Digit characters used in the base 32 notation.
const CHARSET: [char; 32] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I',
    'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V',
];

/// Represents a SCRU128 ID.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Identifier(u128);

impl Identifier {
    /// Creates an object from a 128-bit unsigned integer.
    pub const fn from_u128(int_value: u128) -> Self {
        Self(int_value)
    }

    /// Returns the 128-bit unsigned integer representation.
    pub const fn as_u128(&self) -> u128 {
        self.0
    }

    /// Creates an object from field values.
    pub fn from_field_values(
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

    /// Returns 44-bit millisecond timestamp field.
    pub fn timestamp(&self) -> u64 {
        (self.0 >> 84) as u64
    }

    /// Returns 28-bit per-millisecond counter field.
    pub fn counter(&self) -> u32 {
        (self.0 >> 56) as u32 & MAX_COUNTER
    }

    /// Returns 24-bit per-second randomness field.
    pub fn per_sec_random(&self) -> u32 {
        (self.0 >> 32) as u32 & MAX_PER_SEC_RANDOM
    }

    /// Returns 32-bit per-generation randomness field.
    pub fn per_gen_random(&self) -> u32 {
        self.0 as u32 & u32::MAX
    }
}

impl FromStr for Identifier {
    type Err = Error;

    /// Creates an object from a 26-digit string representation.
    fn from_str(str_value: &str) -> Result<Self, Self::Err> {
        let mut cs = str_value.chars();
        if str_value.chars().count() == 26
            && cs.next().map_or(false, |c| c.is_digit(8))
            && cs.all(|c| c.is_digit(32))
        {
            Ok(Self(u128::from_str_radix(str_value, 32).unwrap()))
        } else {
            Err(Error::InvalidStringRepresentation(str_value.into()))
        }
    }
}

impl fmt::Display for Identifier {
    /// Returns the 26-digit canonical string representation.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buffer = ['0'; 26];
        let mut n = self.0;
        for i in 0..26 {
            buffer[25 - i] = CHARSET[(n & 31) as usize];
            n >>= 5;
        }
        f.write_str(&(buffer.iter().collect::<String>()))
    }
}

impl From<u128> for Identifier {
    fn from(int_value: u128) -> Self {
        Self::from_u128(int_value)
    }
}

impl From<Identifier> for u128 {
    fn from(object: Identifier) -> Self {
        object.as_u128()
    }
}

#[non_exhaustive]
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum Error {
    InvalidStringRepresentation(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Error::InvalidStringRepresentation(str_value) => {
                write!(f, "invalid string representation: {}", str_value)
            }
        }
    }
}

impl error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::Identifier;
    use crate::scru128;

    /// Encodes and decodes prepared cases correctly
    #[test]
    fn it_encodes_and_decodes_prepared_cases_correctly() {
        let cases: Vec<((u64, u32, u32, u32), &str)> = vec![
            ((0, 0, 0, 0), "00000000000000000000000000"),
            ((2u64.pow(44) - 1, 0, 0, 0), "7VVVVVVVVG0000000000000000"),
            ((0, 2u32.pow(28) - 1, 0, 0), "000000000FVVVVU00000000000"),
            ((0, 0, 2u32.pow(24) - 1, 0), "000000000000001VVVVS000000"),
            ((0, 0, 0, u32::MAX), "00000000000000000003VVVVVV"),
            (
                (
                    2u64.pow(44) - 1,
                    2u32.pow(28) - 1,
                    2u32.pow(24) - 1,
                    u32::MAX,
                ),
                "7VVVVVVVVVVVVVVVVVVVVVVVVV",
            ),
        ];

        for e in cases {
            let from_fields = Identifier::from_field_values(e.0 .0, e.0 .1, e.0 .2, e.0 .3);
            let from_str = e.1.parse::<Identifier>().unwrap();

            assert_eq!(from_fields, from_str);
            assert_eq!(
                from_fields.as_u128(),
                u128::from_str_radix(e.1, 32).unwrap()
            );
            assert_eq!(from_str.as_u128(), u128::from_str_radix(e.1, 32).unwrap());
            assert_eq!(
                (
                    (
                        from_fields.timestamp(),
                        from_fields.counter(),
                        from_fields.per_sec_random(),
                        from_fields.per_gen_random(),
                    ),
                    from_fields.to_string().as_str(),
                ),
                e,
            );
            assert_eq!(
                (
                    (
                        from_str.timestamp(),
                        from_str.counter(),
                        from_str.per_sec_random(),
                        from_str.per_gen_random(),
                    ),
                    from_str.to_string().as_str(),
                ),
                e,
            );
        }
    }

    /// Has symmetric from_str() and to_string()
    #[test]
    fn it_has_symmetric_from_str_and_to_string() {
        for _ in 0..1000 {
            let src = scru128();
            assert_eq!(src.parse::<Identifier>().unwrap().to_string(), src);
        }
    }
}
