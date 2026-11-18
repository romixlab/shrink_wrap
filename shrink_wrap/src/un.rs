use crate::{
    BufReader, BufWriter, DeserializeShrinkWrap, DeserializeShrinkWrapOwned, ElementSize, Error,
    SerializeShrinkWrap,
};
use pastey::paste;

macro_rules! un {
    ($bits:literal, $base_bits:literal) => {
        paste! {
            #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
            #[cfg_attr(feature = "defmt", derive(defmt::Format))]
            #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
            #[cfg_attr(feature = "serde", serde(transparent))]
            #[doc = $bits "-bit unsigned number, backed by u" $base_bits ", serialized as " $bits " bits with alignment of 1 bit."]
            pub struct [<U $bits>]([<u $base_bits>]);
            impl [<U $bits>] {
                #[inline(always)]
                pub const fn new(x: [<u $base_bits>]) -> Option<Self> {
                    if x <= Self::max().0 {
                        Some(Self(x))
                    } else {
                        None
                    }
                }

                #[inline(always)]
                pub const fn zero() -> Self {
                    Self(0)
                }

                #[inline(always)]
                pub const fn one() -> Self {
                    Self(1)
                }

                #[inline(always)]
                pub const fn max() -> Self {
                    // avoid overflow (2^(n-1) - 1) + 2^(n-1)
                    Self(([<2 _ u $base_bits>].pow($bits - 1) - 1) + [<2 _ u $base_bits>].pow($bits - 1))
                }

                #[inline(always)]
                pub const fn value(&self) -> [<u $base_bits>] {
                    self.0
                }

                /// The low bits as on the wire, for the fixed-layout fast path (SW-45).
                #[doc(hidden)]
                #[inline(always)]
                pub const fn __to_wire(self) -> u64 {
                    (self.0 as u64) & (u64::MAX >> (64 - $bits))
                }

                /// From the bits on the wire, for the fixed-layout fast path (SW-45).
                #[doc(hidden)]
                #[inline(always)]
                pub const fn __from_wire(raw: u64) -> Self {
                    Self((raw & (u64::MAX >> (64 - $bits))) as [<u $base_bits>])
                }
            }

            impl SerializeShrinkWrap for [<U $bits>] {
                const ELEMENT_SIZE: ElementSize = ElementSize::Sized { size_bits: $bits };
                const MAX_SIZE: Option<crate::max_size::MaxSize> = Some(crate::max_size::MaxSize::bits($bits));

                fn ser_shrink_wrap(&self, wr: &mut BufWriter) -> Result<(), Error> {
                    wr.[<write_un $base_bits>]($bits, self.0)
                }
            }

            #[cfg(feature = "std")]
            impl crate::SerializeShrinkWrapOwned for [<U $bits>] {
                const ELEMENT_SIZE: ElementSize = ElementSize::Sized { size_bits: $bits };

                fn ser_shrink_wrap_owned(&self, wr: &mut crate::BufWriterOwned) -> Result<(), Error> {
                    wr.[<write_un $base_bits>]($bits, self.0)
                }
            }

            impl<'i> DeserializeShrinkWrap<'i> for [<U $bits>] {
                const ELEMENT_SIZE: ElementSize = ElementSize::Sized { size_bits: $bits };

                fn des_shrink_wrap<'di>(rd: &'di mut BufReader<'i>) -> Result<Self, Error> {
                    Ok([<U $bits>](rd.[<read_un $base_bits>]($bits)?))
                }

                #[inline(always)]
                fn des_shrink_wrap_latch(rd: &mut BufReader<'i>) -> Result<Self, Error> {
                    Ok([<U $bits>](rd.[<read_un $base_bits _latch>]($bits)))
                }
            }

            impl DeserializeShrinkWrapOwned for [<U $bits>] {
                const ELEMENT_SIZE: ElementSize = ElementSize::Sized { size_bits: $bits };

                fn des_shrink_wrap_owned(rd: &mut BufReader<'_>) -> Result<Self, Error> {
                    Ok([<U $bits>](rd.[<read_un $base_bits>]($bits)?))
                }

                #[inline(always)]
                fn des_shrink_wrap_owned_latch(rd: &mut BufReader<'_>) -> Result<Self, Error> {
                    Ok([<U $bits>](rd.[<read_un $base_bits _latch>]($bits)))
                }
            }

            impl From<[<U $bits>]> for [<u $base_bits>] {
                #[inline(always)]
                fn from(x: [<U $bits>]) -> Self {
                    x.0
                }
            }

            #[doc = "Fails with [Error::SubtypeOutOfRange] when the number does not fit into " $bits " bits."]
            impl TryFrom<[<u $base_bits>]> for [<U $bits>] {
                type Error = Error;

                #[inline(always)]
                fn try_from(x: [<u $base_bits>]) -> Result<Self, Error> {
                    Self::new(x).ok_or(Error::SubtypeOutOfRange)
                }
            }
        }
    };
}

macro_rules! unx {
    ($($bits:literal),* / $base_bits:literal) => {
        $(un!($bits, $base_bits);)*
    };
}

unx!(1, 2, 3, 4, 5, 6, 7, 8 / 8);
unx!(9, 10, 11, 12, 13, 14, 15, 16 / 16);
unx!(17, 18, 19, 20, 21, 22, 23, 24 / 32);
unx!(25, 26, 27, 28, 29, 30, 31, 32 / 32);
unx!(33, 34, 35, 36, 37, 38, 39, 40 / 64);
unx!(41, 42, 43, 44, 45, 46, 47, 48 / 64);
unx!(49, 50, 51, 52, 53, 54, 55, 56 / 64);
unx!(57, 58, 59, 60, 61, 62, 63, 64 / 64);

macro_rules! signed_un {
    ($bits:literal, $base_bits:literal) => {
        paste! {
            #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
            #[cfg_attr(feature = "defmt", derive(defmt::Format))]
            #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
            #[cfg_attr(feature = "serde", serde(transparent))]
            #[doc = $bits "-bit signed number, backed by i" $base_bits ", serialized as " $bits " bits with alignment of 1 bit."]
            pub struct [<I $bits>]([<i $base_bits>]);
            impl [<I $bits>] {
                #[inline(always)]
                pub const fn new(x: [<i $base_bits>]) -> Option<Self> {
                    if x >= Self::min().0 && x <= Self::max().0 {
                        Some(Self(x))
                    } else {
                        None
                    }
                }

                #[inline(always)]
                pub const fn min() -> Self {
                    // avoid overflow -2^(n-2) - 2^(n-2)
                    Self(-[<2 _ i $base_bits>].pow($bits - 2) - [<2 _ i $base_bits>].pow($bits - 2))
                }

                // the low `$bits` bits of `val` as read from the wire, sign extended
                #[inline(always)]
                fn sign_extend(val: [<u $base_bits>]) -> Self {
                    let is_negative = val & (1 << ($bits - 1)) != 0;
                    if is_negative {
                        const SIGN_EXTEND_M1: [<u $base_bits>] = [<u $base_bits>]::MAX << ($bits - 1);
                        const SIGN_EXTEND: [<u $base_bits>] = SIGN_EXTEND_M1 << 1;
                        Self((val | SIGN_EXTEND) as [<i $base_bits>])
                    } else {
                        Self(val as [<i $base_bits>])
                    }
                }

                #[inline(always)]
                pub const fn minus_one() -> Self {
                    Self(-1)
                }

                #[inline(always)]
                pub const fn zero() -> Self {
                    Self(0)
                }

                #[inline(always)]
                pub const fn one() -> Self {
                    Self(1)
                }

                #[inline(always)]
                pub const fn max() -> Self {
                    // avoid overflow (2^(n-2) - 1) + 2^(n-2)
                    Self(([<2 _ i $base_bits>].pow($bits - 2) - 1) + [<2 _ i $base_bits>].pow($bits - 2))
                }

                #[inline(always)]
                pub const fn value(&self) -> [<i $base_bits>] {
                    self.0
                }

                /// The low bits as on the wire (two's complement), for the fixed-layout fast path (SW-45).
                #[doc(hidden)]
                #[inline(always)]
                pub const fn __to_wire(self) -> u64 {
                    (self.0 as [<u $base_bits>] as u64) & (u64::MAX >> (64 - $bits))
                }

                /// From the bits on the wire, sign extended, for the fixed-layout fast path (SW-45).
                #[doc(hidden)]
                #[inline(always)]
                pub fn __from_wire(raw: u64) -> Self {
                    Self::sign_extend((raw & (u64::MAX >> (64 - $bits))) as [<u $base_bits>])
                }
            }

            impl SerializeShrinkWrap for [<I $bits>] {
                const ELEMENT_SIZE: ElementSize = ElementSize::Sized { size_bits: $bits };
                const MAX_SIZE: Option<crate::max_size::MaxSize> = Some(crate::max_size::MaxSize::bits($bits));

                fn ser_shrink_wrap(&self, wr: &mut BufWriter) -> Result<(), Error> {
                    wr.[<write_un $base_bits>]($bits, self.0 as [<u $base_bits>])
                }
            }

            #[cfg(feature = "std")]
            impl crate::SerializeShrinkWrapOwned for [<I $bits>] {
                const ELEMENT_SIZE: ElementSize = ElementSize::Sized { size_bits: $bits };

                fn ser_shrink_wrap_owned(&self, wr: &mut crate::BufWriterOwned) -> Result<(), Error> {
                    wr.[<write_un $base_bits>]($bits, self.0 as [<u $base_bits>])
                }
            }

            impl<'i> DeserializeShrinkWrap<'i> for [<I $bits>] {
                const ELEMENT_SIZE: ElementSize = ElementSize::Sized { size_bits: $bits };

                fn des_shrink_wrap<'di>(rd: &'di mut BufReader<'i>) -> Result<Self, Error> {
                    Ok(Self::sign_extend(rd.[<read_un $base_bits>]($bits)?))
                }

                #[inline(always)]
                fn des_shrink_wrap_latch(rd: &mut BufReader<'i>) -> Result<Self, Error> {
                    Ok(Self::sign_extend(rd.[<read_un $base_bits _latch>]($bits)))
                }
            }

            impl DeserializeShrinkWrapOwned for [<I $bits>] {
                const ELEMENT_SIZE: ElementSize = ElementSize::Sized { size_bits: $bits };

                fn des_shrink_wrap_owned(rd: &mut BufReader<'_>) -> Result<Self, Error> {
                    Ok(Self::sign_extend(rd.[<read_un $base_bits>]($bits)?))
                }

                #[inline(always)]
                fn des_shrink_wrap_owned_latch(rd: &mut BufReader<'_>) -> Result<Self, Error> {
                    Ok(Self::sign_extend(rd.[<read_un $base_bits _latch>]($bits)))
                }
            }

            impl From<[<I $bits>]> for [<i $base_bits>] {
                #[inline(always)]
                fn from(x: [<I $bits>]) -> Self {
                    x.0
                }
            }

            #[doc = "Fails with [Error::SubtypeOutOfRange] when the number does not fit into " $bits " bits."]
            impl TryFrom<[<i $base_bits>]> for [<I $bits>] {
                type Error = Error;

                #[inline(always)]
                fn try_from(x: [<i $base_bits>]) -> Result<Self, Error> {
                    Self::new(x).ok_or(Error::SubtypeOutOfRange)
                }
            }
        }
    };
}

macro_rules! inx {
    ($($bits:literal),* / $base_bits:literal) => {
        $(signed_un!($bits, $base_bits);)*
    };
}

inx!(2, 3, 4, 5, 6, 7, 8 / 8);
inx!(9, 10, 11, 12, 13, 14, 15, 16 / 16);
inx!(17, 18, 19, 20, 21, 22, 23, 24 / 32);
inx!(25, 26, 27, 28, 29, 30, 31, 32 / 32);
inx!(33, 34, 35, 36, 37, 38, 39, 40 / 64);
inx!(41, 42, 43, 44, 45, 46, 47, 48 / 64);
inx!(49, 50, 51, 52, 53, 54, 55, 56 / 64);
inx!(57, 58, 59, 60, 61, 62, 63, 64 / 64);

/// N-bit unsigned number, unlike Ux and Ix which are const size, here N is serialized as well and not known beforehand.
/// Serialized as `3 + 3(N<8) or 4(N<16) or 5(N<32) or 6(N<64) + N bits`.
/// Serialized layout: discriminant(U3), bit_count(U3, U4, U5 or U6), value (N-bits). N is `bit_count`; a value
/// that needs more bits fails to serialize with `SubtypeOutOfRange`.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum UN {
    UN8 { bit_count: U3, value: u8 },
    UN16 { bit_count: U4, value: u16 },
    UN32 { bit_count: U5, value: u32 },
    UN64 { bit_count: U6, value: u64 },
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
/// N-bit signed number, the layout of [UN] with the value in two's complement of N bits.
pub enum IN {
    IN8 { bit_count: U3, value: i8 },
    IN16 { bit_count: U4, value: i16 },
    IN32 { bit_count: U5, value: i32 },
    IN64 { bit_count: U6, value: i64 },
}

/// Bits of the variant number of `UN` and `IN` on the wire.
const UN_DISCRIMINANT_BITS: u8 = 3;

impl UN {
    /// Variant number, bit count and value; `SubtypeOutOfRange` when the value needs more bits than `bit_count`.
    fn to_wire_parts(self) -> Result<(u8, u8, u64), Error> {
        let (discriminant, bit_count, value) = match self {
            UN::UN8 { bit_count, value } => (0, bit_count.value(), u64::from(value)),
            UN::UN16 { bit_count, value } => (1, bit_count.value(), u64::from(value)),
            UN::UN32 { bit_count, value } => (2, bit_count.value(), u64::from(value)),
            UN::UN64 { bit_count, value } => (3, bit_count.value(), value),
        };
        if value >> bit_count != 0 {
            return Err(Error::SubtypeOutOfRange);
        }
        Ok((discriminant, bit_count, value))
    }

    /// The value of variant `discriminant` from the `bit_count` low bits of `raw`.
    fn from_wire_parts(discriminant: u8, bit_count: u8, raw: u64) -> Result<Self, Error> {
        Ok(match discriminant {
            0 => UN::UN8 {
                bit_count: U3::new(bit_count).ok_or(Error::InvalidBitCount)?,
                value: raw as u8,
            },
            1 => UN::UN16 {
                bit_count: U4::new(bit_count).ok_or(Error::InvalidBitCount)?,
                value: raw as u16,
            },
            2 => UN::UN32 {
                bit_count: U5::new(bit_count).ok_or(Error::InvalidBitCount)?,
                value: raw as u32,
            },
            3 => UN::UN64 {
                bit_count: U6::new(bit_count).ok_or(Error::InvalidBitCount)?,
                value: raw,
            },
            _ => return Err(Error::EnumFutureVersionOrMalformedData),
        })
    }
}

/// `value` in two's complement of `bit_count` bits (below 64); `SubtypeOutOfRange` when it does not fit.
fn signed_to_bits(value: i64, bit_count: u8) -> Result<u64, Error> {
    if sign_extend(value as u64, bit_count) != value {
        return Err(Error::SubtypeOutOfRange);
    }
    Ok(value as u64 & !(u64::MAX << bit_count))
}

/// The two's complement number in the `bit_count` (below 64) low bits of `raw`; 0 for no bits.
fn sign_extend(raw: u64, bit_count: u8) -> i64 {
    if bit_count == 0 {
        return 0;
    }
    let shift = 64 - u32::from(bit_count);
    (raw << shift) as i64 >> shift
}

impl IN {
    /// Variant number, bit count and the value's two's complement bits; `SubtypeOutOfRange` when the value needs
    /// more bits than `bit_count`.
    fn to_wire_parts(self) -> Result<(u8, u8, u64), Error> {
        let (discriminant, bit_count, value) = match self {
            IN::IN8 { bit_count, value } => (0, bit_count.value(), i64::from(value)),
            IN::IN16 { bit_count, value } => (1, bit_count.value(), i64::from(value)),
            IN::IN32 { bit_count, value } => (2, bit_count.value(), i64::from(value)),
            IN::IN64 { bit_count, value } => (3, bit_count.value(), value),
        };
        Ok((discriminant, bit_count, signed_to_bits(value, bit_count)?))
    }

    /// The value of variant `discriminant` from the `bit_count` low bits of `raw`, sign-extended.
    fn from_wire_parts(discriminant: u8, bit_count: u8, raw: u64) -> Result<Self, Error> {
        let value = sign_extend(raw, bit_count);
        Ok(match discriminant {
            0 => IN::IN8 {
                bit_count: U3::new(bit_count).ok_or(Error::InvalidBitCount)?,
                value: value as i8,
            },
            1 => IN::IN16 {
                bit_count: U4::new(bit_count).ok_or(Error::InvalidBitCount)?,
                value: value as i16,
            },
            2 => IN::IN32 {
                bit_count: U5::new(bit_count).ok_or(Error::InvalidBitCount)?,
                value: value as i32,
            },
            3 => IN::IN64 {
                bit_count: U6::new(bit_count).ok_or(Error::InvalidBitCount)?,
                value,
            },
            _ => return Err(Error::EnumFutureVersionOrMalformedData),
        })
    }
}

/// The plain (non-`Series`) form of `UN` and `IN`: the variant number in 3 bits, the bit count in 3, 4, 5 or 6
/// bits by variant, then that many value bits.
macro_rules! bit_count_number {
    ($ty:ident) => {
        impl SerializeShrinkWrap for $ty {
            const ELEMENT_SIZE: ElementSize = ElementSize::SelfDescribing;

            fn ser_shrink_wrap(&self, wr: &mut BufWriter) -> Result<(), Error> {
                let (discriminant, bit_count, value) = self.to_wire_parts()?;
                wr.write_un8(UN_DISCRIMINANT_BITS, discriminant)?;
                wr.write_un8(UN_DISCRIMINANT_BITS + discriminant, bit_count)?;
                wr.write_un64(bit_count, value)
            }
        }

        #[cfg(feature = "std")]
        impl crate::SerializeShrinkWrapOwned for $ty {
            const ELEMENT_SIZE: ElementSize = ElementSize::SelfDescribing;

            fn ser_shrink_wrap_owned(&self, wr: &mut crate::BufWriterOwned) -> Result<(), Error> {
                let (discriminant, bit_count, value) = self.to_wire_parts()?;
                wr.write_un8(UN_DISCRIMINANT_BITS, discriminant)?;
                wr.write_un8(UN_DISCRIMINANT_BITS + discriminant, bit_count)?;
                wr.write_un64(bit_count, value)
            }
        }

        impl<'i> DeserializeShrinkWrap<'i> for $ty {
            const ELEMENT_SIZE: ElementSize = <$ty as SerializeShrinkWrap>::ELEMENT_SIZE;

            fn des_shrink_wrap<'di>(rd: &'di mut BufReader<'i>) -> Result<Self, Error> {
                let discriminant = rd.read_un8(UN_DISCRIMINANT_BITS)?;
                if discriminant > 3 {
                    return Err(Error::EnumFutureVersionOrMalformedData);
                }
                let bit_count = rd.read_un8(UN_DISCRIMINANT_BITS + discriminant)?;
                let raw = rd.read_un64(bit_count)?;
                $ty::from_wire_parts(discriminant, bit_count, raw)
            }
        }

        impl DeserializeShrinkWrapOwned for $ty {
            const ELEMENT_SIZE: ElementSize = <$ty as SerializeShrinkWrap>::ELEMENT_SIZE;

            fn des_shrink_wrap_owned(rd: &mut BufReader<'_>) -> Result<Self, Error> {
                $ty::des_shrink_wrap(rd)
            }
        }
    };
}

bit_count_number!(UN);
bit_count_number!(IN);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanity_check() {
        assert_eq!(U7::max().0, 127);
        assert_eq!(I7::max().0, 63);
        assert_eq!(I7::min().0, -64);
    }

    fn un(variant: u8, bit_count: u8, value: u64) -> UN {
        match variant {
            0 => UN::UN8 {
                bit_count: U3::new(bit_count).unwrap(),
                value: value as u8,
            },
            1 => UN::UN16 {
                bit_count: U4::new(bit_count).unwrap(),
                value: value as u16,
            },
            2 => UN::UN32 {
                bit_count: U5::new(bit_count).unwrap(),
                value: value as u32,
            },
            _ => UN::UN64 {
                bit_count: U6::new(bit_count).unwrap(),
                value,
            },
        }
    }

    fn in_(variant: u8, bit_count: u8, value: i64) -> IN {
        match variant {
            0 => IN::IN8 {
                bit_count: U3::new(bit_count).unwrap(),
                value: value as i8,
            },
            1 => IN::IN16 {
                bit_count: U4::new(bit_count).unwrap(),
                value: value as i16,
            },
            2 => IN::IN32 {
                bit_count: U5::new(bit_count).unwrap(),
                value: value as i32,
            },
            _ => IN::IN64 {
                bit_count: U6::new(bit_count).unwrap(),
                value,
            },
        }
    }

    /// Largest bit count of each variant.
    const BIT_COUNT_MAX: [u8; 4] = [7, 15, 31, 63];

    #[test]
    fn un_bytes() {
        let mut buf = [0u8; 16];
        // 000, 011, 101
        assert_eq!(un(0, 3, 5).to_ww_bytes(&mut buf).unwrap(), [0x0E, 0x80]);
        // 010, 10100, then 1 and 19 zeros
        assert_eq!(
            un(2, 20, 1 << 19).to_ww_bytes(&mut buf).unwrap(),
            [0x54, 0x80, 0x00, 0x00]
        );
        // no value bits
        assert_eq!(un(0, 0, 0).to_ww_bytes(&mut buf).unwrap(), [0x00]);
    }

    #[test]
    fn in_bytes() {
        let mut buf = [0u8; 16];
        // 000, 100, 1000
        assert_eq!(in_(0, 4, -8).to_ww_bytes(&mut buf).unwrap(), [0x12, 0x00]);
        // the same bits as the unsigned 8 in 4 bits
        assert_eq!(
            in_(0, 4, -8).to_ww_bytes(&mut buf).unwrap().to_vec(),
            un(0, 4, 8).to_ww_bytes(&mut [0u8; 16]).unwrap().to_vec()
        );
    }

    #[test]
    fn value_wider_than_its_bit_count_is_refused() {
        let mut buf = [0u8; 16];
        assert_eq!(
            un(0, 3, 8).to_ww_bytes(&mut buf),
            Err(Error::SubtypeOutOfRange)
        );
        assert_eq!(
            un(3, 0, 1).to_ww_bytes(&mut buf),
            Err(Error::SubtypeOutOfRange)
        );
        assert_eq!(
            in_(0, 4, 8).to_ww_bytes(&mut buf),
            Err(Error::SubtypeOutOfRange)
        );
        assert_eq!(
            in_(0, 4, -9).to_ww_bytes(&mut buf),
            Err(Error::SubtypeOutOfRange)
        );
        assert_eq!(
            in_(1, 0, -1).to_ww_bytes(&mut buf),
            Err(Error::SubtypeOutOfRange)
        );
    }

    #[test]
    fn unknown_variant_and_short_input_are_errors() {
        // variant 4
        assert_eq!(
            UN::from_ww_bytes(&[0x80, 0x00]),
            Err(Error::EnumFutureVersionOrMalformedData)
        );
        assert_eq!(
            IN::from_ww_bytes_owned(&[0xE0, 0x00]),
            Err(Error::EnumFutureVersionOrMalformedData)
        );
        // UN64 with 63 value bits, cut after the bit count
        assert_eq!(
            UN::from_ww_bytes(&[0x7F, 0xF0]),
            Err(Error::OutOfBoundsReadUN)
        );
    }

    #[test]
    fn between_other_fields() {
        let mut buf = [0u8; 32];
        let mut wr = BufWriter::new(&mut buf);
        wr.write_bool(true).unwrap();
        wr.write(&un(3, 63, u64::MAX >> 1)).unwrap();
        wr.write(&in_(3, 63, i64::MIN >> 1)).unwrap();
        wr.write_u8(0xAA).unwrap();
        let bytes = wr.finish().unwrap();
        let mut rd = BufReader::new(bytes);
        assert_eq!(rd.read_bool(), Ok(true));
        assert_eq!(rd.read::<UN>(), Ok(un(3, 63, u64::MAX >> 1)));
        assert_eq!(rd.read::<IN>(), Ok(in_(3, 63, i64::MIN >> 1)));
        assert_eq!(rd.read_u8(), Ok(0xAA));
    }

    proptest::proptest! {
        #[test]
        fn un_round_trip(variant in 0u8..4, bit_count: u8, value: u64) {
            let bit_count = bit_count % (BIT_COUNT_MAX[variant as usize] + 1);
            let value = value & !(u64::MAX << bit_count);
            let num = un(variant, bit_count, value);
            let mut buf = [0u8; 16];
            let bytes = num.to_ww_bytes(&mut buf).unwrap();
            let bits = 3 + 3 + variant as usize + bit_count as usize;
            proptest::prop_assert_eq!(bytes.len(), bits.div_ceil(8));
            proptest::prop_assert_eq!(UN::from_ww_bytes(bytes), Ok(num));
        }

        #[test]
        fn in_round_trip(variant in 0u8..4, bit_count: u8, value: i64) {
            let bit_count = bit_count % (BIT_COUNT_MAX[variant as usize] + 1);
            let value = sign_extend(value as u64, bit_count);
            let num = in_(variant, bit_count, value);
            let mut buf = [0u8; 16];
            let bytes = num.to_ww_bytes(&mut buf).unwrap();
            proptest::prop_assert_eq!(IN::from_ww_bytes(bytes), Ok(num));
        }

        #[cfg(feature = "std")]
        #[test]
        fn owned_writer_gives_the_same_bytes(variant in 0u8..4, bit_count: u8, value: u64) {
            let bit_count = bit_count % (BIT_COUNT_MAX[variant as usize] + 1);
            let num = un(variant, bit_count, value & !(u64::MAX << bit_count));
            let mut buf = [0u8; 16];
            let bytes = num.to_ww_bytes(&mut buf).unwrap();
            proptest::prop_assert_eq!(crate::SerializeShrinkWrapOwned::to_ww_bytes_owned(&num).unwrap(), bytes);
        }
    }
}
