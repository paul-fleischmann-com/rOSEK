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
//! USR LEDs forever as a life sign (`rosek-port-am335x::gpio`, issue #4).
//! [`panic`] (issue #5) is this crate's other job: turning an unrecoverable
//! error into a controlled, observable state instead of undefined
//! behaviour, per docs/req42/04-product-backlog.adoc EPIC 0/STORY 0.5.1.
//!
//! [`config`] is `build.rs`-generated from `rosek.cfg` (issue #7/#8/#9):
//! plain `&'static` data, no heap or dynamic allocation. Nothing consumes
//! it at runtime yet (the scheduler is Phase 3) - `board_main` prints its
//! shape over UART as end-to-end proof the config -> codegen pipeline
//! actually produces linked-in, correct data, not just code that compiles.

#![no_std]
#![no_main]

use core::fmt::Write as _;

// Pulls in rosek-port-am335x::startup's `global_asm!` (vector table, `_start`)
// as part of this binary's link graph; `_start` is the linker ENTRY point
// (see linker.ld) and reaches `board_main` by branching to it directly from
// assembly, not through a Rust call.
use rosek_port_am335x as _;

mod config {
    include!(concat!(env!("OUT_DIR"), "/config.rs"));
}

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
    let _ = writeln!(
        uart,
        "rOSEK: config loaded ({} task(s), {} OS-Application(s))",
        config::TASKS.len(),
        config::OS_APPLICATIONS.len()
    );

    let mut leds = unsafe { rosek_port_am335x::gpio::UserLeds::init() };
    rosek_port_am335x::gpio::blink_forever(&mut leds)
}

/// Deliberately self-contained: a panic can happen before or during
/// `board_main`'s own UART/GPIO init (or, in principle, with those in some
/// corrupted state), so this re-initialises both from scratch rather than
/// trusting any prior state - the one function in this crate that must not
/// itself rely on anything having gone right so far.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Mask interrupts unconditionally: true throughout Phase 0 today, but
    // this function must stay correct even once a later phase unmasks them.
    unsafe { core::arch::asm!("cpsid if") };

    let mut uart = unsafe { rosek_port_am335x::uart::Uart0::init() };
    let _ = writeln!(uart, "\nPANIC: {info}");

    let mut leds = unsafe { rosek_port_am335x::gpio::UserLeds::init() };
    // Faster than the normal heartbeat (gpio::blink_forever) so "panicked"
    // is distinguishable from "running normally" by eye, without a UART
    // terminal attached.
    const PANIC_BLINK_HALF_PERIOD_LOOPS: u32 =
        rosek_port_am335x::gpio::HEARTBEAT_HALF_PERIOD_LOOPS / 6;
    loop {
        leds.all_on();
        rosek_port_am335x::gpio::delay(PANIC_BLINK_HALF_PERIOD_LOOPS);
        leds.all_off();
        rosek_port_am335x::gpio::delay(PANIC_BLINK_HALF_PERIOD_LOOPS);
    }
}
