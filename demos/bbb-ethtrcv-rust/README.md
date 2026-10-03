# BBB Rust EthTrcv PoC

Bare-metal Rust proof of concept for an AUTOSAR-like EthTrcv
implementation on the BeagleBone Black / TI AM3358.

## Architecture

C / AUTOSAR
    |
    | EthTrcv_SetTransceiverMode()
    v
Rust C ABI
    |
EthTrcv
    |
LAN8710A
    |
MDIO
    |
AM3358
    |
LAN8710A PHY

## Files

src/startup.rs
    ARM startup, stack setup and BSS initialization.

src/uart.rs
    AM3358 UART0 debug output.

src/mdio.rs
    AM3358 MDIO controller.

src/lan8710.rs
    LAN8710A PHY driver.

src/ethtrcv.rs
    EthTrcv abstraction and global instance.

src/main.rs
    Rust entry point and exported C ABI.

linker.ld
    DDR linker layout.

.cargo/config.toml
    Rust ARM target configuration.

## Target

    armv7a-none-eabihf

Install:

    rustup target add armv7a-none-eabihf

Build:

    cargo build --release

## C Header (automatisch generiert)

`include/EthTrcv.h` wird bei jedem `cargo build` automatisch aus dem
Rust-Code generiert. Niemals manuell editieren.

    Std_ReturnType EthTrcv_SetTransceiverMode(
        uint8_t          TrcvIdx,
        EthTrcv_ModeType CtrlMode   /* ETHTRCV_MODE_DOWN | ETHTRCV_MODE_ACTIVE */
    );

    void EthTrcv_Init(const void *CfgPtr);

    #define E_OK     0
    #define E_NOT_OK 1

Rust ist die **Quelle der Wahrheit**. cbindgen liest den Rust-AST und
schreibt den C-Header — C und Rust können nie auseinanderlaufen.

### Warum cbindgen und nicht bindgen?

`bindgen` geht die umgekehrte Richtung: C-Header → Rust-FFI-Deklarationen
(extern "C" Blöcke). Das ist richtig wenn eine bestehende C-Bibliothek
aus Rust aufgerufen werden soll. Hier implementiert Rust den BSW-Stack und
C ist der Aufrufer — deshalb cbindgen.

| Richtung          | Tool       | Anwendungsfall                              |
|-------------------|------------|---------------------------------------------|
| Rust → C Header   | cbindgen   | Rust ist Implementierung, C ruft auf        |
| C Header → Rust   | bindgen    | Bestehende C-Bibliothek aus Rust aufrufen   |

## Raw binary

    make

Erzeugt via `rust-objcopy`:

    target/armv7a-none-eabihf/release/bbb-ethtrcv-rust.bin

## Hardware assumptions

The linker assumes that U-Boot has loaded the image into DDR
starting at 0x80000000.

This is not a complete AM3358 cold-boot/SPL implementation.

The MDIO divider value currently used by the PoC is:

    0x7D

This must be verified against the actual AM3358 clock tree
before treating the MDIO implementation as production code.

## Rust vs. C für Embedded Treiber

### Vorteile Rust

**Memory Safety ohne Runtime-Overhead**
Der Borrow-Checker verhindert Use-after-free, dangling Pointer und
Data Races zur Compile-Zeit — ohne Garbage Collector. Gerade bei
MMIO-Registern und Interrupt-Handlern kritisch.

**Typsystem erzwingt Korrektheit**
`#[repr(u8)]` Enums machen ungültige Registerwerte undarstellbar.
`Result<T, E>` erzwingt Fehlerbehandlung — ein vergessenes Returncode-
Check ist ein Compile-Fehler, kein stiller Bug.

**Keine impliziten Konvertierungen**
`u8 as u32` ist explizit. In C entstehen durch integer promotion und
signed/unsigned-Mischung regelmäßig subtile Bugs in Registeroperationen.

**cbindgen: C-Header garantiert synchron**
Die C-Schnittstelle wird aus dem Rust-Code generiert — Header und
Implementierung können nie divergieren.

**Modernes Tooling**
`cargo`, integrierte Tests, `rust-analyzer`, reproducible Builds out of the
box.

### Nachteile Rust

**Lernkurve**
Ownership, Lifetimes und `unsafe` erfordern Einarbeitung — besonders für
Teams mit reinem C-Hintergrund.

**`unsafe` nicht vermeidbar**
MMIO, Startup-Code und direkte Registeroperationen brauchen `unsafe`.
Der Sicherheitsgewinn liegt darin, `unsafe` auf kleine, klar abgegrenzte
Blöcke zu beschränken — nicht darin, es ganz zu eliminieren.

**Toolchain-Komplexität**
Cross-Compilation (rustup targets, linker flags, cbindgen) ist aufwändiger
als ein einzelner `arm-none-eabi-gcc` Aufruf. Für neue Targets fehlen
manchmal `core`-Prebuilds.

**AUTOSAR-Toolketten kennen kein Rust**
Klassische AUTOSAR-Generatoren (EB tresos, Vector DaVinci) erzeugen C.
Integration bedeutet immer eine C-ABI-Grenze plus manuell gepflegtes
Makefile/CMake.

**Binärgröße / LTO**
Bei `opt-level = "s"` und `lto = true` vergleichbar mit C. Ohne LTO kann
das Rust-Binary durch Monomorphisierung größer werden.

## Next steps

1. Verify UART0 on hardware.
2. Verify AM3358 MDIO clock configuration.
3. Read LAN8710A PHYID1/PHYID2.
4. Implement GetTransceiverMode.
5. Implement GetLinkState.
6. Implement PHY identifier API.
7. Map the API to AUTOSAR Classic EthTrcv 2025.
8. Add C header and integration test.
