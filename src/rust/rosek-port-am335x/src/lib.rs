//! HAL implementation for the TI AM335x SoC family (Cortex-A8, ARMv7-A).
//!
//! Register-Layout, Interrupt-Controller-Typ, MMU-Aufbau — see PLAN.md
//! "Portierbarkeit: Core / Port / Derivat" and
//! docs/arc42/05-bausteinsicht.adoc for this crate's exact scope.
//!
//! Filled in incrementally per PLAN.md "Meilensteine" Phase 0:
//! [`startup`] (vector table + per-CPU-mode stacks, issue #1), [`mmu`]
//! (flat identity mapping, issue #2), [`uart`] (UART0 console, issue #3)
//! and [`gpio`] (USR-LED life sign, issue #4) so far.

#![no_std]
#![feature(asm_const)]

pub mod gpio;
pub mod mmu;
pub mod startup;
pub mod uart;
