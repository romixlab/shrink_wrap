//! `elide_defaults` (SW-85): the trailing run of `#[default = ..]` fields that hold their default is left off the
//! wire, so the bytes are those of the type without these fields.

use shrink_wrap::prelude::*;

/// The type before the attributes were added.
#[derive_shrink_wrap(borrowed, owned, derive(Debug, PartialEq, Clone))]
struct MetaV1 {
    id: u16,
    kind: u8,
}

/// The same type, written by a writer that doesn't elide.
#[derive_shrink_wrap(borrowed, owned, derive(Debug, PartialEq, Clone))]
struct MetaFull<'i> {
    id: u16,
    kind: u8,
    #[default = None]
    effects: Option<&'i str>,
    #[default = None]
    track: Option<u8>,
}

/// `MetaV1` with two attribute fields appended, leaving them off while they are `None`.
#[derive_shrink_wrap(borrowed, owned, elide_defaults, derive(Debug, PartialEq, Clone))]
struct Meta<'i> {
    id: u16,
    kind: u8,
    #[default = None]
    effects: Option<&'i str>,
    #[default = None]
    track: Option<u8>,
}

fn bytes<T: SerializeShrinkWrap>(value: &T) -> Vec<u8> {
    let mut buf = [0u8; 128];
    value.to_ww_bytes(&mut buf).unwrap().to_vec()
}

fn bytes_owned<T: SerializeShrinkWrapOwned>(value: &T) -> Vec<u8> {
    value.to_ww_bytes_owned().unwrap()
}

fn meta(effects: Option<&'static str>, track: Option<u8>) -> Meta<'static> {
    Meta {
        id: 7,
        kind: 1,
        effects,
        track,
    }
}

fn full(m: &Meta<'static>) -> MetaFull<'static> {
    MetaFull {
        id: m.id,
        kind: m.kind,
        effects: m.effects,
        track: m.track,
    }
}

#[test]
fn all_default_writes_the_old_bytes() {
    let v1 = MetaV1 { id: 7, kind: 1 };
    let m = meta(None, None);
    assert_eq!(bytes(&m), bytes(&v1));
    assert_eq!(bytes(&m), [0x07, 0x00, 0x01]);
    // without the directive the flags are written
    assert_ne!(bytes(&full(&m)), bytes(&v1));
}

#[test]
fn only_the_trailing_run_is_left_off() {
    // `track` at its default after a set `effects`: the bytes of the type without `track`
    #[derive_shrink_wrap(borrowed, derive(Debug, PartialEq))]
    struct MetaV2<'i> {
        id: u16,
        kind: u8,
        #[default = None]
        effects: Option<&'i str>,
    }
    let m = meta(Some("blink"), None);
    let v2 = MetaV2 {
        id: 7,
        kind: 1,
        effects: Some("blink"),
    };
    assert_eq!(bytes(&m), bytes(&v2));
    // a default field before a set one is written: the run is contiguous
    let m = meta(None, Some(3));
    assert_eq!(bytes(&m), bytes(&full(&m)));
    let m = meta(Some("blink"), Some(3));
    assert_eq!(bytes(&m), bytes(&full(&m)));
}

#[test]
fn round_trip() {
    for m in [
        meta(None, None),
        meta(Some("blink"), None),
        meta(None, Some(3)),
        meta(Some(""), Some(0)),
    ] {
        let b = bytes(&m);
        assert_eq!(Meta::from_ww_bytes(&b).unwrap(), m);
        let owned = MetaOwned::from_ww_bytes_owned(&b).unwrap();
        assert_eq!(bytes_owned(&owned), b, "owned writes the same bytes");
        assert_eq!(owned.effects.as_deref(), m.effects);
        assert_eq!(owned.track, m.track);
    }
}

#[test]
fn old_reader_new_writer() {
    for m in [meta(None, None), meta(Some("blink"), Some(3))] {
        let b = bytes(&m);
        let v1 = MetaV1::from_ww_bytes(&b).unwrap();
        assert_eq!(v1, MetaV1 { id: 7, kind: 1 });
    }
}

#[test]
fn new_reader_old_writer() {
    let v1 = MetaV1 { id: 7, kind: 1 };
    assert_eq!(Meta::from_ww_bytes(&bytes(&v1)).unwrap(), meta(None, None));
    let m = meta(Some("blink"), Some(3));
    assert_eq!(Meta::from_ww_bytes(&bytes(&full(&m))).unwrap(), m);
}

/// Nested in a parent, the elided struct's size slot counts only what it wrote.
#[test]
fn nested() {
    #[derive_shrink_wrap(borrowed, derive(Debug, PartialEq))]
    struct Api<'i> {
        meta: Meta<'i>,
        tail: u8,
    }
    #[derive_shrink_wrap(borrowed, derive(Debug, PartialEq))]
    struct ApiV1 {
        meta: MetaV1,
        tail: u8,
    }
    let api = Api {
        meta: meta(None, None),
        tail: 0xAA,
    };
    let v1 = ApiV1 {
        meta: MetaV1 { id: 7, kind: 1 },
        tail: 0xAA,
    };
    assert_eq!(bytes(&api), bytes(&v1));
    assert_eq!(Api::from_ww_bytes(&bytes(&api)).unwrap(), api);
}

/// A default the reader would not get back from the padding at the end is written: a `bool` after a `bool` reads
/// the zero padding bits of the last byte, `false`, not `true`.
#[derive_shrink_wrap(borrowed, elide_defaults, derive(Debug, PartialEq))]
struct Bits {
    a: bool,
    #[default = true]
    b: bool,
    #[default = false]
    c: bool,
}

#[derive_shrink_wrap(borrowed, derive(Debug, PartialEq))]
struct BitsV1 {
    a: bool,
}

#[test]
fn defaults_that_padding_would_change_are_written() {
    let v = Bits {
        a: true,
        b: true,
        c: false,
    };
    // `c` is left off, `b` is written: a reader would take it from the padding as `false`
    assert_eq!(bytes(&v), [0xC0]);
    assert_eq!(Bits::from_ww_bytes(&bytes(&v)).unwrap(), v);
    let v = Bits {
        a: true,
        b: false,
        c: false,
    };
    assert_eq!(bytes(&v), bytes(&BitsV1 { a: true }));
    assert_eq!(Bits::from_ww_bytes(&bytes(&v)).unwrap(), v);
}

/// A byte-aligned number reads nothing from padding: any default is left off. A `Result` with an `Ok` default reads
/// as the default from padding too: its flag reads 0, and the `Err` value after it runs out of data.
#[derive_shrink_wrap(borrowed, elide_defaults, derive(Debug, PartialEq, Clone))]
struct Numbers<'i> {
    flags: bool,
    #[default = 7]
    timeout: u16,
    #[default = Ok(1)]
    mode: Result<u8, u8>,
    #[default = RefVec::new_bytes(&[])]
    tags: RefVec<'i, u8>,
}

#[derive_shrink_wrap(borrowed, derive(Debug, PartialEq))]
struct NumbersV1 {
    flags: bool,
}

#[test]
fn any_default_of_a_byte_aligned_field() {
    let v = Numbers {
        flags: true,
        timeout: 7,
        mode: Ok(1),
        tags: RefVec::new_bytes(&[]),
    };
    assert_eq!(bytes(&v), bytes(&NumbersV1 { flags: true }));
    assert_eq!(Numbers::from_ww_bytes(&bytes(&v)).unwrap(), v);
    // every combination round-trips
    for timeout in [7u16, 8] {
        for mode in [Ok::<u8, u8>(1), Err(1)] {
            for tags in [&[][..], &[1, 2]] {
                let v = Numbers {
                    flags: false,
                    timeout,
                    mode,
                    tags: RefVec::new_bytes(tags),
                };
                let b = bytes(&v);
                assert_eq!(Numbers::from_ww_bytes(&b).unwrap(), v);
            }
        }
    }
}

/// A string after a `bool` reads its length from the free nibble of the last byte, 0: the bytes without the field
/// read as `""`, so a default `"none"` is written.
#[derive_shrink_wrap(borrowed, elide_defaults, derive(Debug, PartialEq))]
struct Label<'i> {
    flags: bool,
    #[default = "none"]
    label: &'i str,
}

#[test]
fn a_default_the_free_nibble_would_change_is_written() {
    let none = Label {
        flags: true,
        label: "none",
    };
    assert_eq!(bytes(&none), [0x80, b'n', b'o', b'n', b'e', 0x04]);
    assert_eq!(Label::from_ww_bytes(&bytes(&none)).unwrap(), none);
    assert_eq!(Label::from_ww_bytes(&[0x80]).unwrap().label, "");
}

/// The owned side compares with the same default: an owned default that doesn't allocate.
#[derive_shrink_wrap(owned, elide_defaults, derive(Debug, PartialEq))]
struct LabelsOwned {
    id: u8,
    #[default = String::new()]
    label: String,
    #[default = Vec::new()]
    tags: Vec<u8>,
}

#[test]
fn owned_defaults() {
    let v = LabelsOwned {
        id: 1,
        label: String::new(),
        tags: vec![],
    };
    assert_eq!(bytes_owned(&v), [0x01]);
    assert_eq!(LabelsOwned::from_ww_bytes_owned(&[0x01]).unwrap(), v);
    let v = LabelsOwned {
        id: 1,
        label: "a".to_owned(),
        tags: vec![],
    };
    let b = bytes_owned(&v);
    assert_eq!(LabelsOwned::from_ww_bytes_owned(&b).unwrap(), v);
}

/// Tuple and generic structs take the directive too.
#[derive_shrink_wrap(borrowed, elide_defaults, derive(Debug, PartialEq))]
struct Pair(u8, #[default = None] Option<u16>);

#[derive_shrink_wrap(owned, elide_defaults, derive(Debug, PartialEq))]
struct TaggedOwned<T> {
    value: u8,
    #[default = None]
    extra: Option<T>,
}

#[test]
fn tuple_and_generic() {
    assert_eq!(bytes(&Pair(5, None)), [0x05]);
    assert_eq!(Pair::from_ww_bytes(&[0x05]).unwrap(), Pair(5, None));
    let t = TaggedOwned::<u32> {
        value: 5,
        extra: None,
    };
    assert_eq!(bytes_owned(&t), [0x05]);
    let t = TaggedOwned::<u32> {
        value: 5,
        extra: Some(9),
    };
    let b = bytes_owned(&t);
    assert_eq!(TaggedOwned::<u32>::from_ww_bytes_owned(&b).unwrap(), t);
}

/// `MAX_SIZE` still bounds every value: elision only makes it shorter.
#[test]
fn max_size_still_bounds() {
    assert_eq!(
        <Bits as SerializeShrinkWrap>::MAX_SIZE,
        <BitsFullForSize as SerializeShrinkWrap>::MAX_SIZE
    );
}

#[derive_shrink_wrap(borrowed, derive(Debug, PartialEq))]
struct BitsFullForSize {
    a: bool,
    #[default = true]
    b: bool,
    #[default = false]
    c: bool,
}
