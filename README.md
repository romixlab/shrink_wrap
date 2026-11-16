# ShrinkWrap

![Crates.io Version](https://img.shields.io/crates/v/shrink_wrap)
[![CI](https://github.com/romixlab/shrink_wrap/actions/workflows/ci.yml/badge.svg)](https://github.com/romixlab/shrink_wrap/actions/workflows/ci.yml)

<p align="center">
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/vhrdtech/wire_weaver/master/docs/assets/shrink_wrap_logo-dark.png">
  <img src="https://raw.githubusercontent.com/vhrdtech/wire_weaver/master/docs/assets/shrink_wrap_logo.png" alt="ShrinkWrap logo" width="480"/>
</picture>
</p>

> Compact zero-copy wire format for microcontrollers using no allocator and supporting dynamic types.

## Design goals

- Support for user-defined types - structs, enums (plain and with data fields), tuples.
- Support for dynamically sized objects (strings, vectors or any user type).
- Rich type system.
  - [Full list of supported types](https://ww.vhrd.tech/types/)
- Backwards and Forwards compatibility (older device with new software / old software with newer device).
- No allocation.
- 1-bit, 4-bit and 1-byte alignment.
- No buffer alignment requirements (1 byte).
- Self-referential types.
- Support for data and protocol evolution.
- Dense wire format that can fit useful things into small messages (e.g., CAN Bus frame).
- Zero-copy de-serialization.
- `StackVec` for storing an object with arbitrary max-bound size on stack.

## Where used

This crate is a core piece of [WireWeaver](https://github.com/vhrdtech/wire_weaver) - lightweight microcontroller API
code generator with support for methods (RPC), properties,
streams, [global traits](https://github.com/vhrdtech/ww_stdlib) and backward/forward compatibility.

## Documentation

The pages in [`docs/`](docs) describe the format, the derive macro and its directives, the special types, `Series`,
the layouts and what the code costs on small CPUs. They are published as the SerDes section of the
[WireWeaver book](https://ww.vhrd.tech/serdes/shrink_wrap/), which assembles them from this repository, so their links
to other parts of the book work there.

## Repository layout

| Path | What |
| --- | --- |
| `shrink_wrap/` | the wire format: `BufReader`, `BufWriter`, the serdes traits, the special types, `Series` |
| `shrink_wrap_derive/` | `#[derive_shrink_wrap]` |
| `size_bench/` | code size of shrink_wrap on small CPUs: the runner, its firmware images (`fw/`) and their runtime (`fw/rt`) |
| `fuzz/` | cargo-fuzz targets of the readers and writers |
| `docs/` | the pages of the format |

`just --list` shows the recipes: `just test`, `just lint`, `just fuzz <target>`, `just size-bench`, `just miri`.

## Recommended way to depend on this crate

### On no_std

```toml
[dependencies]
shrink_wrap = { git = "https://github.com/romixlab/shrink_wrap", default-features = false }
```

To make this crate `no_std`. Releases up to 0.1.2 are on crates.io.

### In API or data type crates for both std and no_std

```toml
[dependencies]
shrink_wrap = { git = "https://github.com/romixlab/shrink_wrap", default-features = false }

[features]
default = ["std"]
std = ["shrink_wrap/std"]
```

And put `#![cfg_attr(not(feature = "std"), no_std)]` in `lib.rs`.
Then use your crate with `default-features = false` on `no_std`.

## Implementation details

- Attribute macro that generates serdes code.
  - `#[owned = "feature"]` attribute to generate TyOwned from Ty<'i> and serdes code for it as well.
- `BufWriter` and `BufReader` that do low-level work with byte buffers, can be used stand-alone as well.
- Main trick is keeping lengths of objects in the back of the buffer and read/write from both ends.
- In order to avoid temporary allocations (or two buffers) during serialization, back of the buffer is used as a stack
  as well.
- Two forms of variable length numbers are used - UNib32 and ReverseUNib32 (nibble based).
- Rust type system and const evaluation is leveraged to automatically compose objects inside of objects:
  - Sized and SelfDescribing objects are serialized as is, without storing length.
  - FinalStructure types (e.g., `Vec<T>`) pick-up `T`’s kind and become it to save even more space.
  - Unsized - size is calculated and stored, can be evolved, default kind for user structs and enums.
- `U1`..`U64` and `I2`..`I64` bit-aligned types are provided as well.
- Can use, e.g., `U1` which takes only one bit as enum discriminant!
- Can use uninitialised buffers when writing, so no need to spend time on zero-filling.
- Downside is - buffer length need to be known, usually not a problem or taken care of by transport layer.

## License

MIT, see [LICENSE](LICENSE).
