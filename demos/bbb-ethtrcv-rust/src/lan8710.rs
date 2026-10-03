use crate::mdio::Mdio;


pub const LAN8710A_PHY_ADDR: u8 = 0;


pub const REG_BMCR: u8 = 0x00;
#[allow(dead_code)]
pub const REG_BMSR: u8 = 0x01;
#[allow(dead_code)]
pub const REG_PHYID1: u8 = 0x02;
#[allow(dead_code)]
pub const REG_PHYID2: u8 = 0x03;


pub const BMCR_RESET: u16 =
    1 << 15;

pub const BMCR_AUTONEG_ENABLE: u16 =
    1 << 12;

pub const BMCR_POWER_DOWN: u16 =
    1 << 11;

pub const BMCR_RESTART_AUTONEG: u16 =
    1 << 9;


#[allow(dead_code)]
pub const BMSR_AUTONEG_COMPLETE: u16 =
    1 << 5;

#[allow(dead_code)]
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


    #[allow(dead_code)]
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


    #[allow(dead_code)]
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
