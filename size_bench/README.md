# size_bench

Builds bare firmware images, one test case each, for RV32I, RV32IC, Cortex-M0 and Cortex-M4F/M7 and prints their
`.text` + `.rodata`, or the largest symbols of an image: `just size-bench --help`. The images are in `fw/`, their
shared entry point, panic handler and linker script in `fw/rt` (`size_bench_rt`). The library (`Bench`) measures a
firmware crate elsewhere of the same shape; results and what they mean are in `docs/code_size.md`.
