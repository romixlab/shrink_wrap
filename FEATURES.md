# shrink_wrap features and roadmap

This file is the single source of truth for what shrink_wrap supports, what is being worked on and what is planned,
for humans and AI agents alike.

## How to use this file

- **Status** of each item:
  - ✅ done
  - 🚧 in progress or partially done (the note says what is missing)
  - 📋 planned
  - 💡 idea, not committed to
  - ⛔ blocked (the note says on what)
  - 🔍 probably done or obsolete, needs a check before closing
  - 🧪 done and tested on the host, the check on hardware is still open (the note says what to check)
  - 🐛 known bug, not yet fixed (the note says how to hit it)
- **Target** is the release an item is planned for (`v0.5` is WireWeaver's next release, which these items were
  planned with). No tag means not scheduled.
- **IDs** (`SW-3`, `DER-8`) are stable: never renumber or reuse one. New items take the next free number of their
  area. Use the ID in commit messages, CHANGELOG entries and code `TODO`s (`// TODO(SW-20): ...`). When talking to
  people, name an ID with a short slug from its title (`SW-3 string-and-vec`), never bare.
- The items came from WireWeaver's `FEATURES.md` with their IDs, where shrink_wrap was developed. IDs of other areas
  (`EVO`, `SRV`, `CLT`, `API`, `STD`, `TEST`, ...) and paths outside this repository's crates (`tests/conformance`,
  `wire_weaver_core`, `docs/api/..`) are [wire_weaver](https://github.com/vhrdtech/wire_weaver)'s. The C runtime
  (`shrink_wrap_c`: SW-43, SW-66, SW-75) and the conformance corpus (SW-67) stayed there, with their items.
- Items are grouped by area. Each area lists what works first (✅), then open items ordered by target.
- When finishing work, update the item in the same change: mark it ✅, move it up to the done items of its area and
  add a pointer (docs page, crate, test or example). Don't delete done items. Move items that turn out to be obsolete
  to [Dropped and superseded](#dropped-and-superseded) with a one-line reason.
- New ideas go into the matching area as 💡. Small code-level gaps stay as `TODO` comments in the source; only those
  that limit users or block a feature get an item here.

## Wire format (`shrink_wrap`, `SW`)

- ✅ **SW-1 Structs, plain enums and data enums**, discriminant type selection with `ww_repr`.
- ✅ **SW-2 `Option` and `Result`**, also inside `Vec`, as one-bit flags.
- ✅ **SW-3 `String` and `Vec<T>`**, with special handling of `Vec<u8>`.
- ✅ **SW-4 Fixed size arrays, tuples and the unit type `()`**.
- ✅ **SW-5 Bit-aligned integers**: nibbles and `u1`..`u32` (`un`); `un8`/`un16` are forwarded to `un32` for smaller
  flash.
- ✅ **SW-6 Variable length numbers**: `UNib32`, `UVlq32`, and `UVlq32Backfill` for values known only after the rest
  is serialized.
- ✅ **SW-7 Borrowed and owned types**: `Sized` and `Unsized` traits, `RefStr`/`RefVec`, borrowed types by default and
  `Owned` variants generated automatically.
- ✅ **SW-8 Evolvable by default**: every struct and enum is `Unsized`; `final_structure`, `self_describing` and
  `sized` are opt-ins.
- ✅ **SW-9 `TailBytes` / `TailBytesOwned`**: bytes that take up the rest of the buffer, written without a size; only
  valid as the last item. Generated API code puts them in place of byte slices, e.g. for return values and
  property values.
- ✅ **SW-10 `EitherAnyVec`**: zero-copy, no-alloc array of `Either` elements of any types, with the L/R flags packed
  in groups of 8 instead of padding each element. Packs e.g. a list of `Result`s tightly; used for multi-read
  and multi-call results (only multi-read exists so far).
- ✅ **SW-55 `OptionAnyVec`**: zero-copy, no-alloc array of `Option` slots of any types (numbers, structs, lists,
  strings, a nested `OptionAnyVec` for a child level), only the present values written, in slot order without
  padding between them. A presence header up front in the smallest of four modes (all, dense bits, two-level bits,
  index list), self-delimiting so that readers with fewer or more slots than the writer read it (slots are only
  appended); a malformed header is an error. Writer takes the present set up front (`SetBits` over a bitset, any
  increasing iterator), `OptionAnyVecWriter` / in-place `OptionAnyVecBuilder`. For a trait level's properties in one
  update (API-38). See `docs/special_types.md#optionanyvec-sparse-slots-of-any-types`, `docs/showcase.md`.
- ✅ **SW-68 Word-wide bit writer and reader**, the same bytes: `BitWriter` with a 32-bit accumulator flushed a word
  at a time, `BitSink::put_block` for packed blocks, `write_un*` / `write_bool` / `UNib32` and the reader's
  `read_un*` / `UNib32` through one 4-byte window and one bounds check per call. The FOC observe poll on an STM32H725
  75.5 k cycles instead of 99.9 k at `opt-level = 3` (121.5 k against 172.2 k at `"z"`), a FOC level batch from the
  store 36.8 k instead of 58.9 k; reads of UN numbers and `UNib32` take 0.4-0.6x the instructions on a Cortex-M4F;
  about 1.5 KB more flash at `"z"`. See `docs/articles/series.md#the-bit-writer-and-reader-a-word-at-a-time`,
  `examples_mcu/mcu_qemu/src/bit_io.rs` (`qemu-bit-io`, `qemu-m4f-bit-io`, `stm32h725ig-bit-io-cycles`).
- ✅ **SW-64 `Series` written straight from a `SeriesStore`**, column by column, the same bytes as the item walk:
  `SeriesStore::columnar` / `write_store` (leaves, `Option`, arrays, tuples, derived structs and enums, by hand for
  `NumericValue`) and `series::write_store_series`, used by `ww_stream`'s batchers on their own. Needs one variant
  per enum and `Option` over the rows (a prefix change falls back to the walk). One FOC level batch (21 x
  `Series<T>` + clock, n = 32, `DeltaPacked`): 74.9 k instructions on a Cortex-M3 instead of 143.5 k, 100 k cycles on
  an STM32H725 instead of 312 k (60 k against 125 k at `opt-level = 3`). See
  `docs/series.md#from-the-store-column-by-column`, `shrink_wrap/src/series/store_write_tests.rs`,
  `examples_mcu/mcu_qemu` (`qemu-store-columns`).
- ✅ **SW-63 Fast and small `Series` encoding on MCUs**, the same bytes: about 6x fewer instructions per item on
  a Cortex-M3/M4F (`Series<Timed<Ampere>>`, n = 32: 341 with a fixed codec, 969 with `SMALLEST`; 25 200 cycles per
  batch of 32 on an STM32H725), and codec policies fixed at compile time (`SeriesPolicy`, `FixedCodecs`,
  `DeltaPacked`, the `no_std` default of `ww_stream`'s batcher) that link only their own codecs: 15 KB of encoder
  flash on a Cortex-M3 instead of 91 KB (25 KB with a run-time `CodecPolicy`). See
  `docs/series.md#cpu-and-flash-on-an-mcu`, `examples_mcu/mcu_qemu`, `examples_mcu/qemu_icount`.
- ✅ **SW-62 `SeriesStore` and series size bounds**: room for `N` items of a `series` type as a struct of arrays
  (`empty_store`, `set`, `get`), derived for structs (field stores) and enums (discriminants and every data
  variant's fields), by hand for `ww_numeric::NumericValue` (a variant byte and one payload as wide as the widest
  variant enabled): `Timed<Ampere>` takes 12 bytes per item with number types up to 32 bits, against 24 as items.
  `SeriesColumns::MAX_ITEM_BITS` / `MAX_FIXED_BITS` and `Series::max_bytes(n)`: the worst case of a batch under any
  codec policy, `None` for strings and opaque fields. `series(no_store)` leaves the derived store out. Piece 1 of
  API-41. See `docs/series.md#collecting-items-on-a-device-seriesstore`,
  `shrink_wrap/src/series/store_tests.rs`, `ww_stdlib/ww_si/tests/store.rs`.
- ✅ **SW-61 `RefVecBuilder<T>`: a list written in place**, element by element (`push`, `extend`, `push_with` for a
  nested builder or an element written by hand), `finish` writes the count; same bytes as `RefVec::Slice`, as the
  whole buffer, a struct field or an element of another list. A failed push undoes its element, `with_max_len` for
  `MaxLen<RefVec<..>, N>`, `RefVecBuilderOwned` for `BufWriterOwned`. A byte list also has `extend_from_slice` and
  `write_bytes_with` (a byte blob written by other means), which replace `RefVecU8Builder`.
  See `docs/special_types.md#refvecbuilder-lists-written-in-place`,
  `shrink_wrap/tests/ref_vec_builder.rs`.
- ✅ **SW-59 Tuple and unit structs in the derive**: `struct Celsius(f32);`, `struct Pair<'i>(u8, &'i str);`,
  `struct Marker;` in every layout and size assumption, same bytes as the struct with the fields named; positional
  names `_0`, `_1`, ... in consts, C names and errors. `#[flag]` needs named fields. The API model has them as
  unnamed fields, dynamic clients and Python (a tuple, a single field its value, a unit `None`) read and write them.
  A unit struct is evolvable (it records its length inside a parent); with `sized`, `final_structure`,
  `self_describing` or `fixed_layout` it is a compile error, as it could never change.
  See `docs/types.md`, `shrink_wrap/tests/tuple_structs.rs`, `tests/dynamic`.
- ✅ **SW-76 A refused value with its numbers** — a `validate = path` function may return
  `shrink_wrap::refused::Refused` (the stored number read and the rule it broke: a range `low..=high`, NaN, ±∞, zero,
  a code that means no value) instead of a plain `Error`. The read still fails with `SubtypeOutOfRange`, so `Error`
  stays a fieldless `u32` on firmware; with the `refused` feature (on with `std`) a reader made with
  `with_refusal_slot(&slot)` writes it into the caller's `refused::Slot`, shared by every reader split off it and by
  no other read, one pointer more in `BufReader` (40 to 48 B on a 64-bit host); `from_ww_bytes_explained` makes one
  and returns a `ReadError` (`"6000 is out of 1..=5000"`), the first refusal of the read. Without the feature (firmware) the reader doesn't grow and
  the numbers are dropped where they are made. A `Series` item's hook reports the plain error only. exact_units'
  `Fixed`, `Index`, `IndexFrom1`, `Niched` and `Coded` hooks moving to it is exact_units' side. Tests
  `shrink_wrap/tests/refused.rs`, `docs/derive_reference.md#validate-path`.
- ✅ **SW-58 `validate = path` derive directive**: a function checking every struct value just read, so a derived
  type (and the API model it gives to dynamic clients) can still reject out-of-range data; used by `ww_date_time::Year`.
- ✅ **SW-57 `ArrayString<N>` / `ArrayVec<T, N>`**: owned, no-alloc string/list types backed by a fixed-size
  array, same bytes on the wire as `&str`/`String` and `Vec<T>`, for an owned no-alloc field (a `value_on_changed`
  field of a string/list property in a `no_alloc` server, a settings value); `Error::MaxLenExceeded` rather than a
  cut value, for serializing, deserializing, `push_str`/`push` and `TryFrom<&str>` alike.
  `docs/special_types.md#arraystring-arrayvec-owned-values-without-an-allocator`.
- ✅ **SW-11 Alloc writer** `BufWriterOwned`, `SerializeShrinkWrap` for `&[u8]`, `String` as UFS.
- ✅ **SW-12 In-place builders** used by the server codegen (event and error builders), and the manual
  patch-the-discriminant-later pattern (`docs/showcase.md`).
- ✅ **SW-13 Binary format documented** in `docs/wire_format.md`, worked tricks in `docs/showcase.md`.
- ✅ **SW-14 Usable standalone** as a dense data storage format (`examples/minimal_shrink_wrap`,
  `examples/compare_wire_formats`).
- ✅ **SW-34 Smaller code on small CPUs**: `BufReader` / `BufWriter` reads and writes that keep an error instead of
  returning it (`read_u8_latch`, `write_u32_latch`, .., `latched()`), used by `#[derive_shrink_wrap]` for plain
  fields with one check per type; `Error` without payloads; one shared integer read and write; no division in
  `UNib32`. 19 to 20 % less code for a derived request/reply codec on RV32IC and Cortex-M0, same wire format.
  Numbers, where the rest goes and what is left to do: `docs/code_size.md` (measured by TEST-17).
- ✅ **SW-36 No length FIFO code for types without lengths**: `SerializeShrinkWrap::PUSHES_LEN`, computed by
  `#[derive_shrink_wrap]` from the fields (default `true` unless `Sized`), lets `to_ww_bytes` and
  `BufWriter::finish_for::<T>()` end with `finish_no_lens()` at compile time, so the FIFO encoder is not linked:
  about 510 bytes less per such encoder on RV32IC (a struct of plain fields 1476 → 964). For types with lengths
  `encode_len_fifo` / `write_nib` are smaller (`vec_ser` 1378 → 1142). Same wire format
  (`shrink_wrap/tests/pushes_len.rs`, `docs/code_size.md` steps 4 and 5).
- ✅ **SW-37 `RefVec` of plain numbers without the decode pass**: a list of `u8`..`u128`, `i8`..`i128`, `f32` or
  `f64` finds its end with one bounds check instead of reading every element (4096 bytes: 10 µs → 6 ns on a host),
  same result, errors and reader state as the pass, same wire format (`shrink_wrap/tests/ref_vec_skip.rs`,
  `just fuzz-shrink-wrap-ref-vec`). Costs 50 to 140 bytes of code in a decoder with such a list. Other `Sized` types
  keep the pass on purpose (stride is not `size_bits`, elements are validated there):
  `docs/code_size.md`.
- ✅ **SW-38 Latched reads of `U1`..`U64` / `I2`..`I64` fields**: read like the integers of SW-34, with one error
  check per type (through a hidden `DeserializeShrinkWrap::des_shrink_wrap_latch`); a bit-packed struct decodes in
  40 bytes less on RV32IC (886 → 846), 88 on RV32I. Same wire format, results and errors as reading field by field
  (`shrink_wrap/tests/latched_reads.rs`, `just fuzz-shrink-wrap-latched`). `Nibble`, `u64` and varints are not
  latched: one such field costs more than it saves (`docs/code_size.md`).
- ✅ **SW-41 First error in types that mix latched and other fields**: an error kept by a `_latch` read stops the
  reader (reads with a `Result` return it, no read moves), so a derived decoder with lists or nested types next to
  plain fields returns the first error again, as before SW-34. 20 bytes on a request/reply decoder (RV32IC), four
  forms measured (`docs/code_size.md`, `shrink_wrap/tests/latched_errors.rs`).
- ✅ **SW-44 Fixed-layout fast path**: `#[derive_shrink_wrap]` sees when a type's encoding is a fixed little-endian
  byte layout (a `sized` or `final_structure` struct, a `sized` or `self_describing` enum with a `u8` / `u16` / `u32`
  `ww_repr`; fields only `u8`..`u64`, `i8`..`i64`, `f32`, `f64` and arrays of them) and generates an offset const
  per field, `ENCODED_LEN`, and `from_fixed` / `to_fixed` that load and store at those offsets on the slice, no
  `BufReader` / `BufWriter`, nothing generic. The `fixed_layout` directive makes it a requirement (and allows
  nested fixed-layout types): a `bool`, `Option`, list, string, varint or bit-packed field is a compile error naming
  the field. Same bytes as `ser_shrink_wrap` (`shrink_wrap/tests/fixed_layout.rs`, 2000 random values per type).
  `FixedLayout::write_c` prints the matching C reader (offset defines and `static inline` loads on a checked view,
  SW-79) from the same
  derive, `fixed_layout::write_c_header` a whole header: the first generated C of API-26. Code size on Cortex-M0
  for a header of eight integers: decode 484 → 276 bytes, encode 888 → 462 (`docs/code_size.md`). First user:
  chunky_mcu_updater's plan format (PROTO-3); it serves the busgen idea for types
  without dynamic fields. Open: arrays of nested types, a C writer, 16-bit `char` targets in the C reader.
- ✅ **SW-45 Bit fields and a variable last field in the fixed-layout fast path**: `bool`, `U1`..`U32`,
  `I2`..`I32` and `Nibble` get compile-time bit offsets (most significant bit first, a `Nibble` on a 4-bit boundary,
  a byte field after them starting a fresh byte, as `BufWriter` packs them) and are read and written with shifts and
  masks, in Rust and in the C reader (`sw_fixed_bits`, `sw_fixed_sbits`). `Option<T>`, `Result<T, E>` and enums of
  unequal variants are allowed as the last field (one rule for all variable-length fields); their flags are bit
  fields, also where `#[flag]` moves them. `MIN_LEN` and `fixed_len()` next to `ENCODED_LEN` (the longest, the
  buffer to have ready). A nested type cannot share a byte with bits of its parent: a compile error says so. Every
  refusal says why and what to use instead. Byte-equal to `ser_shrink_wrap` on mixed bit runs, a run at the end,
  options, results, relocated flags and enum variants with both (`shrink_wrap/tests/fixed_layout.rs`); the
  generated C is built and run with every warning an error (`just lint-c`). A bit-packed sample on Cortex-M0:
  decode 652 → 276 bytes, encode 1034 → 560.
- ✅ **SW-46 Fixed head, sequential tail (`head_tail`)**: an opt-in wire format of its own for types with lists,
  strings and options anywhere. The head holds every fixed-width field at a constant offset (numbers, arrays, bit
  fields as in SW-45) plus a `u16` count per list or string, the flag bit of each `Option` / `Result` and the head
  of each nested type; the tail holds the variable parts in field order. An enum is its tag, then its variant's
  head and tail. Generated by `#[derive_shrink_wrap(head_tail)]`: offset consts, `HEAD_LEN`, `from_head_tail` /
  `to_head_tail` / `head_tail_len`, the `HeadTail` trait (also implemented for every fixed-layout type) and a C reader
  (checked views since SW-79, `shrink_wrap::head_tail::write_c_header`), built and run by
  `just lint-c`. Non-generic, size first: no `BufReader` / `BufWriter`, no length FIFO, no reversed lengths; the
  tail is a cursor (`head_tail::Tail`) that takes lengths it already knows. Lists are `head_tail::List<'i, T>` (items
  numbers or fixed-layout types of one length, read when iterated). No sw serdes and no owned version: one type,
  one format. Tests: exact bytes, round trips, every truncation, 200 000 random buffers, UTF-8, counts over u16
  (`shrink_wrap/tests/head_tail.rs`). busgen's request and reply enums (size bench `busgen_ht`): 4626 bytes on
  Cortex-M0 and 6004 on RV32IC, against 4408 / 5762 hand-written and 6652 / 8210 derived.
  Open: `head_tail` types as ww API arguments (they have no sw serdes), lists of variable-length items, a C writer,
  `u8` counts for short lists (`MaxLen`).
- ✅ **SW-79 Checked C views of fixed-layout and `head_tail` types**: the generated C readers no longer take a bare
  pointer. `<name>_view(buf, len, &v)` checks that the `len` bytes hold a whole value (every flag, tag, nested value
  and list count followed, an unknown enum tag refused) and makes a view, `<name>_view_t`; every reader takes a view,
  so there is no way to read a field without the length check. A view's `len` is the value's length (the next value
  of a stream starts there). Readers stay inside the view: an index past an array or list reads 0, an `Option` /
  `Result` payload, an enum variant (`<enum>_as_<variant>`) and a list item of a fixed-layout type are `int` readers
  with an out parameter that return -1 when it is not there. A caller that ignores a -1 reads zeros, never
  through `NULL`: every reader of an empty view reads 0 and a failed `int` reader leaves an empty view in `*out`.
  A type over 65535 bytes is an `#error` where `size_t` is 16 bits. Views and `int` readers are `SW_NODISCARD`
  (`warn_unused_result` on gcc and clang). No allocation; list lengths are checked before `count * size` can wrap
  on a 16-bit `size_t`. Same acceptance and lengths as Rust's `from_fixed` / `head_tail::read` (a C string is
  bytes: no UTF-8 check), checked on valid, truncated, empty, NULL, changed and random buffers of every test type
  with AddressSanitizer and UBSan and buffers of exactly the case's length (`c_views_check_bounds` in
  `shrink_wrap/tests/fixed_layout.rs` and `head_tail.rs`), and in the conformance matrix (a view from exactly the
  golden bytes, refused on every shorter prefix). C99 with `-pedantic -Wconversion -Wsign-conversion -Werror` on
  the host, MSP430, Cortex-M0 and RV32. Cost on Cortex-M0 at `-Os`: 4 bytes per reading function over the old
  readers with a hand-written length check, and about 6 more for the empty-view check (clang `-Os`, thumbv6m: 9
  fixed-layout readers 148 → 204 bytes, 5 `head_tail` readers +24 bytes and an 8-byte empty view in `.rodata` per
  `_as_<variant>` used). Raised on chunky_mcu_updater PR #6.
- ✅ **SW-52 `head_tail` views and variant writers**: `Type::view(buf)` gives a `TypeView<'_>` without making the
  value: the lengths checked and the tail walked once, the head's fields read by accessors at their offsets when
  called (the same code as `from_head_tail`, one field at a time), lists, strings, options and nested views kept
  from the walk; an enum's view is an enum of its variants' views (a tuple variant of one nested type holds its
  view). Every enum variant gets `Enum::write_<variant>(buf, fields..)`: the bytes of `to_head_tail` for that
  variant without making the value (the same `write_ht` code). `HeadTail::View` / `view_ht`, `head_tail::view`.
  Tests: values back from every accessor, writers byte-equal to `to_head_tail`, every truncation, 200 000 random
  buffers agreeing with `from_head_tail`. Size bench `busgen_view`: 4988 bytes on RV32IC and 3894 on Cortex-M0, 13 %
  and 12 % below the hand-written codec; busgen's firmware 5608 / 5090 bytes (UART / I2C image, fpga_tools BUS-12).
  View types are always `pub` (the `View` of a public trait). Open: lazy views of nested fixed-layout types (read
  when the view is made), a writer for structs.
- ✅ **SW-53 Numbers through `fixed_layout::Le`, out of line on RISC-V**: generated fixed-layout and `head_tail` code
  reads and writes multi-byte numbers with `Le::ld` / `Le::st` on the bytes at the field (`fixed_layout::at`), one
  function per type on RISC-V (no unaligned loads), inline elsewhere (on Cortex-M0 the byte loads are smaller than
  the calls, measured). Under LTO `ld` passes its pointer through `black_box`: argument promotion would otherwise
  move the load into every caller. Bit-field bytes cleared once per head. `busgen_ht` on RV32IC 6004 → 5658; a type
  read or written in one place grows (`fixed_ser` 498 → 612, docs/code_size.md).
- ✅ **SW-54 `#[cfg]` on `head_tail` enum variants**: passed to everything generated for the variant (consts, arms,
  view, writer, C text); a variant compiled out keeps its tag and reads as an unknown one. busgen's firmware builds
  only its bitstream family's requests with it.
- ✅ **SW-39 Byte-wise `memcpy`, measured, advice for very small targets**: a firmware's own `memcpy` (and
  `__aeabi_memcpy` on Arm) of 22 bytes instead of the word-wise one of `compiler_builtins`: 324 bytes less on
  RV32IC, 304 on Cortex-M0, for every image that moves a value, at the price of speed and of replacing `memcpy` for
  the whole firmware. Code and downsides in `docs/code_size.md` ("Build settings"), measured with
  `just size-bench --fw-features byte_memcpy`. Nothing in `shrink_wrap` depends on it.
- ✅ **SW-40 Bit reads stay out of the lengths in the last byte**: `read_un8`..`read_un64` (and `U1`..`U64`,
  `I2`..`I64`) no longer read into the length nibble the last byte shares with the forward data, and a length is
  no longer read from bits the forward side has read; both are `OutOfBounds..` errors. Malformed input only, every
  message the writers produce decodes as before. Checked against a bit-level model in every reader state of 1 to
  3 byte buffers (`buf_reader.rs` `bounds_every_state_every_read`) and by round trips of random messages
  (`shrink_wrap/tests/rev_nibble_bounds.rs`).
- ✅ **SW-81 An empty byte list writes no alignment in the borrowed derive**: a `RefVec<u8>` field of a borrowed
  type (`ser_shrink_wrap_vec_u8`) wrote a padding byte for an empty list that started mid-byte, where every other
  writer and every reader take nothing; a field after it that starts mid-byte was read one byte early (a `ww_tape`
  `TrackDef` with an empty `bundle` failed with `OutOfBoundsReadUN`, a bool after the list read back wrong). The
  borrowed derive now writes the owned derive's bytes; bytes written before the fix still read where they did.
  Tests `shrink_wrap/tests/empty_byte_list.rs` (a property test: borrowed and owned give the same bytes), conformance
  cases `struct.bytes.empty_end`, `struct.bytes.empty_mid`, `struct.bytes.mid`.
- ✅ **SW-35 Exact `ElementSize` of enums**: a `sized` / `self_describing` enum reports the discriminant plus one
  variant (`ElementSize::one_of`) instead of the sum of all variants without the discriminant; `sized` with variants
  of different sizes is a compile error; user types inside arrays, tuples, ranges and options are counted. Same wire
  format (`shrink_wrap/tests/enum_size.rs`, `docs/size_assumptions.md`). Unblocks reading `RefVec<Sized T>` in place.
- ✅ **SW-33 `UVlq64`**: byte-based variable length `u64`, same big endian VLQ encoding as `UVlq32`, 1 to 10 bytes,
  `uvlq64` field type in `#[derive_shrink_wrap(..)]`, `BufReader::read_uvlq64`/`BufWriter`/`BufWriterOwned::write_uvlq64`
  (`shrink_wrap/src/vlq64.rs`, `docs/types.md`, `docs/showcase.md`). No `UVlq64Backfill` (not asked for).
- ✅ **SW-82 `IVlq32` and `IVlq64`: signed byte-based varints, zigzag over `UVlq32` and `UVlq64`** — 1 to 5 and 1 to
  10 bytes, `-64..=63` in one byte; `IVlq64` is the variable-length `i64` that `INib32` cannot be. Field types and
  `#[wire(..)]` types, `read_ivlq32`/`read_ivlq64` and `write_ivlq32`/`write_ivlq64` on the reader and both writers;
  a malformed read is the unsigned type's error. In a `Series` they are plain signed 32- and 64-bit columns
  (`shrink_wrap/src/ivlq.rs`, `docs/showcase.md`). Open in WireWeaver: the dynamic client, Python, the C runtime
  and the conformance corpus do not know the two types yet.
- ✅ **SW-83 `len32`: `BufWriter` past 65 535 bytes** — a cargo feature that keeps a waiting length as `u32` instead
  of `u16`: a list, string or nested value is no longer limited to 65 535 bytes (`Error::LenTooLong`), and each
  waiting length takes 4 bytes of the buffer instead of 2, which `MAX_SIZE` counts. Off by default, so a small
  buffer holds what it held; the wire bytes are the same. Flash, on the five size-bench targets at `opt-level = "z"`,
  against the writer before it: -74 to +28 bytes with `len32` and -24 to +24 without, on the cases with lists
  (`vec_ser`, `busgen_ser`, `busgen`), nothing elsewhere (`shrink_wrap/tests/len32.rs`, `docs/wire_format.md`).
- ✅ **SW-84 `UN` and `IN` outside a `Series`** — the plain form of the numbers whose bit count is data: the variant
  in 3 bits, the bit count in 3, 4, 5 or 6 bits, then that many value bits, `IN` in two's complement. Both writers
  and the reader; before, the six impls panicked (`todo!()`). A value wider than its bit count is
  `Error::SubtypeOutOfRange`, a variant above 3 `Error::EnumFutureVersionOrMalformedData`. No size-bench case uses
  the types and no reader or writer function changed (`shrink_wrap/src/un.rs`, `docs/wire_format.md`).
- ✅ **SW-19 Bounded lists and strings `MaxLen<T, N>`**: a `RefVec`, `Vec`, `&str` or `String` with at most `N`
  elements, same bytes as `T` (a bound can be added, raised or removed without a new wire form), checked on
  serialize and deserialize (`Error::MaxLenExceeded`, never truncated). Works in derived types and in every API
  position (arguments, returns, properties, stream and sink items), visible in introspection
  (`ww_self::Type::MaxLen`), honoured by the dynamic client and Python; raising a bound is compatible, lowering or
  adding one breaking (`ww api check`). `SerializeShrinkWrap::MAX_SIZE: Option<MaxSize>`, the worst-case size of a
  type whose every part is bounded, feeds API-18. `docs/types.md`, `docs/evolution/rules.md`,
  `shrink_wrap/tests/max_len.rs`, `tests/bounded`, `examples_mcu/mcu_qemu` (`qemu-bounded`).
- ✅ **SW-15 Handle end of buffer when reading evolved fields**: a field with `#[default = ..]` reads as its default
  only when it starts at the end of the data and runs out of it (`Error::is_read_eob`); a truncated or malformed
  value is an error. The flag of an evolved `Option` / `Result` falls back with it, `#[default]` works on any type
  and its value is used (an `Option` was always `None`). Same in the `ww_self` dynamic decoder. Padding bits of the
  old data's last byte read as 0, so bit-sized fields default to their zero value. Decided: no implicit defaults
  for `Vec`/`String`/`Option` fields without `#[default]`, the attribute is what marks a field as added later
  (and what `ww api check` looks for). `docs/derive.md#evolving-types`, `shrink_wrap/tests/evolution.rs`.
- ✅ **SW-16 `FutureVersion` enum variant**: an enum ending with `#[future_version] Unknown(u32)` reads an unknown
  discriminant into it instead of failing with `EnumFutureVersionOrMalformedData`, and writes it back; an `Unsized`
  enum skips the newer variant's fields with its size slot, `sized` / `self_describing` ones only survive new unit
  variants. No wire number of its own, no change to `ELEMENT_SIZE`, left out of the API model. The `ww_self`
  dynamic decoder reads an unknown variant of an `Unsized` enum as `<unknown N>`. Added by default since SW-47.
  `docs/derive.md#enum-variants-from-a-newer-version`, `shrink_wrap/tests/evolution.rs`.
- ✅ **SW-48 Bit-sized fields and variant payloads**: `U1`..`U64` / `I2`..`I64` (`u7`, `ub10`, `i2`.. in the API
  model) as struct fields, enum variant payloads and `#[ww_trait]` method arguments, returns and properties, packed
  bit after bit with a sub-byte discriminant: `enum I2cAddress { Seven(U7), Ten(U10) }` with `ww_repr = u1,
  self_describing` takes 8 or 11 bits instead of 16 or 24 with `u8` / `u16`. Server and client codegen (it panicked
  on such a method argument), `I16` (was missing), `From` / `TryFrom` to and from the backing integer. The dynamic
  client and Python already handled them, now tested byte for byte against derived types; the C runtime has the
  bit reads. Same wire format (`shrink_wrap/tests/sub_byte.rs`, `tests/dynamic`, `tests/methods`,
  `docs/showcase.md#bit-sized-fields-and-variant-payloads-a-7-or-10-bit-i2c-address`).
- ✅ **SW-47 `#[future_version]` variant by default**: the derive adds `Unknown(u32)` to every `Unsized` enum with a
  free discriminant; `no_future_version` opts out (framework enums whose readers must fail: `ww_self` model,
  `MultiIndex`), `future_version = Name` renames it, `final_structure` / `self_describing` / `sized` enums only get it
  when written. A Rust API change of every such enum (exhaustive matches need the arm), not of the wire or the API
  model. `docs/derive.md#enum-variants-from-a-newer-version`, `shrink_wrap/tests/evolution.rs`.
- 📋 **SW-77 Store columns across a prefix change** — `write_store_series` (SW-64) writes nothing when an enum or
  `Option` tag changes in the rows (a ww_si prefix that autoranges, a number type), and the batch is rebuilt item by
  item and walked: 5x the instructions of a store column on a Cortex-M3 at `opt-level = "z"` (16.7 k against 3.3 k
  per column of 32), 4.5x at 3. The wire already has the runs and the restarts; the store writer would write the
  tag runs from the store's arrays and the numeric column in segments, restarting at each run boundary, the same
  bytes as the walk. Measured by `examples_mcu/mcu_qemu`'s `qemu-dyn-prefix-auto` and `stm32h725ig-dyn-prefix-cycles`
  (`docs/series.md#a-prefix-per-item`); the round trip and conformance case are in
  (`stdlib.si.timed.series.prefix_change`).
- 💡 **SW-78 A cheaper one-tag check for store columns** — `SeriesStore::columnar` scans every row of a ww_si
  quantity's prefix and number-type arrays before a store column is written: 2 x ~200 instructions per column of
  32 on a Cortex-M3 at `opt-level = "z"`, about 60 % of what a `Quantity` column costs over a `Fixed` one from the
  store (`docs/series.md#a-prefix-per-item`). Comparing the arrays a word at a time, or a flag the store
  keeps as rows are set, would take most of it.
- 📋 **SW-17 Const / magic type** · `v0.5` — a simple type with a generic parameter that serializes a constant value or
  byte string and checks it on deserialization, failing on a mismatch. For example the `magic` field in
  `ww_self` (a plain `u32` today).
- ✅ **SW-18 Size-of-enclosing-type field** `TailSize<N>`: a field the derive macro fills in with the size of the rest
  of the value once it is serialized (a `UVlq32` right-justified in `N` bytes, 1 to 5), in any position, nested types
  each with their own. Readers bound the value by it (a longer buffer stops at the right byte, a truncated one
  errors, newer trailing fields are skipped) or skip it without parsing. Fields before it must be `Sized` or
  `SelfDescribing`, checked at compile time. `docs/special_types.md`, showcase.
- ✅ **SW-60 `Series<T>`: columnar compressed sequences of any derive type** — a list of `T` (structs, enums with
  data, `Option`, nested) written column by column, one column per leaf field: numbers as a numeric column whose
  predictor (raw, delta, delta of delta) and coder (SW-31 prefix code, bit-packed blocks of 8, constant, plain) the
  writer picks per column, floats plain or SW-32 XOR, enum tags, bools, `Option` presence and strings as runs (a value
  and one entry per change); a column restarts where an earlier enum field (a ww_si prefix) changes. 3 to 8 times
  smaller than a plain `Vec<T>` of ADC samples, self-contained per batch: the column codecs (numeric, 128-bit,
  float, runs, codec choice by counting bits or a fixed `CodecPolicy`, no alloc, malformed input an error), the
  `Series<'i, T>` / `SeriesOwned<T>` types and the `SeriesColumns` trait with impls for the primitives, `Option`,
  tuples, arrays, strings and bytes (one-pass validating skim, parallel item iteration, evolvable structs both ways,
  the design's examples and size tables as golden bytes), fuzz targets `shrink-wrap-series-columns` and
  `shrink-wrap-series-type`; the opt-in `series` directive of `#[derive_shrink_wrap]` for structs (also generic,
  DER-14) and enums, `#[series(opaque)]` fields, compile errors naming the field for what v1 doesn't take, on
  `ww_si` quantities and `ww_numeric::NumericValue` (`shrink_wrap/tests/series_derive.rs`,
  `ww_stdlib/ww_si/tests/series.rs`); docs (`docs/series.md`, the layout in
  `docs/wire_format.md#seriest`, worked bytes in `docs/showcase.md#compressed-columns-seriest`).
  `Series<T>` is usable today in any property, method or stream item; a dynamic (non-generated) decoder and
  batching a stream or property into `Series<Timed<T>>` automatically build on it as their own items (CLT-29,
  API-41), not blocking this one.
- 📋 **SW-42 `MAX_SIZE` on owned types** · `v0.5.x` — `SerializeShrinkWrapOwned` has no worst-case size yet, only
  the borrowed trait has (SW-19); host code sizing a buffer for an owned value uses the borrowed type's.
- 📋 **SW-20 Sub-type numbers** · `v0.5.x` — numbers restricted to a range, like `10..=25`, or a set with gaps, like
  `0..=8, 12, 16`; checked on serialization and deserialization, possibly using fewer bits on the wire
  (`docs/types.md`, "Subtypes").
- 📋 **SW-21 Multi-dimensional arrays** (2D, 3D).
- ✅ **SW-70 Hot bit paths: no forced inlining at `opt-level = "z"`** · `v0.5` — flash diet. shrink_wrap's
  `#[inline(always)]` sorted by measurement: the one-line wrappers (integer reads and writes, `_latch` methods, trait
  defaults) stay forced, inlining them is free in size; so do the `Series` store and observe-cell paths (as hints a
  FOC row capture went 266 -> 1456 instructions). The private bit helpers of the writer and reader are `#[inline]`
  hints; a firmware picks with shrink_wrap's `fast-io` feature (forced, as before; on with `std`) or
  `optimize-speed` (Rust 1.101 or nightly, `#[optimize(speed)]` on the bit primitives and `Series` loops), first a `--cfg`. Lists of plain numbers share one non-generic
  length-and-bytes path. Measured (thumbv6m): TEST-30 floor −740 B at `fw-z`, ±0 at `fw-3`; Donguru −824 B at `z`,
  −892 B at `3`; task-side series and bit-field encoding 5-22 % slower at `z` by default, ±0 with `fast-io`, 1-3 %
  faster with `optimize-speed` (+208 B). The planned 2.5-3.5 KB of "duplicate copies" were mostly call sequences:
  `#[inline(never)]` on every derived serdes made Donguru 1.9 KB larger. See
  `docs/code_size.md#hot-paths-at-opt-level-z`.
- ✅ **SW-74 Reader and writer plumbing out of the serdes copies** · `v0.5` — flash diet. The 31.7 KB of
  `BufReader`/`BufWriter` code inlined into Donguru's serdes had four big shapes, found with `fw_budget --attribute
  --sites` (TEST-32): `latched()`'s `take()` (239 copies, 1.1 KB), the `Result<BufReader>` copied out after
  `split()?` in `read::<T>` (45 copies, 1.1 KB), `BufReader::new` / `BufWriter::new` (65 + 41 copies, 2.2 KB) and the
  `Result<UnsizedBuilder>` in `write::<T>` (67 copies, 0.7 KB). Fixed in the shape of the code: `latched()` stores
  only an error it clears; one out-of-line `split_sized` writes the sub-reader in place; `begin_unsized` /
  `end_unsized` keep the builder in registers; the constructors are `#[inline(never)]` unless the `fast-io`
  feature (SW-70's switch, a `--cfg shrink_wrap_hot` before; `std` turns it on). An out-of-line `latched()`
  was larger on both cores and was left inline. Measured: Donguru −3,984 B at `z`, −18,176 B at `3`, −2,708 B with `fast-io`; rePort −6,360 B; the
  TEST-30 floor −76 to −336 B; instructions at `z` within ±0.6 %, at `3` the bit-io micro loops on a local reader or
  writer up to 2.2x (restored with `fast-io`), dispatch ±1.3 % except a call with an argument (+4.8 %). No wire
  change. `docs/articles/flash_budget.md#sw-74-reader-and-writer-plumbing`.
- 🚧 **SW-22 Fuzz `shrink_wrap`** — `fuzz/` has several `shrink_wrap` targets: `shrink-wrap-tail-size` (`TailSize`
  types round-tripped, random bytes into their readers), `shrink-wrap-latched` (SW-38's one-error-check-per-type
  reads against `read_field`), `shrink-wrap-ref-vec` (SW-37) came with its feature, `shrink-wrap-option-any-vec`
  with SW-55 and DER-13 (sparse structs), `shrink-wrap-series-columns` and `shrink-wrap-series-type` with SW-60,
  `shrink-wrap-nesting` with the nesting depth limit (recursive types at random limits). Round-trip every built-in
  type and the derive's features.
- 🔍 **SW-23 Tuple enum variants evolution** — check that tuple variants evolve as `docs/evolution/rules.md` says
  (fields can be appended) and add a test.
- 💡 **SW-65 Aligned plain `Series` columns** — numbers byte-aligned and little-endian at their width (numeric
  escape `11 11`, float codec `0010`, an `AlignedPlain` policy), copied from a `SeriesStore`: 34 k cycles per FOC
  level batch on an STM32H725 at `opt-level = 3` against 60 k for column-wise `DeltaPacked` (SW-64), for 2.3x the
  bytes. Built and measured on branch `s/18oct2026-memcpy-fast-sw65`, not landed; may come back after observe cells
  and a `u32` payload in `NumericValueStore`.
- ✅ **SW-24 `INib32`: a signed nibble varint, zigzag over `UNib32`** — small negative values stay as short as
  small positive ones (`INib32(-4)` and `INib32(4)` both 1 nibble). A field type and a `#[wire(INib32)]` field
  attribute (reuses `Error::MalformedUNib32`: on the wire it *is* a `UNib32`); in `Series` columns and the dynamic
  (`ww_self`) value/`Series`/`expr` paths it is a plain signed 32-bit number, not re-zigzagged there (the column
  codec's own signed delta/zigzag already compacts it). `wire_weaver_py` converts it like `i32`
  (`int`/`"int32"` numpy dtype, not `UNib32`'s unsigned group); `wire_weaver_evolution` gives it `UNib32`'s size
  class. Not wired into `ww_repr`/enum discriminants (a field type, not a discriminant type) — a genuinely new ask.
  Conformance cases (`docs/conformance.md`) pass the Rust derive, the dynamic client, Python and `explain`;
  the C runtime has no `Op::INib32` of its own yet, only `UNib32` (gap-marked, SW-75).
  `NumericBaseType`/`NumericValue::INib32` is breaking (neither enum has a catch-all for an unknown variant):
  `ww_numeric` 0.3.0, wire line w2.0, `#[since = "w2.0"]`; dependents re-saved, none bump their own version.
- 💡 **SW-25 In-place builder for server handlers** · `v0.5.x` — a new step-by-step / partial builder (e.g. for a
  `RefVec`) might be useful with the new `Context` in server handlers, to write replies and events straight
  into the buffer. It might not work out.
- 💡 **SW-26 Map support** — `HashMap`/`BTreeMap`, probably as a `Vec` of `(K, V)`.
- 💡 **SW-27 In-place editing** of serialized data.
- 💡 **SW-28 Compression wrappers** — the time-series part is done as `Series<T>` (SW-60, with the SW-31 delta and
  SW-32 XOR codes); in-place dictionary compression as `Compression<T>` (strings?) stays an idea.
- 💡 **SW-29 Flatten support** — apply a type's layout across struct/enum boundaries.
- 💡 **SW-30 `derive_shrink_wrap` on a whole module at once**. For definitions in separate files with own syntax, see
  [standalone definition files](https://github.com/vhrdtech/wire_weaver/blob/master/FEATURES.md).
- 💡 **SW-56 Tri-state slots in `OptionAnyVec`** — absent / value / error per slot, e.g. a property update that says
  why one value is missing. The header has room for it: mode nibbles `4..=15` are reserved (a reader fails on them
  today), so 2-valued vecs don't pay for it.
- 🧪 **SW-69 No core::fmt Debug of shrink_wrap types on a device** · `v0.5` — flash diet: the variant names of
  `shrink_wrap::Error` (`OutOfBoundsReadU8`, ..., about 450 B) and the `core::fmt` machinery behind them used to
  land in flash because the type's `#[derive(Debug)]` always carried them. `shrink_wrap::Error` keeps implementing
  `Debug` (`std::error::Error` needs it, `unwrap()` / `expect()` on `no_std` must still compile), but by hand now:
  the variant names only with `std`, a numeric code without (`shrink_wrap::Error(17)`); `defmt::Format` (feature
  `defmt`) still has the names, as `.defmt` symbols never loaded into flash. `Display` (which formats through
  `Debug`) follows. Found two on-target callers that reached for `core::fmt::Debug` because there was no
  `defmt::Format` alternative, fixed instead of worked around: the `examples_mcu/nucleo_{g0b1re,h743zi2}`
  `settings.rs` bins no longer wrap `ww_settings::Error` / `LoadReport` in `defmt::Debug2Format` (both already
  derive `defmt::Format`), and `ww_stream::store::RamRingError` now derives `defmt::Format` behind a new `defmt`
  feature, so `examples_mcu/rp2`'s `usb_log.rs` logs its log-store error without `Debug2Format` either. Same wire
  format; only the printed text of a `no_std` build without `defmt` changes. Not yet measured: TEST-30 checks that
  the variant names are not linked; est. 0.5-1 KB.
- ✅ **SW-71 Unit variants of a mixed enum decode apart** · `v0.5` — flash diet: des `exact_units::Prefix` was 574 B
  (5 copies in Donguru). The cost is the mix: a `match` with field-reading arms keeps LLVM's jump table and one arm
  per tag. A fully fieldless enum already folds to about 48 B, so it needs nothing. The derive now puts the longest
  run of consecutive unit tags behind a range check, and those arms fold to a tag -> variant map, without any table of
  our own. It does so for a run of at least 4 with at least 2 data arms (1 with `niche`). `Prefix`, decoded alone on
  M0+: 624 -> 324 B at `fw-z`, 576 -> 460 B at `fw-3`. The ~100 B first estimated left out its three data arms and the
  catch-all, which stay. Same bytes on the wire. Test `shrink_wrap/tests/enum_unit_run.rs`.
- ✅ **SW-72 `MaxLen<_, N>` checks through one helper** · `v0.5` — flash diet: ser `MaxLen<_, N>` was monomorphised
  per bound (9 instances, 296 B in Donguru: `&str` for N = 16, 32, 64, 128, `RefVec<u8>` for 16, 20, 32, 512) though
  the bodies differ only in N. `new`, `ser_shrink_wrap`, `ser_shrink_wrap_owned` and `des_shrink_wrap`'s `check_len`
  now share one `check_max_len(len, max)` (`shrink_wrap/src/max_len.rs`); same error and bytes on the
  wire. Measured on TEST-30's `flash-budget` harness (added `MaxLenBounds8`/`16` to `tests/flash_budget_api`,
  the API had no `MaxLen` before): `#[inline(never)]`, as the item asked for, does **not** save the estimated
  200-250 B — it costs 10 to 53 B more per bounded resource on thumbv6m, `fw-z`/`fw-3`, because a different `N`
  still has to reach the call as an argument, so nothing byte-identical is left to merge, and the call itself is
  overhead a plain inlined `if len > max` didn't pay. Left as a plain `fn` (no `#[inline(never)]`): the compiler
  folds it back in at every site, so the result is one copy of the logic in source and the same bytes as before
  in the binary, not fewer. `docs/articles/flash_budget.md` has the numbers.
- 💡 **SW-73 Panic-free indexing in BufReader / BufWriter** — flash diet, to evaluate: the slice indexing and
  `copy_from_slice` in shrink_wrap's reader and writer keep panic paths (670 B of file paths, `copy_from_slice` 636 B
  and `index`/`index_mut` about 2 KB in Donguru's isolated build) though the reader and writer already report
  `OutOfBounds` errors. `get()` / checked copies where the bound isn't proved; measure instructions too. Est. 1-2 KB.

## Derive macro (`shrink_wrap_derive`, `DER`)

- ✅ **DER-1 `#[derive_shrink_wrap(..)]` attribute** with clear syntax, no hidden magic and good error messages
  (`docs/derive.md`); replaced `#[derive(ShrinkWrap)]`.
- ✅ **DER-2 Discriminant checks**: discriminants must fit `ww_repr`, implicit ones are numbered like in Rust.
- ✅ **DER-3 `#[default = ..]` for evolved fields** and `#[flag]` relocation.
- ✅ **DER-4 `TailBytes` must be last**: rejected anywhere but as the last field of a struct or enum variant, and nested
  in `Option`, `Vec`, tuples or arrays (`check_tail_bytes_position`), like `UVlq32Backfill` must be first.
- ✅ **DER-10 Generic arguments on user field types**: `Foo<u8>`, `Wrapper<'i, T>` are emitted as written,
  `Wrapper<'i, T>` maps to `WrapperOwned<T>` in the owned variant (`docs/derive.md`, type mapping; test
  `shrink_wrap/tests/generic_external.rs`).
- ✅ **DER-14 Generics on derived types**: structs and enums with type parameters (`struct Timed<T> { t: Second,
  value: T }`, `enum Reading<'i, T>`), const parameters of any type (`struct Buf<const N: usize> { data: [u8; N] }`,
  `MaxLen<&'i str, N>`, `const STRICT: bool`), defaults and where clauses, also with a lifetime (`Tagged<'i, T>` /
  `TaggedOwned<T>`); each impl bounds the type parameters by its trait (serdes, owned serdes, `SeriesColumns`) on top
  of the bounds and where clause as written, the size of `T` and `N` is part of the type's. `series`, `display`,
  `validate` and `discriminants` take generics, `fixed_layout`, `head_tail` and `sparse` reject them
  (`docs/derive_reference.md#generics`; tests `shrink_wrap/tests/generic_items.rs`, `series_derive.rs`;
  `ww_si::Timed<T>`). In APIs and the API model: API-43.
- ✅ **DER-15 `PhantomData` fields**: `PhantomData<T>` is zero bits (`Sized { size_bits: 0 }`, like `()`) borrowed
  and owned, and a `SeriesColumns` with no columns, for any `T`; the derive leaves a type parameter used only inside
  `PhantomData<..>` without the serdes / `SeriesColumns` bounds of DER-14, so `struct Quantity<U> { value: i32,
  _unit: PhantomData<U> }` works with a bare unit marker and writes the same bytes as without the field
  (`docs/derive_reference.md#generics`; test `shrink_wrap/tests/phantom_data.rs`). For `ww_si` units.
- ✅ **DER-17 `#[default = ..]` fields default in serde too**: a type deriving `serde::Deserialize` (through
  `derive(..)` or a `cfg_attr`) gets `#[serde(default = ..)]` on each `#[default = ..]` field of a struct or struct
  variant, so RON or JSON written before the field was added still reads; for saved `ww_self` API snapshots when a
  later model field isn't an `Option` (EVO-13) (`docs/derive.md#evolving-types`; test
  `shrink_wrap/tests/serde_default.rs`).
- ✅ **DER-19 `#[wire(W)]` field attribute**: a field keeps a plain Rust integer (`i8`, `u16`, `u64`, ..) and is
  written as a narrower or variable-length wire type (`#[wire(I4)] i8`, `#[wire(UNib32)] u64`, `#[wire(U12)] u16`):
  checked on write (`Error::SubtypeOutOfRange`, never a cut value), widened on read, same bytes as a field of type W
  (runtime trait `shrink_wrap::WireAs`). Struct and enum fields, `#[default = ..]`, `sized`/`final_structure`,
  `#[cfg_attr(pred, wire(W))]`, `series` columns (`series::Wired<W, T>`, a W column). The API model records W (the
  walker reads it, also inside `cfg_attr`), so the dynamic client, Python and C need nothing new
  (`docs/derive_reference.md#wirew-field-attribute`; tests `shrink_wrap/tests/wire_as.rs`, conformance
  `wire.*`). For `ww_si` exact units (design notes in internal tracking).
- 💡 **DER-21 `#[wire(W)]` beyond plain fields** — inside `Option`, arrays and lists (`Option<#[wire] u8>` has no
  syntax yet: a wrapper type or `#[wire(Option<U4>)]`), and in `fixed_layout` / `head_tail` types, which reject
  the attribute with a compile error naming DER-21 today.
- ✅ **DER-18 `transparent` derive directive**: a struct with one field that isn't zero bits (`PhantomData`, `()`)
  takes that field's size class (`ELEMENT_SIZE`, `PUSHES_LEN`, `MAX_SIZE`), series columns, store and cells, and
  writes exactly its bytes in every position, borrowed and owned, generic, with `validate` and `display`; compile
  errors name the type for zero or two real fields and for the directives it excludes. Recorded in the API model
  (`ItemStructExtra::transparent`, read by the walker, `ValueOwned`'s decoder and `Series` columns, so the `ww` CLI and
  `wire_weaver_py` read it too). For `ww_si::Fixed` (STD-55), where a `final_structure` wrapper was
  `UnsizedFinalStructure` even around a `u64` and couldn't sit in a `sized` parent. Also fixed: a generic parameter
  named `P`, `I`, `S` or `C` clashing with the derive's own in series code (`PhantomData<(U, P)>` included), and
  `#[repr(..)]` of a derived struct being dropped (`docs/derive_reference.md#transparent`; tests
  `shrink_wrap/tests/transparent.rs`, `phantom_data.rs`, conformance cases `transparent.*`). The evolution
  checker compares wrappers by meaning: EVO-15.
- ✅ **DER-20 Unit-typed fields cost what plain fields cost**: a `transparent` wrapper (DER-18, e.g.
  `exact_units::Fixed`) inside a derived struct is written and read on exactly the path of its inner plain field:
  the latched `write_*_latch` / `des_shrink_wrap_latch` calls with one error check per type (SW-34, SW-38), not the
  wrapper's own `ser_shrink_wrap` with a checked write per field. Hidden `ser_shrink_wrap_latch` (borrowed and owned)
  with `BufWriter::write_field`, used by the derive for every user-type field; a wrapper with `validate` (`Fixed`'s
  bounds) keeps a value out of range as the latched error (`BufReader::refuse_latch`, the first refusal wins). TEST-27 had measured +2 to 15 instructions and +4 to 42 bytes per struct encode/decode on M0, M4F and
  M33 before it; same wire format (`docs/derive_reference.md#transparent`,
  `docs/code_size.md#transparent-fields`; test `shrink_wrap/tests/transparent.rs`
  `fields_are_latched_like_the_plain_field`, fuzz `shrink_wrap_latched`).
- ✅ **DER-11 Field type from a macro_rules `$T:ty` fragment**: accepted like the type written directly, instead of
  rejected as an unsupported `syn::Type::Group` (test `shrink_wrap/tests/macro_ty_fragment.rs`).
- ✅ **DER-12 `Display` from doc comments**: a `display` directive writes `core::fmt::Display` and
  `core::error::Error` (borrowed and owned) from the first doc line of each variant/struct, `{field}` filled in,
  `#[display("..")]` to override; a variant with neither is a compile error. Error enums need no hand-written
  `Display` on hosts (`docs/derive_reference.md`; test `shrink_wrap/tests/display.rs`).
- ✅ **DER-13 Sparse structs**: `#[derive_shrink_wrap(sparse, ..)]` on a struct of `Option<T>` fields (borrowed and
  owned) encodes it as an `OptionAnyVec` (SW-55), one slot per field in declaration order or at `#[slot = N]`
  (increasing, gaps allowed, a present value in a gap is `Error::OptionAnyVecUnknownSlot`); a nested sparse struct
  is one slot. Fields are only appended. A non-`Option` field or a combination with `final_structure` / `sized` /
  `self_describing` / `fixed_layout` / `head_tail` is a compile error. For the per trait level `<Level>Update` of
  SRV-18 (`docs/layouts.md`, `docs/evolution/rules.md`; test `shrink_wrap/tests/sparse.rs`, fuzz
  `shrink-wrap-option-any-vec`). `ww_self` carries the flag (`ItemStruct::sparse`, `Field::slot`, TOOL-11), so a
  dynamic decoder (`ValueOwned`, the `ww` CLI, `wire_weaver_py`) reads a sparse struct correctly.
- 📋 **DER-5 `#[since = "w1.2"]` on fields** — generate correct evolution code from it (the derive keeps the text,
  `wire_weaver_core` reads it as a wire revision, EVO-24).
- 📋 **DER-6 Check that `TailBytes` ends the outer type too** — a struct ending in `TailBytes` used as a non-last field
  of an outer `sized` / `final_structure` type ("last in the first `Unsized` type on the way up") is not visible to
  the derive; needs a marker on the type, e.g. an associated const, and a compile-time assertion in the outer type.
- 📋 **DER-7 Prefix enum discriminants** · `v0.5.x`.
- 💡 **DER-16 Where clauses naming the borrowed lifetime, for owned types** — a generic type bounded on its
  lifetime (`enum Batched<'i, T> where Timed<T>: SeriesColumns<'i>`) can't be derived with `owned`: the owned side
  has no `'i` and would need the bound as `for<'i> SeriesColumns<'i>`. `ww_client_server::observe::Batched` /
  `BatchedOwned` (API-39) are written by hand for that.
- ✅ **DER-8 `#[cfg(..)]` on enum variants** — a variant keeps its discriminant in every configuration, a build without
  it reads `Error::EnumVariantDisabled` (`docs/derive_reference.md`). First user: `ww_numeric::NumericValue`, one cargo
  feature per number type (32 to 8 bytes in RAM with 32-bit types only). Not yet: `#[cfg]` on fields (it would change
  the wire format, should be a compile error), `ww_si` enums.
- ✅ **DER-23 A field written as a user type**: `#[wire(W)]` (DER-19) with any `W: WireAs<T>`, not only integers: a
  field keeps a handle in Rust and writes the value it stands for (a one-byte handle in Rust, a full `Magnitude` on
  the wire; `exact_units::Prefix`'s families use the integer form, `Pow10(#[wire(i8)] Pow10Exp)`). In a `Series` the field is an opaque run of
  `W`'s bytes (`series::WiredOpaque`, the model marks it series-opaque) while store and cells keep the handle. Tests
  `shrink_wrap/tests/wire_as_user.rs`.
- ✅ **DER-24 An enum's tag in a niche**: `niche` leaves the `#[repr]` the derive gives an enum (`u32` for `unib32`)
  out, so Rust keeps the tag in a field's spare values (`exact_units::Prefix` with its families and `Unknown(u16)` is 4 B, not 8).
  The same bytes, `discriminant()` a `match`. Whether `niche` becomes the default for every enum is open (Roman).
  Tests `shrink_wrap/tests/niche.rs`.
- ✅ **DER-25 Narrow tag-only catch-all for `niche` enums** — `#[future_version] Unknown(u16)` (or `u8`) on
  a `self_describing` / `niche` enum (DER-24) such as `Prefix`: the catch-all holds the tag only, so it doesn't widen
  the enum the way SW-47's `Unknown(u32)` would (`Prefix` 4 B instead of 8). The rule it enforces (Roman, 28 Oct
  2026): every variant added after the enum's first release is tag-only, its numbers packed into the `UNib32` tag,
  never bytes after it, so an older reader yields `Unknown(tag)`, skips the self-describing value and decodes the
  rest of the message. The derive (a known discriminant beyond the field is a compile error, a read one fails as
  without the catch-all, series tag store of the field's type; also as `cfg_attr(feature = .., future_version)`), the
  width in the API model (`FutureVersion::Variant::bits`, `None` for `u32` so older models keep their bytes), the
  dynamic client and the evolution checker (an appended discriminant beyond it is breaking), `exact_units::Prefix`,
  and the conformance case `stdlib.si.quantity.unknown_scale` (derive, dynamic, Python; C gap API-26).
- ✅ **DER-26 The derive accepts only what it writes**: the type names `uleb32`..`ileb128` (never written, a panic
  in codegen) and the field attribute `#[id = N]` (read and not used) are gone; an enum without any variant, and
  `#[flag]` or `#[default = ..]` on a tuple variant's field, are errors that name the cause.

## Testing and code size (`TEST`)

- ✅ **TEST-17 `shrink_wrap` code size bench**: `just size-bench` builds one bare firmware image per test case
  (plain struct, enum, list, bit fields, `Option`, decode or encode only, the request/reply codec of a small
  softcore firmware and the same by hand) for RV32I, RV32IC, Cortex-M0 and Cortex-M4F/M7 and prints `.text` +
  `.rodata`, or the largest symbols (`size_bench`, results in `docs/code_size.md`).

## Dropped and superseded

- **SW-80 A small-field bit read without the word read** — dropped (Roman, 14 Nov 2026): real firmware links the bit
  reader anyway. It was a read for bit fields of 8 bits or less that would spare a firmware without wider ones
  shrink_wrap's word read (about 250 B on a Cortex-M0+, since EVO-76 linked by every server for `Request`'s 3-bit
  priority). Tried while landing EVO-76: a `take_bits8` behind `read_un8` / `read_un8_latch` brought the one-method
  floor from +260 to +108 B, but `bits_de` (`just size-bench`, small and wide fields together) grew 704 → 888 B on
  thumbv6m, as both reads are linked.
- **SW-31 `Delta` / `DeltaOfDelta`**, **SW-32 `XorFloat`**, **SW-50 compressed sequences in APIs** — superseded by
  `Series<T>` (SW-60): its numeric columns use the SW-31 prefix code with the delta or delta of delta predictor, its
  float columns the SW-32 XOR code, plus packed, constant and plain and the smallest picked per column, in every API
  position and in dynamic clients and Python (CLT-29); the types were removed (18 Oct 2026).
