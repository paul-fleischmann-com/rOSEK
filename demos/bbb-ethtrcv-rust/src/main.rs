#![no_std]
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
