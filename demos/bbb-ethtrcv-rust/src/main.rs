#![no_std]
#![no_main]

#![allow(non_snake_case)]

mod startup;
mod uart;
mod mdio;
mod lan8710;
mod ethtrcv;

use core::panic::PanicInfo;
use uart::Uart0;


/*
 * AUTOSAR Std_Types (AUTOSAR_SWS_StandardTypes)
 */
#[allow(non_camel_case_types)]
type Std_ReturnType = u8;

const E_OK:     Std_ReturnType = 0;
#[allow(dead_code)]
const E_NOT_OK: Std_ReturnType = 1;


/*
 * EthTrcv_ModeType (AUTOSAR_SWS_EthernetTransceiverDriver)
 *
 * ETHTRCV_MODE_DOWN   = 0
 * ETHTRCV_MODE_ACTIVE = 1
 */
#[repr(u8)]
#[derive(Copy, Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum EthTrcv_ModeType {
    ETHTRCV_MODE_DOWN   = 0,
    ETHTRCV_MODE_ACTIVE = 1,
}


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
        EthTrcv_Init(core::ptr::null());


        uart.write_str(
            "EthTrcv_Init() done\r\n"
        );


        /*
         * Test the exported C ABI.
         */
        let result =
            EthTrcv_SetTransceiverMode(
                0,
                EthTrcv_ModeType::ETHTRCV_MODE_ACTIVE,
            );


        if result == E_OK {

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
 * EthTrcv_Init
 *
 * AUTOSAR SWS_EthTrcv_00080
 *
 * Initializes the Ethernet Transceiver Driver.
 *
 * void EthTrcv_Init(const EthTrcv_ConfigType* CfgPtr)
 *
 * CfgPtr is ignored in this static-config PoC.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EthTrcv_Init(
    _CfgPtr: *const core::ffi::c_void,
) {
    let _ = ethtrcv::ethtrcv_init();
}


/*
 * EthTrcv_SetTransceiverMode
 *
 * AUTOSAR SWS_EthTrcv_00020
 *
 * Std_ReturnType EthTrcv_SetTransceiverMode(
 *     uint8             TrcvIdx,
 *     EthTrcv_ModeType  CtrlMode
 * )
 *
 * TrcvIdx  [in]  Index of the transceiver (0 = LAN8710A on BBB)
 * CtrlMode [in]  ETHTRCV_MODE_DOWN | ETHTRCV_MODE_ACTIVE
 *
 * return E_OK     operation successful
 * return E_NOT_OK operation failed or invalid parameter
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EthTrcv_SetTransceiverMode(
    TrcvIdx:  u8,
    CtrlMode: EthTrcv_ModeType,
) -> Std_ReturnType {

    ethtrcv::ethtrcv_set_mode(
        TrcvIdx,
        CtrlMode as u8,
    )
}
