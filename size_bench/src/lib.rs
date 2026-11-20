//! Builds bare firmware images, one test case each, for small CPU targets and measures their code size, or lists
//! the largest symbols of an image. See `docs/code_size.md` for what the numbers mean.
//!
//! The `size_bench` binary measures this crate's own images (`fw/`). A firmware crate elsewhere of the same shape
//! is measured the same way through [`Bench`]: one cargo feature per case, a `final` feature for the flavor with
//! size assumptions, and `size_bench_rt` for its entry point and linker script.

use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use object::{Object, ObjectSection, ObjectSymbol, SymbolKind};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A test case of a firmware crate: one of its cargo features.
#[derive(Copy, Clone, Debug)]
pub struct Case {
    /// The cargo feature that selects the case
    pub name: &'static str,
    /// The case uses derived types, so it also has a `final` flavor
    pub has_final: bool,
}

/// A CPU an image is built for.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Target {
    /// RISC-V RV32I, no compressed instructions
    Rv32i,
    /// RISC-V RV32IC (the QERV softcore of `fpga_tools`' busgen)
    Rv32ic,
    /// RV32IC with linker relaxation (`-C target-feature=+relax`): calls take 2 or 4 bytes instead of 8
    Rv32icRelax,
    /// Cortex-M0
    Thumbv6m,
    /// Cortex-M4F / M7 (Thumb-2), for comparison with larger MCUs
    Thumbv7em,
}

impl Target {
    /// The name in the table's header and in `--target`
    pub fn name(self) -> &'static str {
        match self {
            Self::Rv32i => "rv32i",
            Self::Rv32ic => "rv32ic",
            Self::Rv32icRelax => "rv32ic-relax",
            Self::Thumbv6m => "thumbv6m",
            Self::Thumbv7em => "thumbv7em",
        }
    }

    fn triple(self) -> &'static str {
        match self {
            Self::Rv32i | Self::Rv32ic | Self::Rv32icRelax => "riscv32i-unknown-none-elf",
            Self::Thumbv6m => "thumbv6m-none-eabi",
            Self::Thumbv7em => "thumbv7em-none-eabihf",
        }
    }

    fn rustflags(self) -> String {
        let common = "-C link-arg=-Tlink.x -Zlocation-detail=none -Zfmt-debug=none";
        match self {
            Self::Rv32ic => format!("{common} -C target-feature=+c"),
            Self::Rv32icRelax => format!("{common} -C target-feature=+c,+relax"),
            _ => common.to_owned(),
        }
    }
}

/// The two flavors of a case with derived types.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Flavor {
    /// Types as derived by default: `Unsized`, fields can be added later
    Evolvable,
    /// `sized` where the fields allow it, else `final_structure`
    Final,
}

/// What to build and what to print.
#[derive(Parser, Debug)]
#[command(about, version)]
pub struct Args {
    /// Targets to build for, default: all
    #[arg(short, long, value_enum, value_delimiter = ',')]
    pub target: Vec<Target>,
    /// Cases to build, default: all
    #[arg(short, long, value_delimiter = ',')]
    pub case: Vec<String>,
    /// Only the flavor with size assumptions (`sized` / `final_structure` types), or only the evolvable one
    #[arg(long)]
    pub flavor: Option<Flavor>,
    /// `shrink_wrap` features to build with, e.g. `fast-io`
    #[arg(long, value_delimiter = ',')]
    pub sw_features: Vec<String>,
    /// Features of the firmware itself, e.g. `byte_memcpy` (its own byte-wise `memcpy`)
    #[arg(long, value_delimiter = ',')]
    pub fw_features: Vec<String>,
    /// Instead of the table: the N largest symbols of each image
    #[arg(short, long, value_name = "N")]
    pub symbols: Option<usize>,
}

/// Sizes of one built image.
#[derive(Debug)]
pub struct Image {
    /// Bytes of `.text`
    pub text: u64,
    /// Bytes of `.rodata`
    pub rodata: u64,
    /// (size, demangled name) of every code and data symbol, largest first
    pub symbols: Vec<(u64, String)>,
}

/// A firmware crate and its cases.
#[derive(Debug)]
pub struct Bench<'a> {
    /// Directory of the firmware crate (with its own `rust-toolchain.toml`)
    pub fw: PathBuf,
    /// Name of the binary cargo builds there
    pub bin: &'a str,
    /// Its cases, in the order of the table
    pub cases: &'a [Case],
}

/// The images of a run: one per target, case and flavor.
#[derive(Debug)]
pub struct Report {
    targets: Vec<Target>,
    /// (case, flavor, the case has a `final` flavor)
    rows: Vec<(&'static str, Flavor, bool)>,
    /// One list per target, in the order of `rows`
    images: Vec<Vec<Image>>,
}

impl Bench<'_> {
    /// Builds the images `args` asks for, targets in parallel, cases one after another. `progress` gets a line
    /// before each build.
    ///
    /// # Panics
    ///
    /// Panics when a build thread panicked.
    pub fn run(&self, args: &Args, progress: &(dyn Fn(&str) + Sync)) -> Result<Report> {
        let targets = if args.target.is_empty() {
            Target::value_variants().to_vec()
        } else {
            args.target.clone()
        };
        for case in &args.case {
            if !self.cases.iter().any(|c| c.name == case) {
                let names: Vec<_> = self.cases.iter().map(|c| c.name).collect();
                bail!("no case `{case}`, there are: {}", names.join(", "));
            }
        }
        let mut rows = vec![];
        for case in self.cases {
            if !args.case.is_empty() && !args.case.iter().any(|c| c == case.name) {
                continue;
            }
            for flavor in [Flavor::Evolvable, Flavor::Final] {
                let wanted = args.flavor.is_none_or(|f| f == flavor);
                if (flavor == Flavor::Final && !case.has_final) || (case.has_final && !wanted) {
                    continue;
                }
                rows.push((case.name, flavor, case.has_final));
            }
        }

        let results: Vec<Result<Vec<Image>>> = std::thread::scope(|scope| {
            let handles: Vec<_> = targets
                .iter()
                .map(|&target| {
                    let rows = &rows;
                    scope.spawn(move || {
                        rows.iter()
                            .enumerate()
                            .map(|(i, &(case, flavor, _))| {
                                progress(&format!(
                                    "[{}/{}] {} {case} {flavor:?}",
                                    i + 1,
                                    rows.len(),
                                    target.name()
                                ));
                                self.build(target, case, flavor, args)
                            })
                            .collect()
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| {
                    h.join()
                        .expect("a build thread only returns errors, never panics")
                })
                .collect()
        });
        Ok(Report {
            targets,
            rows,
            images: results.into_iter().collect::<Result<Vec<_>>>()?,
        })
    }

    fn build(&self, target: Target, case: &str, flavor: Flavor, args: &Args) -> Result<Image> {
        let mut features = vec![case.to_owned()];
        if flavor == Flavor::Final {
            features.push("final".into());
        }
        features.extend(args.fw_features.iter().cloned());
        features.extend(args.sw_features.iter().map(|f| format!("shrink_wrap/{f}")));
        // one target dir per target: the RV32 ones share a triple and differ in RUSTFLAGS only
        let target_dir = self.fw.join("target").join(target.name());
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&self.fw)
            .args([
                "build",
                "--release",
                "--quiet",
                "--target",
                target.triple(),
                "--features",
                &features.join(","),
            ])
            .arg("--target-dir")
            .arg(&target_dir)
            .env("RUSTFLAGS", target.rustflags());
        // the firmware crate has its own toolchain file; don't hand down the toolchain this runner was started with
        for var in [
            "RUSTUP_TOOLCHAIN",
            "RUSTC",
            "RUSTDOC",
            "CARGO",
            "CARGO_TARGET_DIR",
            "CARGO_ENCODED_RUSTFLAGS",
        ] {
            cmd.env_remove(var);
        }
        let status = cmd.status().context("running cargo")?;
        if !status.success() {
            bail!("build of case {case} for {} failed", target.name());
        }
        let elf = target_dir
            .join(target.triple())
            .join("release")
            .join(self.bin);
        read_image(&elf)
    }
}

fn read_image(elf: &Path) -> Result<Image> {
    let data = std::fs::read(elf).with_context(|| format!("reading {}", elf.display()))?;
    let file = object::File::parse(&*data)?;
    let size = |name| file.section_by_name(name).map_or(0, |s| s.size());
    let mut symbols: Vec<(u64, String)> = file
        .symbols()
        .filter(|s| s.size() > 0 && matches!(s.kind(), SymbolKind::Text | SymbolKind::Data))
        .filter(|s| {
            let section = s
                .section_index()
                .and_then(|i| file.section_by_index(i).ok());
            section.is_some_and(|sec| matches!(sec.name(), Ok(".text" | ".rodata")))
        })
        .map(|s| {
            (
                s.size(),
                format!("{:#}", rustc_demangle::demangle(s.name().unwrap_or("?"))),
            )
        })
        .collect();
    symbols.sort_by(|a, b| b.cmp(a));
    Ok(Image {
        text: size(".text"),
        rodata: size(".rodata"),
        symbols,
    })
}

impl Report {
    /// A Markdown table: `.text` + `.rodata` in bytes, a row per case and flavor, a column per target.
    pub fn table(&self) -> String {
        let mut out = String::from("| case | types |");
        for t in &self.targets {
            write!(out, " {} |", t.name()).expect("writing to a String");
        }
        out.push_str("\n| --- | --- |");
        out.push_str(&" ---: |".repeat(self.targets.len()));
        out.push('\n');
        for (i, &(case, flavor, has_final)) in self.rows.iter().enumerate() {
            let flavor = match (has_final, flavor) {
                (false, _) => "",
                (true, Flavor::Evolvable) => "evolvable",
                (true, Flavor::Final) => "final",
            };
            write!(out, "| {case} | {flavor} |").expect("writing to a String");
            for images in &self.images {
                let image = &images[i];
                write!(out, " {} |", image.text + image.rodata).expect("writing to a String");
            }
            out.push('\n');
        }
        out
    }

    /// The `n` largest symbols of each image, and the sum of the rest.
    pub fn symbols(&self, n: usize) -> String {
        let mut out = String::new();
        for (target, images) in self.targets.iter().zip(&self.images) {
            for (&(case, flavor, _), image) in self.rows.iter().zip(images) {
                writeln!(
                    out,
                    "\n{case} ({flavor:?}), {}: .text {}, .rodata {}",
                    target.name(),
                    image.text,
                    image.rodata
                )
                .expect("writing to a String");
                for (size, name) in image.symbols.iter().take(n) {
                    writeln!(out, "{size:>7}  {name}").expect("writing to a String");
                }
                let rest: u64 = image.symbols.iter().skip(n).map(|(size, _)| size).sum();
                if rest > 0 {
                    writeln!(
                        out,
                        "{rest:>7}  ({} more)",
                        image.symbols.len().saturating_sub(n)
                    )
                    .expect("writing to a String");
                }
            }
        }
        out
    }
}
