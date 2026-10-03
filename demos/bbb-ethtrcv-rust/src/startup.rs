use core::arch::global_asm;

global_asm!(
    r#"
    .section .text._start
    .global _start
    .type _start, %function

_start:

    /* Disable interrupts */
    cpsid if

    /* Enable VFP/NEON (required for armv7a-none-eabihf ABI) */
    mrc p15, 0, r0, c1, c0, 2
    orr r0, r0, #(0xF << 20)
    mcr p15, 0, r0, c1, c0, 2
    isb
    vmrs r0, fpexc
    orr r0, r0, #(1 << 30)
    vmsr fpexc, r0

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
