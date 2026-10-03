#!/usr/bin/env python3

from pathlib import Path

PROJECT = Path("bbb-ethtrcv-rust")


FILES = {
    "Cargo.toml": r'''[package]
name = "bbb-ethtrcv-rust"
version = "0.1.0"
edition = "2021"

[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
panic = "abort"
''',

    ".cargo/config.toml": r'''[build]
target = "armv7a-none-eabihf"

[target.armv7a-none-eabihf]
rustflags = [
    "-C", "link-arg=-Tlinker.ld"
]
''',

    "linker.ld": r'''ENTRY(_start)

MEMORY
{
    RAM (rwx) : ORIGIN = 0x80000000, LENGTH = 128M
}

SECTIONS
{
    .text :
    {
        . = ALIGN(4);
        KEEP(*(.text._start))
        *(.text*)
    } > RAM

    .rodata :
    {
        . = ALIGN(4);
        *(.rodata*)
    } > RAM

    .data :
    {
        . = ALIGN(4);
        *(.data*)
    } > RAM

    .bss (NOLOAD) :
    {
        . = ALIGN(4);

        __bss_start = .;

        *(.bss*)
        *(COMMON)

        . = ALIGN(4);

        __bss_end = .;
    } > RAM

    . = ALIGN(8);

    __stack_bottom = .;

    . += 16K;

    __stack_top = .;
}
''',

    "src/startup.rs": r'''use core::arch::global_asm;

global_asm!(
    r#"
    .section .text._start
    .global _start
    .type _start, %function

_start:

    /* Disable interrupts */
    cpsid if

    /* Load stack pointer */
    ldr sp, =__stack_top

    /* Clear BSS */
    ldr r0, =__bss_start
    ldr r1, =__bss_end

1:
    cmp r0, r1
    bge 2f

    mov r2, #0
    str r2, [r0], #4

    b 1b

2:
    /* Enter Rust */
    bl rust_main

3:
    b 3b

    .size _start, .-_start
"#
);
''',

    "src/uart.rs": r'''use core::ptr::{read_volatile, write_volatile};

const UART0_BASE: usize = 0x44E0_9000;

const THR: usize = UART0_BASE + 0x00;
const DLL: usize = UART0_BASE + 0x00;
const DLH: usize = UART0_BASE + 0x04;
const IER: usize = UART0_BASE + 0x04;
const FCR: usize = UART0_BASE + 0x08;
const LCR: usize = UART0_BASE + 0x0C;
const LSR: usize = UART0_BASE + 0x14;
const MDR1: usize = UART0_BASE + 0x20;

const LSR_THRE: u8 = 1 << 5;

const CONTROL_MODULE: usize = 0x44E1_0000;

const UART0_RXD_MUX: usize = CONTROL_MODULE + 0x170;
const UART0_TXD_MUX: usize = CONTROL_MODULE + 0x174;

const UART_CLOCK_HZ: u32 = 48_000_000;
const BAUDRATE: u32 = 115_200;

const LCR_8N1: u8 = 0x03;
const LCR_DLAB: u8 = 1 << 7;

const MDR1_MODE_DISABLED: u8 = 0x07;
const MDR1_MODE_UART_16X: u8 = 0x00;


pub struct Uart0;


impl Uart0 {

    pub const fn new() -> Self {
        Self
    }


    pub unsafe fn init(&mut self) {

        /*
         * UART0 RXD
         *
         * MODE0
         * RX active
         * pull-up enabled
         */
        write_volatile(
            UART0_RXD_MUX as *mut u32,
            (1 << 5) | (1 << 4),
        );


        /*
         * UART0 TXD
         *
         * MODE0
         */
        write_volatile(
            UART0_TXD_MUX as *mut u32,
            0,
        );


        /*
         * Disable UART while
         * configuring baud rate.
         */
        write_volatile(
            MDR1 as *mut u8,
            MDR1_MODE_DISABLED,
        );


        /*
         * 8 data bits
         * no parity
         * 1 stop bit
         */
        write_volatile(
            LCR as *mut u8,
            LCR_DLAB | LCR_8N1,
        );


        /*
         * Baud divisor:
         *
         * 48 MHz / (16 * 115200)
         * = 26.041...
         */
        let divisor =
            UART_CLOCK_HZ / (16 * BAUDRATE);


        write_volatile(
            DLL as *mut u8,
            (divisor & 0xff) as u8,
        );


        write_volatile(
            DLH as *mut u8,
            ((divisor >> 8) & 0xff) as u8,
        );


        /*
         * Leave divisor-latch mode.
         */
        write_volatile(
            LCR as *mut u8,
            LCR_8N1,
        );


        /*
         * Enable/reset FIFOs.
         */
        write_volatile(
            FCR as *mut u8,
            0x07,
        );


        /*
         * Disable UART interrupts.
         */
        write_volatile(
            IER as *mut u8,
            0x00,
        );


        /*
         * UART 16x mode.
         */
        write_volatile(
            MDR1 as *mut u8,
            MDR1_MODE_UART_16X,
        );
    }


    pub unsafe fn write_byte(
        &mut self,
        byte: u8,
    ) {
        while read_volatile(
            LSR as *const u8
        ) & LSR_THRE == 0
        {}

        write_volatile(
            THR as *mut u8,
            byte,
        );
    }


    pub unsafe fn write_str(
        &mut self,
        text: &str,
    ) {
        for byte in text.bytes() {

            if byte == b'\n' {
                self.write_byte(b'\r');
            }

            self.write_byte(byte);
        }
    }
}
''',

    "src/mdio.rs": r'''use core::ptr::{read_volatile, write_volatile};


const CONTROL_MODULE: usize =
    0x44E1_0000;


const MDIO_DATA_MUX: usize =
    CONTROL_MODULE + 0x148;

const MDIO_CLK_MUX: usize =
    CONTROL_MODULE + 0x14C;


const MDIO_DATA_CONFIG: u32 =
    (1 << 5) | (1 << 4);


const MDIO_CLK_CONFIG: u32 =
    (1 << 5) | (1 << 4);


const MDIO_BASE: usize =
    0x4A10_1000;


const MDIO_CONTROL: usize =
    MDIO_BASE + 0x04;


const MDIO_USERACCESS0: usize =
    MDIO_BASE + 0x80;


const CONTROL_ENABLE: u32 =
    1 << 30;


const USERACCESS_GO: u32 =
    1 << 31;

const USERACCESS_WRITE: u32 =
    1 << 30;

const USERACCESS_ACK: u32 =
    1 << 29;


pub struct Mdio {
    _private: (),
}


impl Mdio {

    pub const fn new() -> Self {
        Self { _private: () }
    }


    pub unsafe fn init(&mut self) {

        write_volatile(
            MDIO_DATA_MUX as *mut u32,
            MDIO_DATA_CONFIG,
        );


        write_volatile(
            MDIO_CLK_MUX as *mut u32,
            MDIO_CLK_CONFIG,
        );


        /*
         * PoC divider.
         *
         * Must be verified against the
         * actual AM3358 clock configuration.
         */
        write_volatile(
            MDIO_CONTROL as *mut u32,
            CONTROL_ENABLE | 0x7D,
        );
    }


    pub unsafe fn read(
        &mut self,
        phy_addr: u8,
        reg_addr: u8,
    ) -> Result<u16, ()> {

        let command =
            USERACCESS_GO
            | ((reg_addr as u32 & 0x1F) << 21)
            | ((phy_addr as u32 & 0x1F) << 16);


        write_volatile(
            MDIO_USERACCESS0 as *mut u32,
            command,
        );


        loop {

            let value =
                read_volatile(
                    MDIO_USERACCESS0 as *const u32
                );


            if value & USERACCESS_GO == 0 {

                if value & USERACCESS_ACK != 0 {
                    return Ok(
                        (value & 0xFFFF) as u16
                    );
                }

                return Err(());
            }
        }
    }


    pub unsafe fn write(
        &mut self,
        phy_addr: u8,
        reg_addr: u8,
        data: u16,
    ) -> Result<(), ()> {

        let command =
            USERACCESS_GO
            | USERACCESS_WRITE
            | ((reg_addr as u32 & 0x1F) << 21)
            | ((phy_addr as u32 & 0x1F) << 16)
            | data as u32;


        write_volatile(
            MDIO_USERACCESS0 as *mut u32,
            command,
        );


        loop {

            let value =
                read_volatile(
                    MDIO_USERACCESS0 as *const u32
                );


            if value & USERACCESS_GO == 0 {
                return Ok(());
            }
        }
    }
}
''',

    "src/lan8710.rs": r'''use crate::mdio::Mdio;


pub const LAN8710A_PHY_ADDR: u8 = 0;


pub const REG_BMCR: u8 = 0x00;
pub const REG_BMSR: u8 = 0x01;
pub const REG_PHYID1: u8 = 0x02;
pub const REG_PHYID2: u8 = 0x03;


pub const BMCR_RESET: u16 =
    1 << 15;

pub const BMCR_AUTONEG_ENABLE: u16 =
    1 << 12;

pub const BMCR_POWER_DOWN: u16 =
    1 << 11;

pub const BMCR_RESTART_AUTONEG: u16 =
    1 << 9;


pub const BMSR_AUTONEG_COMPLETE: u16 =
    1 << 5;

pub const BMSR_LINK_STATUS: u16 =
    1 << 2;


pub struct Lan8710a {
    mdio: Mdio,
    phy_addr: u8,
}


impl Lan8710a {

    pub const fn new(
        phy_addr: u8,
    ) -> Self {

        Self {
            mdio: Mdio::new(),
            phy_addr,
        }
    }


    pub unsafe fn init_mdio(&mut self) {
        self.mdio.init();
    }


    pub unsafe fn read_reg(
        &mut self,
        reg: u8,
    ) -> Result<u16, ()> {

        self.mdio.read(
            self.phy_addr,
            reg,
        )
    }


    pub unsafe fn write_reg(
        &mut self,
        reg: u8,
        value: u16,
    ) -> Result<(), ()> {

        self.mdio.write(
            self.phy_addr,
            reg,
            value,
        )
    }


    pub unsafe fn phy_id(
        &mut self,
    ) -> Result<u32, ()> {

        let id1 =
            self.read_reg(REG_PHYID1)?;

        let id2 =
            self.read_reg(REG_PHYID2)?;

        Ok(
            ((id1 as u32) << 16)
            | id2 as u32
        )
    }


    pub unsafe fn reset(
        &mut self,
    ) -> Result<(), ()> {

        self.write_reg(
            REG_BMCR,
            BMCR_RESET,
        )?;


        for _ in 0..1_000_000 {

            let bmcr =
                self.read_reg(REG_BMCR)?;


            if bmcr & BMCR_RESET == 0 {
                return Ok(());
            }
        }


        Err(())
    }


    pub unsafe fn set_active(
        &mut self,
    ) -> Result<(), ()> {

        let mut bmcr =
            self.read_reg(REG_BMCR)?;


        bmcr &= !BMCR_POWER_DOWN;

        bmcr |=
            BMCR_AUTONEG_ENABLE;

        bmcr |=
            BMCR_RESTART_AUTONEG;


        self.write_reg(
            REG_BMCR,
            bmcr,
        )
    }


    pub unsafe fn set_down(
        &mut self,
    ) -> Result<(), ()> {

        let mut bmcr =
            self.read_reg(REG_BMCR)?;


        bmcr |= BMCR_POWER_DOWN;


        self.write_reg(
            REG_BMCR,
            bmcr,
        )
    }


    pub unsafe fn link_up(
        &mut self,
    ) -> Result<bool, ()> {

        let bmsr =
            self.read_reg(REG_BMSR)?;


        Ok(
            (bmsr & BMSR_LINK_STATUS) != 0
        )
    }
}
''',

    "src/ethtrcv.rs": r'''use crate::lan8710::{
    Lan8710a,
    LAN8710A_PHY_ADDR,
};


#[repr(u8)]
#[derive(Copy, Clone, PartialEq)]
pub enum EthTrcvMode {
    Down = 0,
    Active = 1,
}


#[repr(u8)]
#[derive(Copy, Clone, PartialEq)]
pub enum EthTrcvLinkState {
    Down = 0,
    Up = 1,
}


pub struct EthTrcv {
    phy: Lan8710a,
    mode: EthTrcvMode,
}


impl EthTrcv {

    pub const fn new() -> Self {

        Self {
            phy: Lan8710a::new(
                LAN8710A_PHY_ADDR
            ),

            mode:
                EthTrcvMode::Down,
        }
    }


    pub unsafe fn init(
        &mut self,
    ) -> Result<(), ()> {

        self.phy.init_mdio();

        self.phy.reset()?;

        self.phy.set_active()?;

        self.mode =
            EthTrcvMode::Active;

        Ok(())
    }


    pub unsafe fn set_mode(
        &mut self,
        mode: EthTrcvMode,
    ) -> Result<(), ()> {

        match mode {

            EthTrcvMode::Down => {
                self.phy.set_down()?;
            }


            EthTrcvMode::Active => {
                self.phy.set_active()?;
            }
        }


        self.mode = mode;

        Ok(())
    }


    pub fn mode(
        &self,
    ) -> EthTrcvMode {
        self.mode
    }


    pub unsafe fn get_link_state(
        &mut self,
    ) -> Result<EthTrcvLinkState, ()> {

        if self.phy.link_up()? {

            Ok(
                EthTrcvLinkState::Up
            )

        } else {

            Ok(
                EthTrcvLinkState::Down
            )
        }
    }
}


/*
* Global EthTrcv instance.
*
* One BBB onboard LAN8710A PHY.
*/
static mut ETH_TRCV: EthTrcv =
    EthTrcv::new();


pub unsafe fn ethtrcv_init()
    -> Result<(), ()>
{
    ETH_TRCV.init()
}


pub unsafe fn ethtrcv_set_mode(
    trcv_idx: u8,
    mode: u8,
) -> u8 {

    /*
     * Only transceiver 0 exists
     * in this PoC.
     */
    if trcv_idx != 0 {
        return 1;
    }


    let mode =
        match mode {

            0 =>
                EthTrcvMode::Down,

            1 =>
                EthTrcvMode::Active,

            _ =>
                return 1,
        };


    match ETH_TRCV.set_mode(mode) {

        Ok(()) =>
            0,

        Err(()) =>
            1,
    }
}
''',

    "src/main.rs": r'''#![no_std]
#![no_main]

mod startup;
mod uart;
mod mdio;
mod lan8710;
mod ethtrcv;

use core::panic::PanicInfo;
use uart::Uart0;


#[panic_handler]
fn panic(
    _info: &PanicInfo,
) -> ! {

    loop {

        unsafe {
            core::arch::asm!("wfi");
        }
    }
}


/*
* Bare-metal Rust entry point.
*/
#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {

    let mut uart =
        Uart0::new();


    unsafe {

        uart.init();


        uart.write_str(
            "\r\n"
        );


        uart.write_str(
            "================================\r\n"
        );


        uart.write_str(
            "Rust AUTOSAR EthTrcv PoC\r\n"
        );


        uart.write_str(
            "================================\r\n"
        );


        /*
         * Initialize global EthTrcv.
         */
        EthTrcv_Init();


        uart.write_str(
            "EthTrcv_Init() done\r\n"
        );


        /*
         * Test the exported C ABI.
         *
         * 0 = DOWN
         * 1 = ACTIVE
         */
        let result =
            EthTrcv_SetTransceiverMode(
                0,
                1,
            );


        if result == 0 {

            uart.write_str(
                "EthTrcv_SetTransceiverMode(ACTIVE) OK\r\n"
            );

        } else {

            uart.write_str(
                "EthTrcv_SetTransceiverMode(ACTIVE) FAILED\r\n"
            );
        }
    }


    loop {

        unsafe {
            core::arch::asm!("wfi");
        }
    }
}


/*
* C ABI
*
* Equivalent conceptually to:
*
* void EthTrcv_Init(void);
*/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EthTrcv_Init() {

    let _ =
        ethtrcv::ethtrcv_init();
}


/*
* C ABI
*
* PoC:
*
* trcv_idx 0 = LAN8710A
* mode      0 = DOWN
* mode      1 = ACTIVE
*
* return    0 = E_OK
* return    1 = E_NOT_OK
*/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EthTrcv_SetTransceiverMode(
    trcv_idx: u8,
    mode: u8,
) -> u8 {

    ethtrcv::ethtrcv_set_mode(
        trcv_idx,
        mode,
    )
}
''',

    "README.md": r'''# BBB Rust EthTrcv PoC

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
'''
}


def write_project():
    print(f"Creating project: {PROJECT}")

    for filename, content in FILES.items():
        path = PROJECT / filename
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
        print(f"  created: {path}")

    print()
    print("Project created successfully.")
    print()
    print("Files:")
    for filename in FILES:
        print(f"  {PROJECT / filename}")


if __name__ == "__main__":
    write_project() 
