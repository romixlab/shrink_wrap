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
            return Some(Repr::UNib32);
        }
        if s == "nib" || s == "Nibble" {
            return Some(Repr::Nibble);
        }
        if let Some(s) = s.strip_prefix("ub") {
            return Some(Repr::U(Self::parse_bits(s)?));
        }
        let bits = Self::parse_bits(s.strip_prefix("u")?)?;
        match bits {
            8 => Some(Repr::U8),
            16 => Some(Repr::U16),
            32 => Some(Repr::U32),
            other => Some(Repr::U(other)),
        }
    }

    /// A bit count of a discriminant: 1 to 32, what the reader and writer take in one call.
    fn parse_bits(s: &str) -> Option<u8> {
        let bits: u8 = s.parse().ok()?;
        (1..=32).contains(&bits).then_some(bits)
    }

    pub(crate) fn max_discriminant(&self) -> u32 {
        match self {
            Repr::U(bits) => {
                if *bits == 32 {
                    u32::MAX
                } else {
                    2u32.pow(*bits as u32) - 1
                }
            }
            Repr::UNib32 => u32::MAX,
            Repr::Nibble => 15,
            Repr::U8 => 255,
            Repr::U16 => 65_535,
            Repr::U32 => u32::MAX,
        }
    }

    pub(crate) fn required_bits(&self) -> u8 {
        match self {
            Repr::U(bits) => *bits,
            Repr::UNib32 => 32,
            Repr::Nibble => 4,
            Repr::U8 => 8,
            Repr::U16 => 16,
            Repr::U32 => 32,
        }
    }

    pub(crate) fn std_bits(&self) -> u8 {
        match &self {
            Repr::Nibble => 8,
            Repr::U8 => 8,
            Repr::U16 => 16,
            Repr::U32 => 32,
            Repr::UNib32 => 32,
            Repr::U(bits) if *bits < 8 => 8,
            Repr::U(bits) if *bits < 16 => 16,
            Repr::U(bits) if *bits < 32 => 32,
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
