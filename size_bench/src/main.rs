//! Builds the firmware images in `fw/` (one test case each) for small CPU targets and prints their code size,
//! or the largest symbols of one image. See `docs/code_size.md` for what the numbers mean.

use anyhow::Result;
use clap::Parser;
use size_bench::{Args, Bench, Case};
use std::path::PathBuf;

const fn case(name: &'static str, has_final: bool) -> Case {
    Case { name, has_final }
}

/// Test cases of `fw/` (its cargo features); `true`: the case uses derived types, so it has a `final` flavor.
const CASES: &[Case] = &[
    case("empty", false),
    case("rw_de", false),
    case("rw_ser", false),
    case("struct_de", true),
    case("struct_ser", true),
    case("header_de", false),
    case("header_ser", false),
    case("fixed_de", false),
    case("fixed_ser", false),
    case("bits_fixed_de", false),
    case("bits_fixed_ser", false),
    case("enum_de", true),
    case("enum_ser", true),
    case("vec_de", true),
    case("vec_ser", true),
    case("bits_de", true),
    case("bits_ser", true),
    case("option_de", true),
    case("option_ser", true),
    case("busgen_de", true),
    case("busgen_ser", true),
    case("busgen", true),
    case("busgen_hand", false),
    case("busgen_ht", false),
    case("busgen_view", false),
];

fn main() -> Result<()> {
    let args = Args::parse();
    let bench = Bench {
        fw: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fw"),
        bin: "size_bench_fw",
        cases: CASES,
    };
    let report = bench.run(&args, &|line| eprintln!("{line}"))?;
    match args.symbols {
        Some(n) => print!("{}", report.symbols(n)),
        None => print!("{}", report.table()),
    }
    Ok(())
}
