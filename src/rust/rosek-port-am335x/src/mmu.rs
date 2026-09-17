//! 1:1 (flat) MMU mapping for the AM335x, via ARMv7-A short-descriptor
//! section (1 MiB) entries. Issue #2 — see PLAN.md "Meilensteine" Phase 0
//! and docs/req42/04-product-backlog.adoc EPIC 0/STORY 0.2.1: "damit die
//! späteren Memory-Protection-Regionen (Phase 2) auf einer bereits aktiven
//! MMU aufbauen, statt sie nachträglich einzuführen."
//!
//! ## Table format
//!
//! One 4096-entry, 16 KiB-aligned L1 table of 1 MiB section descriptors
//! covers the entire 4 GiB address space identity-mapped (`TTBCR.N` is left
//! at its reset value of 0, so translation always uses `TTBR0` — no split
//! table needed for a single flat mapping). Each entry's physical base
//! equals its virtual base (`i << 20`), so this changes no addresses,
//! only adds permission/domain checking and, where enabled, caching.
//!
//! ## Memory types
//!
//! `SCTLR.TRE` (TEX remap) is left at its reset value of 0, so `PRRR`/`NMRR`
//! are not in play and the direct (non-remapped) `TEX`/`C`/`B` encodings
//! from the ARMv7-A ARM (table B3-10) apply:
//!
//! - The AM335x's DDR window (`0x8000_0000`..`0xC000_0000`, 1 GiB per the
//!   AM335x TRM/TI's own memory-map headers — physical BeagleBone Black
//!   RAM populates only the first 512 MiB of it, the rest of the section
//!   range is simply unused, mapping it costs nothing) is Normal,
//!   Write-Back Write-Allocate cacheable memory (`TEX=001,C=1,B=1`).
//! - Everything else is Shareable Device memory (`TEX=000,C=0,B=1`) — the
//!   safe default for MMIO/peripheral windows and anything not explicitly
//!   known to be RAM: Device memory forbids reordering, speculation and
//!   caching, which peripheral registers with side effects require.
//!
//! Access permission is full read/write at any privilege level
//! (`AP[1:0]=11`, `APX=0`) and every section uses domain 0, which the
//! Domain Access Control Register (DACR) sets to Client (permission-checked
//! by the AP bits above, not bypassed) — finer per-OS-Application domains
//! are Phase 2's job, not this one's.
//!
//! `enable_flat_mapping` also turns on the data and instruction caches
//! (`SCTLR.C`/`SCTLR.I`): a "flat mapping" that leaves both caches off
//! behaves almost identically to MMU-off and defers the actual point of
//! this step, which is to have a cached-and-mapped baseline that Phase 2
//! narrows into per-OS-Application regions instead of introducing caching
//! *and* protection at the same time later.

use core::arch::asm;

const SECTION_SHIFT: u32 = 20; // 1 MiB per section, 2^20 bytes
const NUM_SECTIONS: usize = 4096; // 4096 * 1 MiB = 4 GiB, the full address space

const DESC_SECTION: u32 = 0b10; // bits[1:0]: section (not page-table/fault/supersection)
const AP_FULL_ACCESS: u32 = 0b11 << 10; // AP[1:0]=11, APX(bit9)=0: full R/W, PL1 and PL0
const DOMAIN0: u32 = 0 << 5;

/// Normal memory, Write-Back Write-Allocate cacheable: TEX=0b001, C=1, B=1.
const MEM_NORMAL_CACHED: u32 = (0b001 << 12) | (1 << 3) | (1 << 2);
/// Shareable Device memory: TEX=0b000, C=0, B=1.
#[allow(clippy::identity_op)] // `0b000 << 12` spells out TEX=0 to match the ARM ARM table, not dead code
const MEM_DEVICE: u32 = (0b000 << 12) | (1 << 2);

/// AM335x DDR window per the TRM / TI memory-map headers: 1 GiB at
/// `0x8000_0000`. Expressed in section (1 MiB) units for the fill loop
/// below.
const DDR_SECTION_RANGE: core::ops::Range<usize> =
    (0x8000_0000usize >> SECTION_SHIFT)..(0xC000_0000usize >> SECTION_SHIFT);

#[repr(align(16384))]
struct TranslationTable([u32; NUM_SECTIONS]);

static mut L1_TABLE: TranslationTable = TranslationTable([0; NUM_SECTIONS]);

/// Builds the flat (identity) section-mapped L1 table described above and
/// enables the MMU together with the data/instruction caches.
///
/// # Safety
///
/// Must run with interrupts masked (true of every caller during the boot
/// path this crate controls) and be called at most once — it unconditionally
/// (re)writes the single static `L1_TABLE` and (re)points `TTBR0` at it.
pub unsafe fn enable_flat_mapping() {
    let table: *mut [u32; NUM_SECTIONS] = core::ptr::addr_of_mut!(L1_TABLE.0);
    for i in 0..NUM_SECTIONS {
        let base = (i as u32) << SECTION_SHIFT;
        let mem_type = if DDR_SECTION_RANGE.contains(&i) {
            MEM_NORMAL_CACHED
        } else {
            MEM_DEVICE
        };
        (*table)[i] = base | mem_type | AP_FULL_ACCESS | DOMAIN0 | DESC_SECTION;
    }

    let ttbr0 = table as u32;
    asm!(
        "dsb",                            // ensure the table writes above are visible before use
        "mcr p15, 0, {zero}, c8, c7, 0",   // TLBIALL: invalidate entire unified TLB
        "mcr p15, 0, {ttbr0}, c2, c0, 0",  // TTBR0 = &L1_TABLE
        "mcr p15, 0, {client}, c3, c0, 0", // DACR: domain 0 = Client (AP-bits enforced)
        "mrc p15, 0, {sctlr}, c1, c0, 0",
        "orr {sctlr}, {sctlr}, {mmu_c_i}", // set SCTLR.M | SCTLR.C | SCTLR.I
        "mcr p15, 0, {sctlr}, c1, c0, 0",
        "isb",
        zero = in(reg) 0u32,
        ttbr0 = in(reg) ttbr0,
        client = in(reg) 0b01u32, // DACR domain 0 field = Client
        sctlr = out(reg) _,
        mmu_c_i = in(reg) (1u32 | (1 << 2) | (1 << 12)), // M | C | I
    );
}
