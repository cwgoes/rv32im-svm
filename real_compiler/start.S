/*
 * Startup code for Real Compiler benchmark
 * Sets up stack and calls real_compiler_benchmark, then exits via ecall
 */

.section .text
.global _start

_start:
    /* Set up stack pointer at top of memory (0x7000) */
    lui     sp, 0x7
    addi    sp, sp, 0

    /* Call real_compiler_benchmark - result will be in a0 */
    jal     ra, real_compiler_benchmark

    /* Exit via ecall with result in a0 */
    ecall
