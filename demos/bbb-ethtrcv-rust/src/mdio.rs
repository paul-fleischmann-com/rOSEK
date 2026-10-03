use core::ptr::{read_volatile, write_volatile};


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


const CM_PER_BASE: usize =
    0x44E0_0000;

const CM_PER_CPGMAC0_CLKCTRL: usize =
    CM_PER_BASE + 0x14;

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

        /*
         * Enable CPGMAC0 (CPSW/MDIO) clock via CM_PER.
         * Without this the MDIO peripheral has no clock and
         * every register access hangs indefinitely.
         */
        write_volatile(
            CM_PER_CPGMAC0_CLKCTRL as *mut u32,
            0x02,
        );

        loop {
            let idlest =
                read_volatile(
                    CM_PER_CPGMAC0_CLKCTRL as *const u32,
                );

            if idlest & (0x3 << 16) == 0 {
                break;
            }
        }

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
