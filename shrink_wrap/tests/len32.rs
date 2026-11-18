//! `BufWriter` with the `len32` feature (SW-83): lengths wait as `u32`, so a value may pass 65_535 bytes, and each
//! waiting length takes 4 bytes of the buffer. Run by `just test` with `--features len32`.
#![cfg(feature = "len32")]

use hex_literal::hex;
use shrink_wrap::prelude::*;
use shrink_wrap::{BufWriter, Error};

#[derive_shrink_wrap(derive(Debug, PartialEq))]
struct Blob<'i> {
    id: u8,
    data: RefVec<'i, u8>,
}

#[derive_shrink_wrap(derive(Debug, PartialEq))]
struct Outer<'i> {
    blob: Blob<'i>,
    name: &'i str,
}

#[test]
fn long_list_round_trips() {
    let data = vec![0x5A; 70_000];
    let mut buf = vec![0u8; 70_016];
    let mut wr = BufWriter::new(&mut buf);
    wr.write_bytes(&data).unwrap();
    let bytes = wr.finish().unwrap();
    let mut rd = BufReader::new(bytes);
    assert_eq!(rd.read_bytes().unwrap(), &data[..]);
}

#[test]
fn long_nested_value_round_trips() {
    let data = vec![0xA5; 100_000];
    let value = Outer {
        blob: Blob {
            id: 7,
            data: RefVec::new_bytes(&data),
        },
        name: "long",
    };
    let mut buf = vec![0u8; 100_032];
    let bytes = value.to_ww_bytes(&mut buf).unwrap();
    assert!(bytes.len() > 100_000);
    assert_eq!(Outer::from_ww_bytes(bytes).unwrap(), value);
}

#[test]
fn same_bytes_with_four_scratch_bytes_per_length() {
    // two lengths wait while the strings are written: 5 bytes of text and 2 slots of 4 bytes
    let mut small = [0u8; 12];
    assert!(("abc", "de").to_ww_bytes(&mut small).is_err());
    let mut scratch = [0u8; 13];
    let bytes = ("abc", "de").to_ww_bytes(&mut scratch).unwrap();
    assert_eq!(bytes, hex!("61 62 63 64 65 23"));
}

#[test]
fn slots_are_four_bytes() {
    let mut buf = [0u8; 8];
    let mut wr = BufWriter::new(&mut buf);
    let pos = wr.write_rev_len(0).unwrap();
    assert_eq!(wr.bytes_left(), 4);
    wr.update_rev_len(pos, 0x0001_0000).unwrap();
    wr.write_rev_len(1).unwrap();
    assert_eq!(wr.write_rev_len(1), Err(Error::OutOfBoundsRev));
}

#[test]
fn max_size_counts_the_wider_slot() {
    type Name<'i> = MaxLen<&'i str, 3>;
    let max = MaxSize::buffer_len_of(<(Name, Name) as SerializeShrinkWrap>::MAX_SIZE).unwrap();
    let mut buf = vec![0u8; max];
    let value: (Name, Name) = (MaxLen::new("abc").unwrap(), MaxLen::new("def").unwrap());
    assert!(value.to_ww_bytes(&mut buf).is_ok());
}
