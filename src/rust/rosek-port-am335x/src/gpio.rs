//! GPIO driver and USR-LED life-sign blink. Issue #4 — see PLAN.md
//! "Zielhardware" ("Lebenszeichen ohne Kabel: User-LEDs USR0–3 über
//! GPIO1") and docs/req42/04-product-backlog.adoc EPIC 0/STORY 0.4.1.
//!
//! ## Board discrepancy: GPIO1 vs GPIO2
//!
//! PLAN.md says the USR LEDs hang off GPIO1, but the mainline Linux
//! device tree for the actual BeagleBone Black
//! (`am335x-bone-common.dtsi`) wires USR0..USR3 to **GPIO2** pins 21..24
//! (`gpios = <&gpio2 21 0>` etc.) — GPIO1 pins 21..24 is the wiring for
//! the BeagleBone **Blue**, a different board. Per explicit user
//! direction (raised and confirmed during this issue's implementation,
//! rather than silently "fixing" PLAN.md's assumption or silently
//! guessing which board this actually runs on): drive **both** GPIO1 and
//! GPIO2 pins 21..24, so the right LEDs light up either way, alternating
//! on/off as the life sign.
//!
//! ## Register layout
//!
//! GPIO0/1/2/3 base addresses and the `GPIO_OE`/`GPIO_SETDATAOUT`/
//! `GPIO_CLEARDATAOUT` offsets below are confirmed against the AM335x TRM
//! memory map (section 2, GPIO module addresses) and TRM section 25 (GPIO
//! registers), not assumed from memory. `GPIO_OE` uses the OMAP-inherited
//! convention where a **cleared** bit means "output enabled" and a set
//! bit means "input" (confirmed against the mainline `gpio-omap.c`
//! driver's direction-set logic) — the inverse of what the register name
//! alone might suggest, and worth getting wrong exactly once rather than
//! per board bring-up.

const GPIO1_BASE: usize = 0x4804_C000;
const GPIO2_BASE: usize = 0x481A_C000;

const GPIO_OE: usize = 0x134;
const GPIO_CLEARDATAOUT: usize = 0x190;
const GPIO_SETDATAOUT: usize = 0x194;

/// USR0..USR3 are pins 21..24 on whichever bank actually carries them.
const USR_LED_MASK: u32 = 0b1111 << 21;

const GPIO_BANKS: [usize; 2] = [GPIO1_BASE, GPIO2_BASE];

#[inline(always)]
unsafe fn read_reg(base: usize, offset: usize) -> u32 {
    core::ptr::read_volatile((base + offset) as *const u32)
}

#[inline(always)]
unsafe fn write_reg(base: usize, offset: usize, value: u32) {
    core::ptr::write_volatile((base + offset) as *mut u32, value);
}

/// The four USR LEDs, driven redundantly on GPIO1 and GPIO2 (see module
/// docs) so they light up on both BeagleBone Black and BeagleBone Blue
/// wiring.
pub struct UserLeds;

impl UserLeds {
    /// Configures pins 21..24 as outputs on both GPIO1 and GPIO2, read-
    /// modify-write so any other pin on either bank is left untouched.
    ///
    /// # Safety
    ///
    /// Must not run concurrently with any other access to the GPIO1/GPIO2
    /// `GPIO_OE` registers.
    pub unsafe fn init() -> Self {
        for &base in &GPIO_BANKS {
            let oe = read_reg(base, GPIO_OE);
            write_reg(base, GPIO_OE, oe & !USR_LED_MASK);
        }
        UserLeds
    }

    pub fn all_on(&mut self) {
        for &base in &GPIO_BANKS {
            unsafe { write_reg(base, GPIO_SETDATAOUT, USR_LED_MASK) };
        }
    }

    pub fn all_off(&mut self) {
        for &base in &GPIO_BANKS {
            unsafe { write_reg(base, GPIO_CLEARDATAOUT, USR_LED_MASK) };
        }
    }
}

/// Crude busy-wait: no timer driver exists yet (later phases), and a
/// life-sign blink only needs to be roughly human-visible, not precisely
/// timed.
fn delay(loops: u32) {
    for _ in 0..loops {
        unsafe { core::arch::asm!("nop") };
    }
}

/// Blinks all USR LEDs on/off forever as a life sign. Never returns.
pub fn blink_forever(leds: &mut UserLeds) -> ! {
    const HALF_PERIOD_LOOPS: u32 = 6_000_000; // roughly human-visible at ~1 GHz, no timer to calibrate against yet
    loop {
        leds.all_on();
        delay(HALF_PERIOD_LOOPS);
        leds.all_off();
        delay(HALF_PERIOD_LOOPS);
    }
}
