use crate::lan8710::{
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
#[allow(dead_code)]
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


    #[allow(dead_code)]
    pub fn mode(
        &self,
    ) -> EthTrcvMode {
        self.mode
    }


    #[allow(dead_code)]
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
    (*core::ptr::addr_of_mut!(ETH_TRCV)).init()
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


    match (*core::ptr::addr_of_mut!(ETH_TRCV)).set_mode(mode) {

        Ok(()) =>
            0,

        Err(()) =>
            1,
    }
}
