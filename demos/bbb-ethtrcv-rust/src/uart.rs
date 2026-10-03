use core::ptr::{read_volatile, write_volatile};

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
