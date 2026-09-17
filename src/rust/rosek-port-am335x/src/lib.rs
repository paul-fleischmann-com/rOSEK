//! HAL implementation for the TI AM335x SoC family (Cortex-A8, ARMv7-A).
//!
//! Register-Layout, Interrupt-Controller-Typ, MMU-Aufbau — see PLAN.md
//! "Portierbarkeit: Core / Port / Derivat" and
//! docs/arc42/05-bausteinsicht.adoc for this crate's exact scope.
//!
//! Filled in incrementally per PLAN.md "Meilensteine" Phase 0:
//! [`startup`] (vector table + per-CPU-mode stacks, issue #1) and [`mmu`]
//! (flat identity mapping, issue #2) so far. UART and GPIO follow as their
//! own issues (#3-#4).

#![no_std]
#![feature(asm_const)]

pub mod mmu;
pub mod startup;
