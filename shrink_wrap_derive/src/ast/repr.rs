#[derive(Copy, Clone, Debug, Default)]
pub(crate) enum Repr {
    /// Variable length, nibble based and nibble aligned
    #[default]
    UNib32,
    /// 4-bit aligned
    Nibble,
    /// 1-bit aligned
    U(u8),
    /// Byte-aligned
    U8,
    /// Byte-aligned
    U16,
    /// Byte-aligned
    U32,
}

impl Repr {
    pub(crate) fn parse_str(s: &str) -> Option<Self> {
        if s == "unib32" || s == "UNib32" {
            return Some(Self::UNib32);
        }
        if s == "nib" || s == "Nibble" {
            return Some(Self::Nibble);
        }
        if let Some(s) = s.strip_prefix("ub") {
            return Some(Self::U(Self::parse_bits(s)?));
        }
        let bits = Self::parse_bits(s.strip_prefix("u")?)?;
        match bits {
            8 => Some(Self::U8),
            16 => Some(Self::U16),
            32 => Some(Self::U32),
            other => Some(Self::U(other)),
        }
    }

    /// A bit count of a discriminant: 1 to 32, what the reader and writer take in one call.
    fn parse_bits(s: &str) -> Option<u8> {
        let bits: u8 = s.parse().ok()?;
        (1..=32).contains(&bits).then_some(bits)
    }

    pub(crate) fn max_discriminant(&self) -> u32 {
        match self {
            Self::U(bits) => {
                if *bits == 32 {
                    u32::MAX
                } else {
                    2u32.pow(*bits as u32) - 1
                }
            }
            Self::UNib32 => u32::MAX,
            Self::Nibble => 15,
            Self::U8 => 255,
            Self::U16 => 65_535,
            Self::U32 => u32::MAX,
        }
    }

    pub(crate) fn required_bits(&self) -> u8 {
        match self {
            Self::U(bits) => *bits,
            Self::UNib32 => 32,
            Self::Nibble => 4,
            Self::U8 => 8,
            Self::U16 => 16,
            Self::U32 => 32,
        }
    }

    pub(crate) fn std_bits(&self) -> u8 {
        match &self {
            Self::Nibble => 8,
            Self::U8 => 8,
            Self::U16 => 16,
            Self::U32 => 32,
            Self::UNib32 => 32,
            Self::U(bits) if *bits < 8 => 8,
            Self::U(bits) if *bits < 16 => 16,
            Self::U(bits) if *bits < 32 => 32,
            _ => 32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Repr;

    #[test]
    fn bit_counts_are_1_to_32() {
        assert!(matches!(Repr::parse_str("ub1"), Some(Repr::U(1))));
        assert!(matches!(Repr::parse_str("ub32"), Some(Repr::U(32))));
        assert!(matches!(Repr::parse_str("u31"), Some(Repr::U(31))));
        assert!(matches!(Repr::parse_str("u32"), Some(Repr::U32)));
        for refused in ["u0", "ub0", "u33", "ub33", "u64", "ub255", "u256"] {
            assert!(Repr::parse_str(refused).is_none(), "{refused}");
        }
    }
}
