/*
 * Mini rv32im Compiler - A minimal compiler that runs on SVM
 *
 * This is a simple rv32im instruction encoder that demonstrates
 * running a compiler compiled to rv32im, then to SVM.
 *
 * It compiles a simple factorial function to rv32im machine code.
 */

/* rv32im instruction encoding helpers */

/* R-type: funct7[31:25] | rs2[24:20] | rs1[19:15] | funct3[14:12] | rd[11:7] | opcode[6:0] */
static unsigned int encode_r(unsigned int opcode, unsigned int rd, unsigned int funct3,
                             unsigned int rs1, unsigned int rs2, unsigned int funct7) {
    return (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode;
}

/* I-type: imm[31:20] | rs1[19:15] | funct3[14:12] | rd[11:7] | opcode[6:0] */
static unsigned int encode_i(unsigned int opcode, unsigned int rd, unsigned int funct3,
                             unsigned int rs1, int imm) {
    return ((imm & 0xFFF) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode;
}

/* B-type: imm[12|10:5] | rs2 | rs1 | funct3 | imm[4:1|11] | opcode */
static unsigned int encode_b(unsigned int opcode, unsigned int funct3,
                             unsigned int rs1, unsigned int rs2, int imm) {
    unsigned int imm12 = (imm >> 12) & 1;
    unsigned int imm10_5 = (imm >> 5) & 0x3F;
    unsigned int imm4_1 = (imm >> 1) & 0xF;
    unsigned int imm11 = (imm >> 11) & 1;
    return (imm12 << 31) | (imm10_5 << 25) | (rs2 << 20) | (rs1 << 15) |
           (funct3 << 12) | (imm4_1 << 8) | (imm11 << 7) | opcode;
}

/* Opcodes */
#define OP_IMM    0x13
#define OP_OP     0x33
#define OP_BRANCH 0x63
#define OP_SYSTEM 0x73

/* Registers */
#define ZERO 0
#define A0   10
#define A1   11
#define A2   12

/* ADDI rd, rs1, imm */
static unsigned int addi(unsigned int rd, unsigned int rs1, int imm) {
    return encode_i(OP_IMM, rd, 0, rs1, imm);
}

/* ADD rd, rs1, rs2 */
static unsigned int add(unsigned int rd, unsigned int rs1, unsigned int rs2) {
    return encode_r(OP_OP, rd, 0, rs1, rs2, 0);
}

/* MUL rd, rs1, rs2 */
static unsigned int mul(unsigned int rd, unsigned int rs1, unsigned int rs2) {
    return encode_r(OP_OP, rd, 0, rs1, rs2, 1);
}

/* BLT rs1, rs2, offset */
static unsigned int blt(unsigned int rs1, unsigned int rs2, int offset) {
    return encode_b(OP_BRANCH, 4, rs1, rs2, offset);
}

/* ECALL */
static unsigned int ecall(void) {
    return 0x00000073;
}

/* Output buffer for compiled code */
static unsigned int output[64];
static int output_len = 0;

static void emit(unsigned int inst) {
    if (output_len < 64) {
        output[output_len++] = inst;
    }
}

/*
 * Compile factorial function to rv32im
 *
 * Factorial in pseudo-assembly:
 *   li a1, 10        # n = 10
 *   li a0, 1         # result = 1
 *   li a2, 2         # constant 2
 * loop:
 *   blt a1, a2, done # if n < 2, exit
 *   mul a0, a0, a1   # result *= n
 *   addi a1, a1, -1  # n--
 *   j loop           # encoded as blt zero, zero (always false, so always jumps? no...)
 * done:
 *   ecall            # exit with result in a0
 *
 * Actually for unconditional jump, we use BEQ x0, x0 with negative offset
 * But BEQ isn't in our minimal set. Let's use a different approach:
 * BLT x0, x1, loop where x1 > 0 (always true for our setup)
 */
static int compile_factorial(void) {
    output_len = 0;

    /* li a1, 10  (addi a1, zero, 10) */
    emit(addi(A1, ZERO, 10));

    /* li a0, 1   (addi a0, zero, 1) */
    emit(addi(A0, ZERO, 1));

    /* li a2, 2   (addi a2, zero, 2) */
    emit(addi(A2, ZERO, 2));

    /* loop: blt a1, a2, done (offset = 16 bytes = 4 instructions forward) */
    emit(blt(A1, A2, 16));

    /* mul a0, a0, a1 */
    emit(mul(A0, A0, A1));

    /* addi a1, a1, -1 */
    emit(addi(A1, A1, -1));

    /* blt zero, a2, loop (always branches since 0 < 2, offset = -12 bytes) */
    emit(blt(ZERO, A2, -12));

    /* done: ecall */
    emit(ecall());

    return output_len;
}

/*
 * Verify the compiled code by checking instruction encodings
 * Returns 1 if valid, 0 if invalid
 */
static int verify_output(void) {
    /* Check we have the right number of instructions */
    if (output_len != 8) return 0;

    /* Verify first instruction: addi a1, zero, 10 */
    /* Expected: 0x00a00593 */
    if (output[0] != 0x00a00593) return 0;

    /* Verify second instruction: addi a0, zero, 1 */
    /* Expected: 0x00100513 */
    if (output[1] != 0x00100513) return 0;

    /* All basic checks passed */
    return 1;
}

/*
 * Compute a simple checksum of the output
 * This exercises memory access patterns
 */
static unsigned int compute_checksum(void) {
    unsigned int sum = 0;
    for (int i = 0; i < output_len; i++) {
        sum += output[i];
        sum ^= (output[i] >> 16);
    }
    return sum;
}

/*
 * Main benchmark function
 * Compiles factorial multiple times and returns performance metric
 */
int mini_compiler_benchmark(void) {
    int total_instructions = 0;
    unsigned int checksum = 0;

    /* Compile factorial 100 times to get measurable runtime */
    for (int i = 0; i < 100; i++) {
        int count = compile_factorial();
        total_instructions += count;

        /* Verify periodically */
        if (i % 10 == 0) {
            if (!verify_output()) {
                return -1;  /* Compilation error */
            }
        }

        checksum ^= compute_checksum();
    }

    /* Return a combined result:
     * Upper 16 bits: total instructions compiled
     * Lower 16 bits: checksum for verification */
    return ((total_instructions & 0xFFFF) << 16) | (checksum & 0xFFFF);
}
