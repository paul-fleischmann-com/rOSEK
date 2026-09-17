//! Reset entry, exception vector table, and per-CPU-mode stack setup for the
//! AM335x (Cortex-A8, ARMv7-A). Runs before any other Rust code.
//!
//! ## Entry state (verified, not assumed)
//!
//! rOSEK is entered via u-boot's `go <addr>` command after loading the raw
//! binary into DDR — there is no reset vector fetch, u-boot branches to
//! `_start` directly. At that point the CPU is documented to be in SVC32
//! mode with the MMU and D-/I-caches disabled and interrupts not enabled
//! during the boot process (U-Boot ARM boot code analysis, e.g.
//! <https://blogs.coreboot.org/blog/2011/06/05/gsoc2011week-1-analysis-of-u-boot-arm-boot-code/>).
//! `_start` still explicitly masks IRQ/FIQ itself before doing anything else,
//! rather than trusting that state — cheap, and it removes any dependency on
//! a specific u-boot build's exact behaviour.
//!
//! ## Vector relocation via VBAR
//!
//! The AM335x's Cortex-A8 implements the Security Extensions, so the
//! Vector Base Address Register (VBAR, `c12, c0, 0`) can relocate the
//! exception vector table to wherever this binary is actually linked,
//! instead of requiring it to sit at the fixed low (0x00000000) or high
//! (0xFFFF0000) address. VBAR only takes effect when SCTLR.V (bit 13) is
//! clear ("low vectors" mode) — a set V bit forces the fixed high address
//! unconditionally. `_start` clears SCTLR.V before relying on VBAR.
//! VBAR itself must hold a 32-byte-aligned address, matching the required
//! alignment of the vector table (8 entries x 4 bytes = 32 bytes).
//!
//! Note that this VBAR-based relocation applies to exceptions *other than*
//! Reset — a genuine hardware Reset exception is not affected by VBAR and
//! always fetches from the fixed reset vector. That path is not exercised
//! by the u-boot `go` handoff this project targets; vector slot 0 is filled
//! in anyway for structural completeness (see below).
//!
//! ## Per-mode stacks
//!
//! ARM mode registers are banked per exception mode for `r13`(`sp`)/`r14`(`lr`)
//! with one exception: User (usr) and System (sys) modes *share* the same
//! banked `sp`/`lr` — they are not separately banked from each other. So
//! there are six independent stacks, not seven: `und`, `abt`, `irq`, `fiq`,
//! `svc`, and one shared `sys`/`usr` stack. `_start` switches through each
//! mode via `msr cpsr_c, #(mode | I_BIT | F_BIT)` (keeping interrupts masked
//! throughout, since nothing is set up to handle one yet) and loads that
//! mode's dedicated stack top, then ends in SVC mode — matching the
//! privilege level u-boot handed off in — before branching to `board_main`.
//! Interrupts are left masked on entry to `board_main`; unmasking them is a
//! later phase's job (needs an interrupt controller driver and real
//! handlers first, see PLAN.md "Meilensteine" Phase 3/4).
//!
//! The six stack-top symbols (`__stack_und_top`, `__stack_abt_top`,
//! `__stack_irq_top`, `__stack_fiq_top`, `__stack_svc_top`,
//! `__stack_sys_top`) are this module's contract with the linker script:
//! whichever board crate links against `rosek-port-am335x` must define
//! them (stack sizes are a board/memory-map concern, not a port concern).
//!
//! ## .bss zeroing
//!
//! `_start` also zeroes `.bss` (`__bss_start`..`__bss_end`, provided by the
//! board's linker script) before touching any mode stack. Strictly this
//! goes one step beyond "vector table + stacks", but any Rust code beyond
//! a bare `loop {}` needs well-defined `static`/`static mut` storage to be
//! sound, and every later Phase-0 issue adds exactly that - doing it here
//! once, in the one place that already owns "get to a safe Rust
//! environment", avoids every future issue needing to remember it.
//!
//! ## Contract with the board crate
//!
//! After stack setup, `_start` branches to `board_main`, an
//! `extern "C" fn() -> !` that the linking board crate must provide
//! (`#[no_mangle]`). This is deliberately the *only* thing this module does
//! after boot — it does not call into UART/LED/panic-handler code, since
//! those are separate, not-yet-implemented pieces (see PLAN.md "Meilensteine"
//! Phase 0, issues #2-#5).

use core::arch::global_asm;

/// ARM CPSR mode bits (`M[4:0]`), ARMv7-A Architecture Reference Manual.
mod cpsr_mode {
    pub const FIQ: u32 = 0x11;
    pub const IRQ: u32 = 0x12;
    pub const SVC: u32 = 0x13;
    pub const ABT: u32 = 0x17;
    pub const UND: u32 = 0x1B;
    pub const SYS: u32 = 0x1F;
}

/// CPSR I-bit (IRQ mask) and F-bit (FIQ mask).
const I_BIT: u32 = 1 << 7;
const F_BIT: u32 = 1 << 6;

global_asm!(
    r#"
.section .vector_table, "ax"
.global _vector_table
.align 5                           @ 32-byte alignment, required for VBAR

_vector_table:
    b   _start                     @ 0x00 Reset      (not actually reached via VBAR, see module docs)
    b   _undefined_instruction     @ 0x04 Undefined Instruction
    b   _software_interrupt        @ 0x08 Software Interrupt (SVC)
    b   _prefetch_abort            @ 0x0C Prefetch Abort
    b   _data_abort                @ 0x10 Data Abort
    b   _reserved                  @ 0x14 Reserved
    b   _irq                       @ 0x18 IRQ
    b   _fiq                       @ 0x1C FIQ

@ Placeholder handlers: parking loops. Real handling (protection-error
@ reaction, IRQ dispatch, ...) belongs to later phases (PLAN.md Phase 2-4),
@ not to this issue (#1: vector table + per-mode stacks only).
_undefined_instruction:  b _undefined_instruction
_software_interrupt:     b _software_interrupt
_prefetch_abort:         b _prefetch_abort
_data_abort:             b _data_abort
_reserved:               b _reserved
_irq:                    b _irq
_fiq:                    b _fiq

.section .text._start, "ax"
.global _start

_start:
    cpsid   if                              @ mask IRQ+FIQ; don't trust caller state

    ldr     r0, =_vector_table
    mcr     p15, 0, r0, c12, c0, 0           @ VBAR = &_vector_table

    mrc     p15, 0, r0, c1, c0, 0            @ r0 = SCTLR
    bic     r0, r0, #(1 << 13)               @ clear V: low vectors / VBAR-relocated, not fixed 0xFFFF0000
    mcr     p15, 0, r0, c1, c0, 0            @ SCTLR = r0

    @ Zero .bss (excluded from the raw-binary image, see linker.ld) so that
    @ Rust `static`/`static mut` data starts well-defined. Needs no stack.
    ldr     r0, =__bss_start
    ldr     r1, =__bss_end
    mov     r2, #0
zero_bss:
    cmp     r0, r1
    strlt   r2, [r0], #4
    blt     zero_bss

    msr     cpsr_c, #({und} | {i} | {f})
    ldr     sp, =__stack_und_top

    msr     cpsr_c, #({abt} | {i} | {f})
    ldr     sp, =__stack_abt_top

    msr     cpsr_c, #({irq} | {i} | {f})
    ldr     sp, =__stack_irq_top

    msr     cpsr_c, #({fiq} | {i} | {f})
    ldr     sp, =__stack_fiq_top

    msr     cpsr_c, #({sys} | {i} | {f})
    ldr     sp, =__stack_sys_top

    msr     cpsr_c, #({svc} | {i} | {f})
    ldr     sp, =__stack_svc_top

    bl      board_main
    b       .                                @ board_main is `-> !` and must not return; park if it somehow does
    "#,
    und = const cpsr_mode::UND,
    abt = const cpsr_mode::ABT,
    irq = const cpsr_mode::IRQ,
    fiq = const cpsr_mode::FIQ,
    svc = const cpsr_mode::SVC,
    sys = const cpsr_mode::SYS,
    i = const I_BIT,
    f = const F_BIT,
);
