---
level: advanced
---

# Derive in depth

The Basic chapters teach `#[derive_shrink_wrap]` on a sensor packet:
[Borrowed and owned](../basic/borrowed_and_owned.md) covers the types it generates,
[Structs and enums](../basic/structs_and_enums.md) the discriminants of enums, and
[Types that change](../basic/types_that_change.md) new fields with `#[default = ..]`. This page covers what those
chapters leave out: the whole type mapping between borrowed and owned types, how a default is read and what serde
makes of it, tuple and unit structs, the details of the variant for a newer version, generic types, and the
directives you meet most. The [derive reference](derive_reference.md) lists every directive.

## Type mapping

The derive makes the owned twin of a borrowed type by mapping each field's type. The whole mapping:

| Borrowed type              | Owned equivalent    |
|----------------------------|---------------------|
| `RefVec<'i, T>`            | `Vec<T>`            |
| `&'i str`                  | `String`            |
| `RefBox<'i, T>`            | `Box<T>`            |
| `MaxLen<RefVec<'i, T>, N>` | `MaxLen<Vec<T>, N>` |
| `MaxLen<&'i str, N>`       | `MaxLen<String, N>` |
| `Series<'i, T>`            | `SeriesOwned<T>`    |
| `UserType<'i>`             | `UserTypeOwned`     |
| `UserType<'i, T>`          | `UserTypeOwned<T>`  |

Generic arguments of a user type are emitted as written (`Twice<u8>` stays `Twice<u8>` on both sides); only a
leading lifetime is replaced by the generated type's, and dropped in the owned type. Everything else (numbers,
arrays, `Option`, `TailSize`, ...) is the same on both sides.

## Evolving types

A field with `#[default = ..]` is one a later version added, as [Types that change](../basic/types_that_change.md)
shows. This section covers how such a field is read, and what serde does with it. It works as long as the type stays
evolvable, which is the default (see [size assumptions](size_assumptions.md) for the alternatives).

A type that also derives `serde::Deserialize` (with `derive(..)` or `cfg_attr(cond, derive(..))`) gets
`#[serde(default = ..)]` on each such field, so serde data written before the field was added (a RON or JSON file
saved by an older version) reads it as its default too, instead of failing with "missing field".

A field with a default reads as its default only when it starts at the end of the data and its read runs out of
data. A field that starts with data left is read as usual: a truncated or malformed value is an error, never
replaced with the default. The default can be any expression of the field's type (`#[default = 7]`,
`#[default = Vec::new()]`, `#[default = Default::default()]`).

!!! note "Padding bits read as 0"
    The last byte of older data can have unused bits (the rest of a byte after a `bool`, or the nibble next to a
    size at the back of the buffer). A new bit-sized field there (`bool`, nibble, the flag of an `Option` or
    `Result`) is read from them as 0, and newer data packs it the same way. So give such fields their zero value as
    the default: `false`, `None`. An `Option` with `#[default = Some(..)]` reads as `None` or the default depending
    on where the old data ends.

See the [evolution rules](../evolution/rules.md) for every change that stays compatible, and the
[evolution checker](../evolution/checker_tool.md) to verify it across versions.

## Tuple and unit structs

`struct Celsius(f32);`, `struct Pair<'i>(u8, &'i str);` and `struct Marker;` take the same directives as a struct
with named fields and are written the same way, field after field. New fields go at the end with
`#[default = ..]`. Their fields are `_0`, `_1`, ... wherever the derive needs a name; `#[flag]` needs named fields.
See [types in depth](../types.md#tuple-structs).

## Enum variants from a newer version

A reader that gets a discriminant it doesn't know would fail with `Error::EnumFutureVersionOrMalformedData`. So
every evolvable enum (the default) gets an extra variant without writing it (SW-47): the derive adds `Unknown(u32)`
as if it were written last. This

```rust
#[derive_shrink_wrap(borrowed, derive(Debug, PartialEq), ww_repr = unib32)]
enum IoError {
    InUse,
    Refused,
}
```

is the same type as

```rust
#[derive_shrink_wrap(borrowed, derive(Debug, PartialEq), ww_repr = unib32)]
enum IoError {
    InUse,
    Refused,
    /// A variant from a newer version of this type, with its discriminant.
    #[future_version]
    Unknown(u32),
}
```

An older host then reports a newer device's error instead of failing to decode the reply, and can pass the value
on. How this fits with the rest of an API's errors: [Errors](../api/errors.md).

- The variant takes no number on the wire: the next version adds `NoIoLevel` as 2 before it, and an older reader
  gets `Unknown(2)`.
- The enum's bytes and sizes are the same as without it, so it can be added in any version.
- New variants may carry data only in an evolvable enum: its size slot lets older readers skip their fields.
- Writing `Unknown(d)` writes the discriminant `d` without fields.
- `discriminant()` returns the wire discriminant, also for `Unknown(d)`.

To leave it out (an enum that is complete by nature, like `Parity`) or rename it, see
[`no_future_version`](derive_reference.md#no_future_version-future_version-name-enums-only).

## Generic types

Structs and enums take type and const parameters and where clauses like any Rust type (DER-14):

```rust
#[derive_shrink_wrap(borrowed, owned, final_structure, derive(Debug, PartialEq))]
struct Timed<T> {
    t: u32,
    value: T,
}

#[derive_shrink_wrap(owned, sized, derive(Debug, PartialEq))]
struct Buf<const N: usize> {
    len: u8,
    data: [u8; N],
}

#[derive_shrink_wrap(borrowed, owned, ww_repr = u4, derive(Debug, PartialEq))]
enum Reading<'i, T: Copy> {
    Value(T),
    Missing,
    Labeled { label: &'i str, value: T },
}
```

The parameters' sizes are part of the type's: `Buf { len: 2, data: [1, 2, 3] }` is the four bytes `02 01 02 03`, and
a `Buf<3>` is `sized` at 32 bits. The derive adds the serialization traits as bounds on each impl, your own bounds
and where clause stay as written. More in the [reference](derive_reference.md#generics); in an API they are used with
their arguments, see [generic types in an API](../types.md#generic-types-in-an-api).

## Directives at a glance

`#[derive_shrink_wrap(directive, directive, ..)]` takes a comma-separated list. The ones you'll meet most:

| Directive                               | What it does                                                                | Details                                                                                   |
|-----------------------------------------|-----------------------------------------------------------------------------|-------------------------------------------------------------------------------------------|
| `borrowed`, `owned`, `owned(feature = "std")` | which types to generate                                               | [Borrowed and owned](../basic/borrowed_and_owned.md#which-types-the-derive-makes)        |
| `derive(..)`                            | regular derives on the generated types                                      | [reference](derive_reference.md#derive-derive_owned-derive_borrowed)                      |
| `ww_repr = ..`                          | an enum's discriminant type                                                 | [reference](derive_reference.md#ww_repr-repr-enums-only)                                  |
| `sized`, `final_structure`, `self_describing` | smaller, but can't be evolved                                         | [size assumptions](size_assumptions.md)                                                   |
| `display`                               | `Display` and `Error` from doc comments, for error types                    | [reference](derive_reference.md#display)                                                  |
| `validate = path`                       | reject out-of-range values on read                                          | [reference](derive_reference.md#validate-path)                                            |
| `fixed_layout`, `head_tail`, `sparse`   | other encodings: constant offsets, a C-friendly head and tail, sparse fields | [layouts](layouts.md)                                                                    |

The [derive reference](derive_reference.md) has all of them, including `cfg_attr(..)`, `#[cfg]` on variants,
`discriminants`, `crate_path` and the field attributes.

## Next steps

- [Types in depth](../types.md): bounded lists in an API, recursive and generic types, aliases.
- [Size assumptions](size_assumptions.md): smaller types that can't change any more.
