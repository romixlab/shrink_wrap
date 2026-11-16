//! What every `size_bench` firmware image shares. An image is a binary crate with one `run(rx, tx) -> usize` per
//! test case (a cargo feature each) and an [`entry!`] naming it; the `size_bench` runner builds it with
//! `-C link-arg=-Tlink.x` and measures it. The images are measured, not run: the reset handler sets up nothing.
//!
//! Inputs and outputs go through [core::hint::black_box] ([`src`], [`sink`]), so nothing is folded away at compile
//! time and every decoded field counts as used.
#![no_std]

/// A value the compiler knows nothing about
pub fn src<T>(v: T) -> T {
    core::hint::black_box(v)
}

/// Counts as a use of every part of `v`
pub fn sink<T>(v: T) {
    core::hint::black_box(&v);
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}

/// The reset handler of an image: calls `$run` on the receive buffer, again and again, with the send buffer for its
/// output. `$run` is `fn(&[u8], &mut [u8]) -> usize`, the bytes it wrote.
#[macro_export]
macro_rules! entry {
    ($run:path) => {
        static mut RX: [u8; 1100] = [0; 1100];
        static mut TX: [u8; 300] = [0; 300];

        #[unsafe(no_mangle)]
        #[unsafe(link_section = ".text.start")]
        extern "C" fn _start() -> ! {
            // SAFETY: one thread, no interrupts; this function owns the buffers
            let (rx, tx) =
                unsafe { (&*core::ptr::addr_of!(RX), &mut *core::ptr::addr_of_mut!(TX)) };
            loop {
                let rx = core::hint::black_box(&rx[..]);
                let len = $run(rx, tx);
                core::hint::black_box(&tx[..len]);
            }
        }
    };
}

/// A byte at a time `memcpy`, instead of the word-wise one of `compiler_builtins` (338 bytes on RV32IC, 316 on
/// Cortex-M0): about 20 bytes. The firmware's own symbols win over the weak ones of `compiler_builtins`. Off by
/// default: a measurement, and advice for very small targets only, it is slower and replaces `memcpy` for the
/// whole firmware (docs/code_size.md, "Build settings").
#[cfg(feature = "byte_memcpy")]
mod byte_memcpy {
    use core::ffi::c_void;

    /// # Safety
    /// `dest` and `src` are valid for `n` bytes and do not overlap.
    #[inline(always)]
    unsafe fn copy(dest: *mut c_void, src: *const c_void, n: usize) {
        let (dest, src) = (dest.cast::<u8>(), src.cast::<u8>());
        let mut i = 0;
        while i < n {
            // SAFETY: as the caller promised. The volatile store keeps LLVM from turning the loop back into a
            // call of `memcpy`, i.e. of itself.
            unsafe { dest.add(i).write_volatile(*src.add(i)) };
            i += 1;
        }
    }

    /// # Safety
    /// As `memcpy` in C: `dest` and `src` are valid for `n` bytes and do not overlap.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn memcpy(
        dest: *mut c_void,
        src: *const c_void,
        n: usize,
    ) -> *mut c_void {
        // SAFETY: the same contract as ours
        unsafe { copy(dest, src, n) };
        dest
    }

    /// What Arm code calls; the one of `compiler_builtins` would call its own `memcpy`, not the one above.
    ///
    /// # Safety
    /// As `memcpy` in C: `dest` and `src` are valid for `n` bytes and do not overlap.
    #[cfg(target_arch = "arm")]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn __aeabi_memcpy(dest: *mut c_void, src: *const c_void, n: usize) {
        // SAFETY: the same contract as ours
        unsafe { copy(dest, src, n) };
    }
}
