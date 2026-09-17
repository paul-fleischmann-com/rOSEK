//! rOSEK entry point for the BeagleBone Black.
//!
//! Everything up to here (vector table, per-CPU-mode stacks, `.bss` zeroing)
//! is `rosek-port-am335x::startup`'s job. This crate only has to provide
//! `board_main`, the symbol that startup code branches to once it's safe to
//! run arbitrary Rust.
//!
//! After that, `board_main` enables the flat MMU mapping
//! (`rosek-port-am335x::mmu`, issue #2), initialises UART0 and prints a
//! boot message (`rosek-port-am335x::uart`, issue #3), then blinks the
//! USR LEDs forever as a life sign (`rosek-port-am335x::gpio`, issue #4) -
//! the real panic handler is a separate, not-yet-implemented issue (#5;
//! see PLAN.md "Meilensteine" Phase 0 and
//! docs/req42/04-product-backlog.adoc EPIC 0).

#![no_std]
#![no_main]

use core::fmt::Write as _;

// Pulls in rosek-port-am335x::startup's `global_asm!` (vector table, `_start`)
// as part of this binary's link graph; `_start` is the linker ENTRY point
// (see linker.ld) and reaches `board_main` by branching to it directly from
// assembly, not through a Rust call.
use rosek_port_am335x as _;

/// Branched to from `rosek-port-am335x::startup::_start` once the vector
/// table, VBAR, all per-CPU-mode stacks and `.bss` are set up.
#[no_mangle]
pub extern "C" fn board_main() -> ! {
    // Safe here specifically: interrupts are still masked (set by _start,
    // never unmasked since) and both are the only call site, ever.
    unsafe { rosek_port_am335x::mmu::enable_flat_mapping() };
    let mut uart = unsafe { rosek_port_am335x::uart::Uart0::init() };

    let _ = writeln!(
        uart,
        "rOSEK: boot fundament up (MMU flat-mapped, UART0 115200 8N1)"
    );

    let mut leds = unsafe { rosek_port_am335x::gpio::UserLeds::init() };
    rosek_port_am335x::gpio::blink_forever(&mut leds)
}

/// Placeholder only - every `#![no_std]` binary needs *some* panic handler
/// to compile, but a real one (UART diagnostic + LED signal) is issue #5's
/// job, not this issue's.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    idle()
}

/// `wfi` (Wait For Interrupt) instead of a bare spin loop: halts the core
/// until *some* event/interrupt occurs instead of burning cycles at full
/// clock, even though nothing is unmasked to actually act on one yet.
fn idle() -> ! {
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}
