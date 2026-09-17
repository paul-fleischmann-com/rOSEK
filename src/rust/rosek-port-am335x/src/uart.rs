//! UART0 driver: initialisation (115200 8N1) and polled text output.
//! Issue #3 — see PLAN.md "Zielhardware" ("Konsole: UART0, 115200 8N1, über
//! USB-Seriell-Kabel am J1-Header") and docs/req42/04-product-backlog.adoc
//! EPIC 0/STORY 0.3.1.
//!
//! ## Register layout
//!
//! TI's UART IP on the AM335x (used identically across OMAP/Sitara SoCs) is
//! 16550-compatible for the low registers, with TI-specific extensions
//! above offset 0x20. UART0's base address and every offset below are
//! confirmed against coreboot's AM335x UART register header
//! (`src/soc/ti/am335x/uart.h`) and the AM335x TRM section on UART/IrDA/CIR
//! ("Protocol, Baud Rate, and Interrupt Settings" for MDR1, "Software
//! Reset" for SYSC/SYSS) rather than assumed from memory alone:
//!
//! - `0x44E0_9000`: UART0 base
//! - `+0x00` THR (write) / RHR (read), or DLL when `LCR.DLAB=1`
//! - `+0x04` IER, or DLH when `LCR.DLAB=1`
//! - `+0x08` FCR (write) / IIR (read)
//! - `+0x0C` LCR
//! - `+0x10` MCR
//! - `+0x14` LSR
//! - `+0x20` MDR1 (TI-specific mode select, not part of plain 16550)
//! - `+0x54` SYSC (module reset control)
//! - `+0x58` SYSS (module reset status)
//!
//! ## Baud rate
//!
//! The UART functional clock on the AM335x is 48 MHz (confirmed against
//! the mainline Linux `omap-serial` driver's `DEFAULT_CLK_SPEED`, which
//! targets this exact IP). With 16x oversampling the divisor for 115200
//! baud is `48_000_000 / (16 * 115200) = 26.04`, rounded to 26 — the same
//! value effectively every AM335x bring-up driver (U-Boot, Linux, TI's own
//! StarterWare examples) uses for this baud rate from this clock.
//!
//! ## Pinmux
//!
//! UART0 is the SoC's default boot/debug console — U-Boot/SPL already uses
//! it to print their own output before handing off to rOSEK, which means
//! its pin mux is already configured by the time `init()` runs here. This
//! driver deliberately does *not* touch `CONTROL_MODULE` pinmux registers;
//! if a future board ever changes the default console UART in its own
//! U-Boot config, this assumption would need revisiting.

use core::fmt;

const UART0_BASE: usize = 0x44E0_9000;

const THR_RHR_DLL: usize = 0x00;
const IER_DLH: usize = 0x04;
const FCR_IIR: usize = 0x08;
const LCR: usize = 0x0C;
const MCR: usize = 0x10;
const LSR: usize = 0x14;
const MDR1: usize = 0x20;
const SYSC: usize = 0x54;
const SYSS: usize = 0x58;

const LCR_DLAB: u32 = 1 << 7;
const LCR_8N1: u32 = 0b011; // 8 data bits, no parity, 1 stop bit
const FCR_FIFO_EN_CLEAR: u32 = 0b111; // FIFO enable + clear RX FIFO + clear TX FIFO
const LSR_THRE: u32 = 1 << 5; // Transmit Holding Register Empty
const MDR1_DISABLE: u32 = 0x7;
const MDR1_UART_16X: u32 = 0x0;
const SYSC_SOFTRESET: u32 = 1 << 1;
const SYSS_RESETDONE: u32 = 1 << 0;

const UART_FUNCTIONAL_CLOCK_HZ: u32 = 48_000_000;
const BAUD_115200_DIVISOR: u32 = UART_FUNCTIONAL_CLOCK_HZ / (16 * 115_200);

#[inline(always)]
unsafe fn read_reg(offset: usize) -> u32 {
    core::ptr::read_volatile((UART0_BASE + offset) as *const u32)
}

#[inline(always)]
unsafe fn write_reg(offset: usize, value: u32) {
    core::ptr::write_volatile((UART0_BASE + offset) as *mut u32, value);
}

/// UART0, configured for 115200 8N1 polled (non-interrupt-driven) output.
pub struct Uart0;

impl Uart0 {
    /// Resets and (re)initialises UART0 to 115200 8N1, independent of
    /// whatever state U-Boot left it in.
    ///
    /// # Safety
    ///
    /// Must not run concurrently with any other access to the UART0
    /// registers (true of every caller during the single-core, single-
    /// threaded boot path this crate controls).
    pub unsafe fn init() -> Self {
        write_reg(SYSC, SYSC_SOFTRESET);
        while read_reg(SYSS) & SYSS_RESETDONE == 0 {}

        write_reg(MDR1, MDR1_DISABLE); // TRM: mode must be disabled before changing baud/LCR

        write_reg(LCR, LCR_DLAB);
        write_reg(THR_RHR_DLL, BAUD_115200_DIVISOR & 0xFF);
        write_reg(IER_DLH, (BAUD_115200_DIVISOR >> 8) & 0xFF);

        write_reg(LCR, LCR_8N1); // also clears DLAB, back to normal register access
        write_reg(FCR_IIR, FCR_FIFO_EN_CLEAR);
        write_reg(MCR, 0); // no flow control

        write_reg(MDR1, MDR1_UART_16X); // re-enable normal UART operation

        Uart0
    }

    /// Blocks until THR is empty, then writes one byte.
    pub fn write_byte(&mut self, byte: u8) {
        unsafe {
            while read_reg(LSR) & LSR_THRE == 0 {}
            write_reg(THR_RHR_DLL, byte as u32);
        }
    }

    /// Writes a `\n` as `\r\n` (standard terminal convention), everything
    /// else byte-for-byte.
    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }
}

impl fmt::Write for Uart0 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        Uart0::write_str(self, s);
        Ok(())
    }
}
