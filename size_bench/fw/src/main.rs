//! One firmware image per test case (a cargo feature each), built by the `size_bench` runner next to this crate
//! to see how much code shrink_wrap costs on a small CPU. Entry point, buffers and panic handler come from
//! `size_bench_rt`.
//!
//! Every case is a `run(rx, tx) -> usize`: decode what is in `rx`, encode into `tx`, return the bytes written.
#![no_std]
#![no_main]

mod cases;
#[cfg(feature = "busgen_hand")]
mod hand;
#[cfg(any(feature = "busgen_ht", feature = "busgen_view"))]
mod ht;
#[allow(dead_code)]
mod types;

size_bench_rt::entry!(cases::run);
