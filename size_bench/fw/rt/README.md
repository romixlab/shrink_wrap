# size_bench_rt

What every firmware image of `size_bench` shares: the `entry!` macro (the reset handler and the receive and send
buffers), the panic handler, `src` / `sink`, the optional byte-wise `memcpy` and the linker script. An image built
elsewhere (a firmware measuring a framework's reply code, for example) takes it by git and uses the same runner.
