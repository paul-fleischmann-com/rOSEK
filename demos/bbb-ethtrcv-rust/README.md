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

## Exported C ABI

    void EthTrcv_Init(void);

    uint8_t EthTrcv_SetTransceiverMode(
        uint8_t trcv_idx,
        uint8_t mode
    );

PoC values:

    trcv_idx 0 = onboard LAN8710A

    mode 0 = DOWN
    mode 1 = ACTIVE

    return 0 = E_OK
    return 1 = E_NOT_OK

These types are intentionally simplified and are not yet
the final AUTOSAR 2025 type definitions.

## Raw binary

If arm-none-eabi-objcopy is installed:

    arm-none-eabi-objcopy \
        -O binary \
        target/armv7a-none-eabihf/release/bbb-ethtrcv-rust \
        bbb-ethtrcv-rust.bin

## Hardware assumptions

The linker assumes that U-Boot has loaded the image into DDR
starting at 0x80000000.

This is not a complete AM3358 cold-boot/SPL implementation.

The MDIO divider value currently used by the PoC is:

    0x7D

This must be verified against the actual AM3358 clock tree
before treating the MDIO implementation as production code.

## Next steps

1. Verify UART0 on hardware.
2. Verify AM3358 MDIO clock configuration.
3. Read LAN8710A PHYID1/PHYID2.
4. Implement GetTransceiverMode.
5. Implement GetLinkState.
6. Implement PHY identifier API.
7. Map the API to AUTOSAR Classic EthTrcv 2025.
8. Add C header and integration test.
