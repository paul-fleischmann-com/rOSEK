ENTRY(_start)

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
