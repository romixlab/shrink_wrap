---
level: basic
---

# ShrinkWrap

![ShrinkWrap logo](../assets/shrink_wrap_logo.svg#only-light){ width="420" }
![ShrinkWrap logo](../assets/shrink_wrap_logo-dark.svg#only-dark){ width="420" }
{ style="text-align:center" }

ShrinkWrap is the wire format of WireWeaver: every method argument, return value, property and stream item goes
through it. It is dense (values are packed down to single bits), it works on a microcontroller without an allocator,
and it lets newer and older versions of a type read each other's data.

It lives in its own crate, `shrink_wrap`, with no dependency on the rest of WireWeaver. If all you need is a compact
format for files, messages or flash records, you can use it on its own.

## Where to start

The Basic part of the book teaches ShrinkWrap on one example, a sensor node that sends its readings over a radio
link. [Your types on the wire](../basic/types_on_the_wire.md) puts a first struct on the wire and explains its bytes
bit by bit, and the chapters after it cover borrowed and owned types, the built-in types, structs and enums, and
types that change. This page sums up what the format gives you and how it stays small.

## What you get

- **Small.** Values are packed at 1-bit, 4-bit or 1-byte alignment, lengths take a nibble when they are small, and
  nothing is stored that the reader already knows (no field names, no type tags).
- **`no_std` and no allocator,** also for strings, lists and nested types, for reading and writing. On the host,
  the same type gets an owned twin (`Sample` → `SampleOwned`) with `String` and `Vec`, writing the same bytes.
- **Zero-copy reading.** A `&str` or a list read from the buffer points into it.
- **Safe on untrusted data.** A malformed buffer is an error, never a panic, and a recursive type nested too deep
  is `Error::NestingTooDeep` instead of a stack overflow ([nesting depth](wire_format.md#nesting-depth)).
- **Evolvable.** By default a type can get new fields later; old and new code keep reading each other's data
  ([Types that change](../basic/types_that_change.md)).
- **Many types out of the box:** integers of any bit width, variable-length numbers, strings and lists with an
  optional maximum length, `Option`, `Result`, tuples, arrays, compressed time series and more
  ([Built-in types](../basic/built_in_types.md)).

## How it stays small

**Bits are packed.** A `bool` takes one bit, a `U3` three bits, an enum with four variants can take two. Byte-sized
values (`u8`, `u32`, `f32`, strings) start on a fresh byte, the unused bits before them stay zero. Those zero bits
are not wasted forever: a later version of a type can put a new `bool` or `Option` flag there, and old data reads
as `false` or `None`.

**Lengths go to the back.** A string, a list or a nested type whose size isn't known in advance needs its length
stored somewhere. Most formats write it in front of the value, which means knowing the size before writing it.
ShrinkWrap writes the value first and puts its length at the end of the buffer, as a variable-length number of
nibbles. That keeps serialization a single pass with no copying, and small lengths cost half a byte. The reader
picks the lengths up from the back while it walks the data from the front.

**Nothing extra by default, less if you ask.** A struct is evolvable by default, which costs a length nibble or
two when it is nested in another type. A type that will never change can say so (`sized`, `final_structure`,
`self_describing`) and then costs exactly its fields ([size assumptions](size_assumptions.md)).

For the exact bytes, bit by bit, see [wire format internals](wire_format.md).

## Next steps

| Page                                     | Level    | What's there                                                                          |
|------------------------------------------|----------|---------------------------------------------------------------------------------------|
| [Your types on the wire](../basic/types_on_the_wire.md) | Basic | A first struct on the wire, bit by bit                                |
| [Borrowed and owned](../basic/borrowed_and_owned.md) | Basic   | Zero-copy types for firmware, owned twins for hosts                       |
| [Built-in types](../basic/built_in_types.md) | Basic   | Every type that goes on the wire and what it costs                                     |
| [Structs and enums](../basic/structs_and_enums.md) | Basic | Your own types, discriminants and variants a reader doesn't know            |
| [Types that change](../basic/types_that_change.md) | Basic | New fields and variants that old and new readers both read                  |
| [Derive in depth](derive.md)             | Advanced | Type mapping, defaults and serde, tuple structs, generic types, directives            |
| [Types in depth](../types.md)            | Advanced | Bounded lists in an API, recursive and generic types, aliases                          |
| [Size assumptions](size_assumptions.md)  | Advanced | `sized`, `final_structure`, `self_describing`: smaller, but frozen                     |
| [Derive reference](derive_reference.md)  | Advanced | Every directive of the macro                                                           |
| [Special types](special_types.md)        | Advanced | `TailSize`, `ArrayString`/`ArrayVec`, compressed sequences, `OptionAnyVec`             |
| [Fixed and sparse layouts](layouts.md)   | Advanced | `fixed_layout`, `head_tail` and `sparse`: other encodings for special cases            |
| [Showcase](showcase.md)                  | Advanced | Worked, byte-verified tricks                                                           |
| [Use cases](use_cases.md)                | Advanced | Real files, frames and messages built with ShrinkWrap                                  |
| [ShrinkWrap in C](../c.md)               | Advanced | The C reader and writer                                                                |
| [Wire format internals](wire_format.md)  | Nomicon  | The FIFO of lengths, `BufWriter`/`BufReader`, `ElementSize`, exact encodings           |
| [Code size on small CPUs](code_size.md)  | Nomicon  | What ShrinkWrap costs in flash and why                                                 |
