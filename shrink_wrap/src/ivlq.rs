//! Signed byte-based variable length numbers (SW-82): [`IVlq32`] and [`IVlq64`], zigzag over [`UVlq32`] and [`UVlq64`].

use core::fmt::{Debug, Display, Formatter};

use crate::nib32::{unzigzag32, zigzag32};
use crate::{
    BufReader, BufWriter, DeserializeShrinkWrap, DeserializeShrinkWrapOwned, ElementSize, Error,
    SerializeShrinkWrap, UVlq32, UVlq64,
};

/// Protobuf-style zigzag: the sign in the lowest bit, the magnitude above it.
pub(crate) const fn zigzag64(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

pub(crate) const fn unzigzag64(z: u64) -> i64 {
    ((z >> 1) as i64) ^ -((z & 1) as i64)
}

macro_rules! ivlq {
    (
        $(#[$doc:meta])*
        $ty:ident($int:ty) over $unsigned:ident, $zigzag:ident, $unzigzag:ident, $write_owned:ident
    ) => {
        $(#[$doc])*
        ///
        /// On the wire it is exactly the unsigned type of the zigzag-encoded value: `0`, `-1`, `1`, `-2`, `2`, .. map
        /// to `0`, `1`, `2`, `3`, `4`, .., so `-64..=63` take one byte. A malformed read fails with the unsigned
        /// type's error.
        #[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        pub struct $ty(pub $int);

        impl $ty {
            /// The zigzag-encoded value, as the unsigned number written on the wire.
            pub const fn to_zigzag(self) -> $unsigned {
                $unsigned($zigzag(self.0))
            }

            /// The value an unsigned number `z` zigzag-decodes to.
            pub const fn from_zigzag(z: $unsigned) -> Self {
                $ty($unzigzag(z.0))
            }

            /// How many bytes the value takes on the wire.
            pub fn len_bytes(&self) -> usize {
                self.to_zigzag().len_bytes()
            }

            pub(crate) fn write_forward(&self, wr: &mut BufWriter) -> Result<(), Error> {
                self.to_zigzag().write_forward(wr)
            }

            pub(crate) fn read_forward(rd: &mut BufReader) -> Result<Self, Error> {
                $unsigned::read_forward(rd).map(Self::from_zigzag)
            }
        }

        impl SerializeShrinkWrap for $ty {
            const ELEMENT_SIZE: ElementSize = ElementSize::SelfDescribing;
            const PUSHES_LEN: bool = false;
            const MAX_SIZE: Option<crate::max_size::MaxSize> =
                <$unsigned as SerializeShrinkWrap>::MAX_SIZE;

            fn ser_shrink_wrap(&self, wr: &mut BufWriter) -> Result<(), Error> {
                self.write_forward(wr)
            }
        }

        #[cfg(feature = "std")]
        impl crate::SerializeShrinkWrapOwned for $ty {
            const ELEMENT_SIZE: ElementSize = ElementSize::SelfDescribing;

            fn ser_shrink_wrap_owned(&self, wr: &mut crate::BufWriterOwned) -> Result<(), Error> {
                wr.$write_owned(self.0)
            }
        }

        impl<'i> DeserializeShrinkWrap<'i> for $ty {
            const ELEMENT_SIZE: ElementSize = ElementSize::SelfDescribing;

            fn des_shrink_wrap<'di>(rd: &'di mut BufReader<'i>) -> Result<Self, Error> {
                $ty::read_forward(rd)
            }
        }

        impl DeserializeShrinkWrapOwned for $ty {
            const ELEMENT_SIZE: ElementSize = ElementSize::SelfDescribing;

            fn des_shrink_wrap_owned(rd: &mut BufReader<'_>) -> Result<Self, Error> {
                $ty::read_forward(rd)
            }
        }

        impl From<$ty> for $int {
            fn from(value: $ty) -> Self {
                value.0
            }
        }

        impl From<$int> for $ty {
            fn from(num: $int) -> Self {
                $ty(num)
            }
        }

        impl Debug for $ty {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl Display for $ty {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

ivlq! {
    /// Variable length encoded i32 based on bytes: zigzag over [UVlq32], 1 to 5 bytes, alignment of 1 byte.
    IVlq32(i32) over UVlq32, zigzag32, unzigzag32, write_ivlq32
}

ivlq! {
    /// Variable length encoded i64 based on bytes: zigzag over [UVlq64], 1 to 10 bytes, alignment of 1 byte.
    IVlq64(i64) over UVlq64, zigzag64, unzigzag64, write_ivlq64
}

#[cfg(test)]
mod test {
    use super::{IVlq32, IVlq64, unzigzag64, zigzag64};
    use crate::{
        BufReader, BufWriter, DeserializeShrinkWrap, DeserializeShrinkWrapOwned, Error,
        SerializeShrinkWrap, UVlq32, UVlq64,
    };

    #[test]
    fn zigzag64_mapping() {
        for (v, z) in [
            (0i64, 0u64),
            (-1, 1),
            (1, 2),
            (-2, 3),
            (i64::MAX, u64::MAX - 1),
            (i64::MIN, u64::MAX),
        ] {
            assert_eq!(zigzag64(v), z);
            assert_eq!(unzigzag64(z), v);
        }
    }

    #[test]
    fn same_bytes_as_the_unsigned_zigzag() {
        let mut a = [0u8; 16];
        let mut b = [0u8; 16];
        for v in [0i64, -1, 63, -64, 64, -65, i64::MAX, i64::MIN] {
            assert_eq!(
                IVlq64(v).to_ww_bytes(&mut a).unwrap(),
                UVlq64(zigzag64(v)).to_ww_bytes(&mut b).unwrap(),
                "{v}"
            );
        }
        for v in [0i32, -1, 63, -64, 64, -65, i32::MAX, i32::MIN] {
            assert_eq!(
                IVlq32(v).to_ww_bytes(&mut a).unwrap(),
                UVlq32(IVlq32(v).to_zigzag().0).to_ww_bytes(&mut b).unwrap(),
                "{v}"
            );
        }
    }

    #[test]
    fn one_byte_range() {
        assert_eq!(IVlq64(63).len_bytes(), 1);
        assert_eq!(IVlq64(-64).len_bytes(), 1);
        assert_eq!(IVlq64(64).len_bytes(), 2);
        assert_eq!(IVlq64(-65).len_bytes(), 2);
        assert_eq!(IVlq32(i32::MIN).len_bytes(), 5);
        assert_eq!(IVlq64(i64::MIN).len_bytes(), 10);
    }

    #[test]
    fn byte_aligned() {
        let mut buf = [0u8; 8];
        let mut wr = BufWriter::new(&mut buf);
        wr.write_bool(true).unwrap();
        wr.write_ivlq32(-65).unwrap();
        wr.write_ivlq64(-1).unwrap();
        let bytes = wr.finish().unwrap();
        assert_eq!(bytes.len(), 4);
        let mut rd = BufReader::new(bytes);
        assert_eq!(rd.read_bool(), Ok(true));
        assert_eq!(rd.read_ivlq32(), Ok(-65));
        assert_eq!(rd.read_ivlq64(), Ok(-1));
    }

    #[test]
    fn malformed_read_is_the_unsigned_error() {
        let buf = [0x80; 11];
        assert_eq!(IVlq32::from_ww_bytes(&buf), Err(Error::MalformedUVlq32));
        assert_eq!(
            IVlq64::from_ww_bytes_owned(&buf),
            Err(Error::MalformedUVlq64)
        );
    }

    proptest::proptest! {
        #[test]
        fn round_trip_32(num: i32) {
            let mut buf = [0u8; 8];
            let bytes = IVlq32(num).to_ww_bytes(&mut buf).unwrap();
            proptest::prop_assert_eq!(bytes.len(), IVlq32(num).len_bytes());
            proptest::prop_assert_eq!(IVlq32::from_ww_bytes(bytes), Ok(IVlq32(num)));
        }

        #[test]
        fn round_trip_64(num: i64) {
            let mut buf = [0u8; 16];
            let bytes = IVlq64(num).to_ww_bytes(&mut buf).unwrap();
            proptest::prop_assert_eq!(bytes.len(), IVlq64(num).len_bytes());
            proptest::prop_assert_eq!(IVlq64::from_ww_bytes(bytes), Ok(IVlq64(num)));
        }

        #[cfg(feature = "std")]
        #[test]
        fn owned_writer_matches(a: i32, b: i64) {
            use crate::SerializeShrinkWrapOwned;
            let mut buf = [0u8; 16];
            proptest::prop_assert_eq!(
                IVlq32(a).to_ww_bytes_owned().unwrap(),
                IVlq32(a).to_ww_bytes(&mut buf).unwrap()
            );
            proptest::prop_assert_eq!(
                IVlq64(b).to_ww_bytes_owned().unwrap(),
                IVlq64(b).to_ww_bytes(&mut buf).unwrap()
            );
        }
    }
}
