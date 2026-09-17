/* Linker script for the BeagleBone Black (AM335x, 512 MiB DDR3 @ 0x80000000).
 *
 * Link/load address 0x80200000 matches the stock U-Boot `loadaddr` default
 * used on TI AM335x boards (BeagleBone Black included). Flash/boot with:
 *
 *   setenv loadaddr 0x80200000
 *   tftpboot ${loadaddr} rosek.bin   (or loadb / fatload from SD card)
 *   go ${loadaddr}
 *
 * rOSEK is loaded and run as a raw binary at its final address (no XIP,
 * no separate load-vs-run address) - .data needs no copy-from-flash step,
 * .bss is explicitly zeroed by rosek-port-am335x's startup code instead of
 * being included as zero bytes in the image (see NOLOAD below).
 */

MEMORY
{
    DDR (rwx) : ORIGIN = 0x80200000, LENGTH = 256M
}

/* Per-CPU-mode stack sizes. Generous defaults for Phase 0, where nothing
 * beyond an infinite loop runs yet - see rosek-port-am335x::startup for the
 * mode/stack contract these symbols satisfy. usr and sys modes share one
 * stack (banked r13/lr are shared between them), hence no separate
 * "usr" stack. */
STACK_SIZE_UND = 1K;
STACK_SIZE_ABT = 1K;
STACK_SIZE_IRQ = 4K;
STACK_SIZE_FIQ = 1K;
STACK_SIZE_SVC = 16K;
STACK_SIZE_SYS = 4K;

ENTRY(_start)

SECTIONS
{
    .vector_table : {
        KEEP(*(.vector_table))
    } > DDR

    .text : {
        *(.text._start)
        *(.text .text.*)
    } > DDR

    .rodata : {
        *(.rodata .rodata.*)
    } > DDR

    .data : {
        *(.data .data.*)
    } > DDR

    .bss (NOLOAD) : ALIGN(8) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    } > DDR

    .stacks (NOLOAD) : ALIGN(8) {
        . = ALIGN(8);
        . += STACK_SIZE_UND;
        __stack_und_top = .;

        . = ALIGN(8);
        . += STACK_SIZE_ABT;
        __stack_abt_top = .;

        . = ALIGN(8);
        . += STACK_SIZE_IRQ;
        __stack_irq_top = .;

        . = ALIGN(8);
        . += STACK_SIZE_FIQ;
        __stack_fiq_top = .;

        . = ALIGN(8);
        . += STACK_SIZE_SVC;
        __stack_svc_top = .;

        . = ALIGN(8);
        . += STACK_SIZE_SYS;
        __stack_sys_top = .;
    } > DDR

    /DISCARD/ : {
        *(.ARM.exidx*)
        *(.ARM.extab*)
        *(.comment)
    }
}
