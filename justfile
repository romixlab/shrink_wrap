# `just jobs=N <recipe>` caps build and test parallelism for that run; unset leaves CARGO_BUILD_JOBS and
# NEXTEST_TEST_THREADS as inherited, or at cargo's default (the CPU count)
jobs := ""
export CARGO_BUILD_JOBS := if jobs != "" { jobs } else { env_var_or_default('CARGO_BUILD_JOBS', num_cpus()) }
export NEXTEST_TEST_THREADS := if jobs != "" { jobs } else { env_var_or_default('NEXTEST_TEST_THREADS', 'num-cpus') }

fuzz_targets := "tail-size ref-vec latched option-any-vec series-columns series-type nesting"
# cargo-fuzz defaults its fuzz target to the triple its own binary was built for; a
# prebuilt cargo-fuzz release (taiki-e/install-action's cargo-binstall fallback uses
# one) is a static musl build, so that default becomes a musl target this host never
# installed. Passing the host's own triple keeps the fuzz build on it regardless.
fuzz_host_triple := `rustc -vV | sed -n 's/^host: //p'`

# List all available targets
# The pinned nightly of the fuzz and miri recipes; rust-toolchain.toml pins the stable one
nightly := "nightly-2026-11-19"

default:
    @just --list

# The tests. `just test`: all of them (test-all). `just test --changed`: only the crates changed against main and
# the crates that depend on them (through tpm when installed, otherwise the full suite)
test *flags:
    if [ "{{ flags }}" = "--changed" ]; then \
        if command -v changed-tests >/dev/null 2>&1; then f=$(changed-tests) && if [ -n "$f" ]; then cargo nextest run --workspace -E "$f"; fi; \
        else echo "changed-tests not found; running the full test suite instead" >&2; {{ just_executable() }} test-all; fi; \
    else {{ just_executable() }} test-all {{ flags }}; fi

# Every test: nextest (the C readers of fixed_layout and head_tail types run there, under the sanitizers), the
# doctests, the builds of `just check`, and a short run of every fuzz target
test-all: check
    cargo nextest run --workspace --no-fail-fast
    # the writer with 4-byte length slots: its own tests, and every test that is not sized for 2-byte ones
    cargo nextest run -p shrink_wrap --features len32 --no-fail-fast
    # nextest does not run doctests
    cargo test --workspace --doc -q
    just fuzz-smoke

# cargo check what tests don't build: no_std on small CPUs with the firmware features, `optimize-speed` on nightly,
# the size bench's firmware images
check:
    cargo check -p shrink_wrap --no-default-features --target thumbv6m-none-eabi
    cargo check -p shrink_wrap --no-default-features --features fast-io,refused,defmt,serde --target thumbv7em-none-eabihf
    cargo check -p shrink_wrap --no-default-features --features fast-io,len32 --target riscv32i-unknown-none-elf
    cargo +{{ nightly }} check -p shrink_wrap --features optimize-speed
    cd size_bench/fw && cargo check --features busgen --target thumbv6m-none-eabi

# Format, manifests, spelling, clippy and rustdoc as CI runs them, warnings are errors; the C readers on their own
lint:
    cargo fmt --all --check
    cd fuzz && cargo fmt --check
    cd size_bench/fw && cargo fmt --all --check
    cargo sort -w -g --check
    typos
    cargo clippy --workspace --all-targets -- -D warnings
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
    just lint-c

# Build and run the generated fixed-layout (SW-45) and head+tail (SW-46) C readers, warnings are errors
lint-c:
    mkdir -p target
    cc -std=c99 -Wall -Wextra -Werror -pedantic -Wconversion -Wsign-conversion -o target/fixed_layout_c shrink_wrap/tests/fixed_layout_c.c
    target/fixed_layout_c
    cc -std=c99 -Wall -Wextra -Werror -pedantic -Wconversion -Wsign-conversion -o target/head_tail_c shrink_wrap/tests/head_tail_c.c
    target/head_tail_c

# The series cells round trips and the in-place builders under miri (needs `rustup component add miri` on the toolchain `nightly` names)
miri:
    cargo +{{ nightly }} miri test -p shrink_wrap --lib cells_tests
    cargo +{{ nightly }} miri test -p shrink_wrap --lib in_place

# Code size on small CPUs (RV32I, RV32IC, Cortex-M0) per test case, `just size-bench --help` (docs/code_size.md)
size-bench *args:
    cargo run -q --release -p size_bench -- {{ args }}

# Install cargo-fuzz (fuzzing needs a nightly toolchain)
fuzz-install:
    cargo install cargo-fuzz

# Fuzz one target for SECONDS (empty: until stopped): tail-size (SW-22), ref-vec (SW-37), latched (SW-38),
# option-any-vec (SW-55), series-columns, series-type, nesting. The corpus it grows (fuzz/corpus) and a crash
# (fuzz/artifacts) stay out of git
[working-directory('fuzz')]
fuzz target seconds="":
    cargo +{{ nightly }} fuzz run --target {{ fuzz_host_triple }} shrink-wrap-{{ target }} -- -max_len=4096 {{ if seconds == "" { "" } else { "-max_total_time=" + seconds } }}

# Every fuzz target for 20000 runs: a smoke run, part of `just test`
[working-directory('fuzz')]
fuzz-smoke:
    for t in {{ fuzz_targets }}; do cargo +{{ nightly }} fuzz run -O --target {{ fuzz_host_triple }} shrink-wrap-$t -- -runs=20000 -max_len=4096 || exit 1; done

# A library has nothing to put on PATH: builds it with the deploy profile, as a dependent's release build would
install:
    cargo build --profile deploy --locked --workspace

# Bring this checkout up to main: refuses on local changes, pulls, builds
deploy:
    #!/usr/bin/env bash
    set -euo pipefail
    dirty=$(git status --porcelain)
    if [ -n "$dirty" ]; then echo "local changes, not deploying:"; echo "$dirty"; exit 1; fi
    git pull --ff-only
    just install
