# AGENTS.md

Guidance for AI code assistants working in this repository.

## What this is

shrink_wrap is a compact, zero-copy wire format for microcontrollers: no allocator, bit- and nibble-packed,
backwards and forwards compatible, with a derive macro for user types. It is the lowest layer of
[WireWeaver](https://github.com/vhrdtech/wire_weaver) and depends on nothing of it. WireWeaver, `exact_units` and
other repositories take it by git.

Read `docs/shrink_wrap.md` and `docs/wire_format.md` first, then `docs/derive.md` with `derive_reference.md`,
`size_assumptions.md` and `layouts.md` (before touching any type definition), `special_types.md`, `series.md` and
`showcase.md` (byte-verified tricks). The docs are published as the SerDes section of the WireWeaver book, which
assembles them from this repository: links to other pages of the book (`../api/..`, `../types.md`) only resolve
there. Prefer the docs to the source; where they are unclear, the source and its comments are authoritative.

## Layout

| Path | What |
| --- | --- |
| `shrink_wrap/` | `BufReader`/`BufWriter`, the `SerializeShrinkWrap`/`DeserializeShrinkWrap` traits (+ `..Owned`), `U1`..`U64`, `UNib32`, varints, `RefVec`, `TailBytes`, `OptionAnyVec`, `fixed_layout`, `head_tail`, `Series` |
| `shrink_wrap_derive/` | `#[derive_shrink_wrap]`: directives in `docs/derive_reference.md` |
| `size_bench/` | code size per test case on small CPUs: runner library and binary; `fw/` the images, `fw/rt` (`size_bench_rt`) what they share |
| `fuzz/` | cargo-fuzz targets (`shrink-wrap-*`), a workspace of its own on nightly |
| `docs/` | the pages of the format |

`size_bench/fw` and `fuzz/` are separate workspaces (bare-metal targets, nightly): build them through `just`.

## FEATURES.md

`FEATURES.md` is the feature tracker and roadmap, the only place status is kept. Check it before starting a feature
and update the matching item in the same change when finishing one (conventions at its top).

- IDs are stable per area (`SW-3`, `DER-8`): never renumber or reuse one; new items take the next free number of
  their area. Use them in commits, CHANGELOG entries and
  `// TODO(SW-20): ...`.
- When talking to the user, name an ID with a 2-4 word kebab-case slug from its title: `SW-3 string-and-vec`.
- Items that came from WireWeaver kept their IDs; IDs of other areas (`EVO`, `SRV`, `API`, ...) are WireWeaver's.

## Commands

```sh
just test --changed   # the crates changed against main and their dependents; before a session's work is ready
just test             # everything: nextest, doctests, just check, a smoke run of every fuzz target
just lint             # fmt, cargo sort -w -g, typos, clippy and rustdoc -D warnings, the C readers
just check            # no_std builds on Cortex-M0, M4F and RV32I, optimize-speed on nightly, a size_bench image
just fuzz <target>    # one fuzz target until stopped (or for N seconds)
just size-bench       # code size per case and target, `just size-bench --help`
just miri             # series cells and in-place builders under miri
```

Single test: `cargo nextest run -p shrink_wrap <test>` (nextest is required). Tests run with default features only
(`defmt` breaks host linking). Plain `cargo sort` flattens grouped tables: always pass `-w -g`. Project words go
into `typos.toml`. CI (`.github/workflows/ci.yml`) fails on any warning.

## Proper fixes, never a lost capability

- A bug or limit found on the way is fixed where it lives, as its own commit, with a test that fails without it.
- A removal, replacement or refactor keeps everything that worked before. Never narrow tests, docs or
  `FEATURES.md` to fit a loss.
- Flaky and slow tests: find the cause, never raise a limit (`.config/nextest.toml` holds hang detectors only).

## Wire changes and downstream

- The bytes are a contract. A change of what any value encodes to is a wire break: it bumps the `wire = "wN.M"`
  line in `shrink_wrap/Cargo.toml` (`GENERATION`) when it is a global break, needs an `EVO-n` item in WireWeaver and a
  breaking CHANGELOG entry here.
- WireWeaver keeps the conformance corpus (`tests/conformance` there): the derive, the dynamic client, Python and
  the C runtime decode the same golden bytes. A wire-relevant change is checked against it before it lands
  downstream: develop with a `[patch."https://github.com/romixlab/shrink_wrap"]` in WireWeaver's worktree
  `.cargo/config.toml` (never committed) and run its `frozen_bytes` and `coverage` tests.
- Claims about bytes or bit layout in docs are never hand-derived: run a temporary example
  (`cargo run --example scratch_*`), copy the output, delete the example. `tests/use_cases.rs` asserts every number
  of `docs/use_cases.md`.

## Firmware: stack, cycles and flash

The readers and writers run on small MCUs, often in interrupts. Every change to them is checked for code size and
speed, not only correctness.

- **Flash.** `just size-bench` before and after, at the shipping profile (`opt-level = "z"`, fat LTO); say which
  targets and quote the table rows that changed. What a firmware does not call must not be linked.
- **Speed.** Hot per-item functions take `#[inline(always)]` behind the `fast-io` feature; `optimize-speed` puts
  `#[optimize(speed)]` on the bit paths. Measure at the profile firmware ships with and say which.
- **No feature changes bytes.** A cargo feature may trade size for speed, never the wire format.

## Changelogs, commits and versions

- Every user-visible change gets a line under `## Unreleased` in `CHANGELOG.md`, in the same change, with the item
  id (`### ⚠️ Breaking`, `### 🚀 Features`, `### 🐛 Fixes`; for breaking changes say what users must change).
- Commits use Conventional Commits with a scope (`feat(series): ...`): a short imperative subject, a blank line, a
  body of at most 5 lines saying what and why, and a `Tested:` line. A move is never edited in the same commit.
- Work commits never bump versions; the release commit does. Each landing turns `## Unreleased` into a version
  section. Pre-1.0 a breaking change bumps the minor, anything else the patch.
- Dependencies live in the root `[workspace.dependencies]`; crates inherit `authors`, `edition`, `rust-version`,
  `license`, `repository` and `[lints]`.
- Public text (README, docs, CHANGELOG, commits, comments) is neutral: no private paths or internal tracker names.

## Naming

Bit-sized integers are `U1`..`U64` and `I2`..`I64`; owned forms of borrowed types end in `Owned` (`RefVec` and
`Vec`, `TailBytes` and `TailBytesOwned`, `ArrayString`/`ArrayVec` on the stack).
