/*
 * Startup code for SHA-256 benchmark
 * Sets up stack and calls sha256_benchmark, then exits via ecall
 */

.section .text
.global _start

_start:
    /* Set up stack pointer at top of memory (0x7000) */
    lui     sp, 0x7
    addi    sp, sp, 0

    /* Call sha256_benchmark - result will be in a0 */
    jal     ra, sha256_benchmark

    /* Exit via ecall with result in a0 */
    ecall
