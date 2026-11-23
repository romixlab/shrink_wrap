//! Field and variant attributes for one generated type only (SW-86): `#[attr_owned(..)]`, `#[attr_borrowed(..)]`, and
//! `#[serde(..)]` going to the types that derive serde.

use shrink_wrap::prelude::*;

/// A unit written as text, the way `exact_units::serde::with_unit` writes a quantity.
mod with_unit {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(ms: &u32, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{ms} ms"))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
        let text = String::deserialize(d)?;
        let number = text
            .strip_suffix(" ms")
            .ok_or_else(|| D::Error::custom("no ms"))?;
        number.parse().map_err(D::Error::custom)
    }
}

/// Only the owned type derives serde: the unmarked `serde` attribute goes there, the borrowed type has no serde.
#[derive_shrink_wrap(
    borrowed,
    owned,
    derive(Debug, PartialEq),
    derive_owned(Clone, serde::Serialize, serde::Deserialize)
)]
struct Settings<'i> {
    laugh_tokens: RefVec<'i, &'i str>,
    #[serde(with = "with_unit")]
    delay: u32,
    #[attr_owned(serde(rename = "lvl"))]
    level: u8,
}

#[test]
fn serde_on_the_owned_type_shrink_wrap_on_both() {
    let owned = SettingsOwned {
        laugh_tokens: vec!["ha".into(), "lol".into()],
        delay: 300,
        level: 2,
    };
    let json = serde_json::to_string(&owned).unwrap();
    assert_eq!(
        json,
        r#"{"laugh_tokens":["ha","lol"],"delay":"300 ms","lvl":2}"#
    );
    assert_eq!(serde_json::from_str::<SettingsOwned>(&json).unwrap(), owned);

    let bytes = owned.to_ww_bytes_owned().unwrap();
    let mut buf = [0u8; 32];
    let borrowed = Settings {
        laugh_tokens: RefVec::Slice {
            slice: &["ha", "lol"],
        },
        delay: 300,
        level: 2,
    };
    assert_eq!(borrowed.to_ww_bytes(&mut buf).unwrap(), bytes);
    assert_eq!(Settings::from_ww_bytes(&bytes).unwrap(), borrowed);
    assert_eq!(SettingsOwned::from_ww_bytes_owned(&bytes).unwrap(), owned);
}

/// Both types derive `Serialize` (the owned one under a condition): each takes its own attributes, and the unmarked
/// one goes on both, under the owned type's condition there.
#[derive_shrink_wrap(
    borrowed,
    owned,
    derive_borrowed(serde::Serialize),
    cfg_attr_owned(all(), derive(serde::Serialize))
)]
struct Both<'i> {
    #[attr_borrowed(serde(rename = "b"))]
    #[attr_owned(serde(rename = "o"))]
    name: &'i str,
    #[serde(rename = "n")]
    count: u8,
}

#[test]
fn each_side_its_own() {
    let borrowed = serde_json::to_string(&Both {
        name: "x",
        count: 1,
    })
    .unwrap();
    assert_eq!(borrowed, r#"{"b":"x","n":1}"#);
    let owned = serde_json::to_string(&BothOwned {
        name: "x".into(),
        count: 1,
    })
    .unwrap();
    assert_eq!(owned, r#"{"o":"x","n":1}"#);
}

/// Variants and their fields, named and unnamed, and the `#[future_version]` variant.
#[derive_shrink_wrap(
    borrowed,
    owned,
    ww_repr = u4,
    derive(Debug, PartialEq),
    derive_owned(serde::Serialize)
)]
enum Event<'i> {
    #[attr_owned(serde(rename = "said"))]
    Say {
        #[attr_owned(serde(rename = "t"))]
        text: &'i str,
    },
    Wait(#[serde(with = "with_unit")] u32),
    #[future_version]
    #[attr_owned(serde(rename = "unknown"))]
    Unknown(u32),
}

#[test]
fn enum_variants_and_fields() {
    let say = serde_json::to_string(&EventOwned::Say { text: "hi".into() }).unwrap();
    assert_eq!(say, r#"{"said":{"t":"hi"}}"#);
    let wait = serde_json::to_string(&EventOwned::Wait(5)).unwrap();
    assert_eq!(wait, r#"{"Wait":"5 ms"}"#);
    let unknown = serde_json::to_string(&EventOwned::Unknown(9)).unwrap();
    assert_eq!(unknown, r#"{"unknown":9}"#);
    let mut buf = [0u8; 16];
    let bytes = Event::Say { text: "hi" }.to_ww_bytes(&mut buf).unwrap();
    assert_eq!(
        EventOwned::from_ww_bytes_owned(bytes).unwrap(),
        EventOwned::Say { text: "hi".into() }
    );
}

/// An ambiguous type with both sides is one type: it takes the attributes of both.
#[derive_shrink_wrap(borrowed, owned, derive(serde::Serialize))]
struct Plain {
    #[attr_borrowed(serde(rename = "b"))]
    a: u8,
    #[attr_owned(serde(rename = "o"))]
    b: u8,
}

#[test]
fn one_type_takes_both() {
    let json = serde_json::to_string(&Plain { a: 1, b: 2 }).unwrap();
    assert_eq!(json, r#"{"b":1,"o":2}"#);
}

/// Several attributes in one `attr_owned`, and a `cfg_attr` inside it.
#[derive_shrink_wrap(owned, derive(serde::Serialize))]
struct Many {
    #[attr_owned(
        serde(rename = "x"),
        cfg_attr(all(), serde(skip_serializing_if = "is_zero"))
    )]
    a: u8,
}

fn is_zero(v: &u8) -> bool {
    *v == 0
}

#[test]
fn several_in_one() {
    assert_eq!(serde_json::to_string(&Many { a: 1 }).unwrap(), r#"{"x":1}"#);
    assert_eq!(serde_json::to_string(&Many { a: 0 }).unwrap(), "{}");
}
