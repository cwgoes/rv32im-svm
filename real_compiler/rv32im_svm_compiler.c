/*
 * Real rv32im to SVM Compiler
 *
 * This compiler translates rv32im instructions to SVM (sBPF) bytecode.
 * It can be compiled to rv32im itself, creating a meta-compiler.
 *
 * Memory layout in SVM:
 * - 0x0000-0x7FFF: Program data
 * - 0x8000-0x807F: RISC-V register file (32 x 4 bytes)
 * - 0x8080+: Stack
 *
 * SVM register usage:
 * - R0: Return value
 * - R1-R4: Temporaries for compilation
 * - R9: Register file base pointer (0x8000)
 */

/* SVM opcodes */
#define ALU64_IMM   0x07
#define ALU64_REG   0x0f
#define ALU32_IMM   0x04
#define ALU32_REG   0x0c
#define JMP_IMM     0x05
#define JMP_REG     0x0d
#define LDX_W       0x61
#define STX_W       0x63

/* ALU operations */
#define OP_ADD      0x00
#define OP_SUB      0x10
#define OP_MUL      0x20
#define OP_DIV      0x30
#define OP_OR       0x40
#define OP_AND      0x50
#define OP_LSH      0x60
#define OP_RSH      0x70
#define OP_NEG      0x80
#define OP_MOD      0x90
#define OP_XOR      0xa0
#define OP_MOV      0xb0
#define OP_ARSH     0xc0
#define OP_SDIV     0xe0

/* Jump operations */
#define JMP_JA      0x00
#define JMP_JEQ     0x10
#define JMP_JGT     0x20
#define JMP_JGE     0x30
#define JMP_JNE     0x50
#define JMP_JSGT    0x60
#define JMP_JSGE    0x70
#define JMP_EXIT    0x90
#define JMP_JLT     0xa0
#define JMP_JLE     0xb0
#define JMP_JSLT    0xc0
#define JMP_JSLE    0xd0

/* SVM instruction encoding */
typedef struct {
    unsigned char opcode;
    unsigned char dst_src;  /* dst in lower 4 bits, src in upper 4 bits */
    short offset;
    int imm;
} svm_inst_t;

/* Output buffer */
static svm_inst_t svm_output[512];
static int svm_len = 0;

/* Mapping from rv32im PC to SVM PC for branch fixups */
static int pc_map[64];
static int rv_count = 0;

/* Emit an SVM instruction */
static void emit_svm(unsigned char opcode, unsigned char dst, unsigned char src,
                     short offset, int imm) {
    if (svm_len < 512) {
        svm_output[svm_len].opcode = opcode;
        svm_output[svm_len].dst_src = (src << 4) | (dst & 0x0f);
        svm_output[svm_len].offset = offset;
        svm_output[svm_len].imm = imm;
        svm_len++;
    }
}

/* Emit mov64 r, imm */
static void emit_mov64_imm(int reg, int imm) {
    emit_svm(ALU64_IMM | OP_MOV, reg, 0, 0, imm);
}

/* Emit mov64 r1, r2 */
static void emit_mov64_reg(int dst, int src) {
    emit_svm(ALU64_REG | OP_MOV, dst, src, 0, 0);
}

/* Emit add64 r, imm */
static void emit_add64_imm(int reg, int imm) {
    emit_svm(ALU64_IMM | OP_ADD, reg, 0, 0, imm);
}

/* Emit add64 r1, r2 */
static void emit_add64_reg(int dst, int src) {
    emit_svm(ALU64_REG | OP_ADD, dst, src, 0, 0);
}

/* Emit sub64 r1, r2 */
static void emit_sub64_reg(int dst, int src) {
    emit_svm(ALU64_REG | OP_SUB, dst, src, 0, 0);
}

/* Emit mul64 r1, r2 */
static void emit_mul64_reg(int dst, int src) {
    emit_svm(ALU64_REG | OP_MUL, dst, src, 0, 0);
}

/* Emit and64 r, imm */
static void emit_and64_imm(int reg, int imm) {
    emit_svm(ALU64_IMM | OP_AND, reg, 0, 0, imm);
}

/* Emit or64 r1, r2 */
static void emit_or64_reg(int dst, int src) {
    emit_svm(ALU64_REG | OP_OR, dst, src, 0, 0);
}

/* Emit xor64 r1, r2 */
static void emit_xor64_reg(int dst, int src) {
    emit_svm(ALU64_REG | OP_XOR, dst, src, 0, 0);
}

/* Emit lsh64 r, imm */
static void emit_lsh64_imm(int reg, int imm) {
    emit_svm(ALU64_IMM | OP_LSH, reg, 0, 0, imm);
}

/* Emit arsh64 r, imm */
static void emit_arsh64_imm(int reg, int imm) {
    emit_svm(ALU64_IMM | OP_ARSH, reg, 0, 0, imm);
}

/* Emit neg64 r */
static void emit_neg64(int reg) {
    emit_svm(ALU64_IMM | OP_NEG, reg, 0, 0, 0);
}

/* Emit ldx r1, [r2 + offset] (32-bit load) */
static void emit_ldxw(int dst, int src, short offset) {
    emit_svm(LDX_W, dst, src, offset, 0);
}

/* Emit stx [r1 + offset], r2 (32-bit store) */
static void emit_stxw(int dst, int src, short offset) {
    emit_svm(STX_W, dst, src, offset, 0);
}

/* Emit jump always */
static void emit_ja(short offset) {
    emit_svm(JMP_IMM | JMP_JA, 0, 0, offset, 0);
}

/* Emit conditional jump (register compare) */
static void emit_jmp_reg(unsigned char op, int dst, int src, short offset) {
    emit_svm(JMP_REG | op, dst, src, offset, 0);
}

/* Emit conditional jump (immediate compare) */
static void emit_jmp_imm(unsigned char op, int dst, int imm, short offset) {
    emit_svm(JMP_IMM | op, dst, 0, offset, imm);
}

/* Emit exit */
static void emit_exit(void) {
    emit_svm(JMP_IMM | JMP_EXIT, 0, 0, 0, 0);
}

/* Load rv32im register to SVM register with sign extension */
static void load_rv_reg(int svm_reg, int rv_reg) {
    if (rv_reg == 0) {
        /* x0 is always 0 */
        emit_mov64_imm(svm_reg, 0);
    } else {
        /* Load from register file: [R9 + rv_reg*4] */
        short offset = (short)(rv_reg * 4);
        emit_ldxw(svm_reg, 9, offset);
        /* Sign extend from 32 to 64 bits */
        emit_lsh64_imm(svm_reg, 32);
        emit_arsh64_imm(svm_reg, 32);
    }
}

/* Load rv32im register to SVM register without sign extension */
static void load_rv_reg_raw(int svm_reg, int rv_reg) {
    if (rv_reg == 0) {
        emit_mov64_imm(svm_reg, 0);
    } else {
        short offset = (short)(rv_reg * 4);
        emit_ldxw(svm_reg, 9, offset);
    }
}

/* Store SVM register to rv32im register */
static void store_rv_reg(int rv_reg, int svm_reg) {
    if (rv_reg == 0) {
        /* Writes to x0 are ignored */
        return;
    }
    short offset = (short)(rv_reg * 4);
    emit_stxw(9, svm_reg, offset);
}

/* Extract fields from rv32im instruction */
static int get_opcode(unsigned int inst) { return inst & 0x7f; }
static int get_rd(unsigned int inst) { return (inst >> 7) & 0x1f; }
static int get_funct3(unsigned int inst) { return (inst >> 12) & 0x7; }
static int get_rs1(unsigned int inst) { return (inst >> 15) & 0x1f; }
static int get_rs2(unsigned int inst) { return (inst >> 20) & 0x1f; }
static int get_funct7(unsigned int inst) { return (inst >> 25) & 0x7f; }

/* Sign-extend a value from a given bit width */
static int sign_extend(unsigned int val, int bits) {
    unsigned int sign_bit = 1u << (bits - 1);
    if (val & sign_bit) {
        return (int)(val | (~0u << bits));
    }
    return (int)val;
}

/* Extract I-type immediate */
static int get_imm_i(unsigned int inst) {
    return sign_extend(inst >> 20, 12);
}

/* Extract S-type immediate */
static int get_imm_s(unsigned int inst) {
    unsigned int imm = ((inst >> 7) & 0x1f) | ((inst >> 20) & 0xfe0);
    return sign_extend(imm, 12);
}

/* Extract B-type immediate */
static int get_imm_b(unsigned int inst) {
    unsigned int imm = ((inst >> 7) & 0x1e) |     /* imm[4:1] */
                       ((inst >> 20) & 0x7e0) |   /* imm[10:5] */
                       ((inst << 4) & 0x800) |    /* imm[11] */
                       ((inst >> 19) & 0x1000);   /* imm[12] */
    return sign_extend(imm, 13);
}

/* Extract U-type immediate */
static int get_imm_u(unsigned int inst) {
    return (int)(inst & 0xfffff000);
}

/* Extract J-type immediate */
static int get_imm_j(unsigned int inst) {
    unsigned int imm = ((inst >> 20) & 0x7fe) |   /* imm[10:1] */
                       ((inst >> 9) & 0x800) |    /* imm[11] */
                       (inst & 0xff000) |          /* imm[19:12] */
                       ((inst >> 11) & 0x100000);  /* imm[20] */
    return sign_extend(imm, 21);
}

/* Compile prologue: initialize R9 to register file base */
static void compile_prologue(void) {
    /* R9 = 0x8000 (register file base) */
    emit_mov64_imm(9, 0x8000);
}

/* Compile a single rv32im instruction */
static int compile_instruction(unsigned int inst, int rv_pc) {
    int opcode = get_opcode(inst);
    int rd = get_rd(inst);
    int rs1 = get_rs1(inst);
    int rs2 = get_rs2(inst);
    int funct3 = get_funct3(inst);
    int funct7 = get_funct7(inst);

    switch (opcode) {
    case 0x13: /* OP-IMM: ADDI, SLTI, SLTIU, XORI, ORI, ANDI, SLLI, SRLI, SRAI */
    {
        int imm = get_imm_i(inst);
        load_rv_reg_raw(1, rs1);

        switch (funct3) {
        case 0: /* ADDI */
            emit_add64_imm(1, imm);
            break;
        case 4: /* XORI */
            emit_svm(ALU64_IMM | OP_XOR, 1, 0, 0, imm);
            break;
        case 6: /* ORI */
            emit_svm(ALU64_IMM | OP_OR, 1, 0, 0, imm);
            break;
        case 7: /* ANDI */
            emit_and64_imm(1, imm);
            break;
        case 1: /* SLLI */
            emit_lsh64_imm(1, imm & 0x1f);
            break;
        case 5: /* SRLI or SRAI */
            if (funct7 == 0x20) {
                /* SRAI - arithmetic right shift */
                emit_lsh64_imm(1, 32);
                emit_arsh64_imm(1, 32 + (imm & 0x1f));
            } else {
                /* SRLI - logical right shift */
                emit_and64_imm(1, -1);  /* Zero extend */
                emit_svm(ALU64_IMM | OP_RSH, 1, 0, 0, imm & 0x1f);
            }
            break;
        case 2: /* SLTI */
            load_rv_reg(1, rs1);  /* Need sign extension for comparison */
            emit_mov64_imm(2, imm);
            /* r1 = r1 < r2 (signed) */
            emit_svm(ALU64_REG | OP_SUB, 1, 2, 0, 0);
            emit_arsh64_imm(1, 63);
            emit_and64_imm(1, 1);
            break;
        case 3: /* SLTIU */
            emit_mov64_imm(2, imm);
            /* r1 = r1 < r2 (unsigned) - use subtraction trick */
            emit_svm(ALU64_REG | OP_SUB, 1, 2, 0, 0);
            emit_svm(ALU64_IMM | OP_RSH, 1, 0, 0, 63);
            emit_and64_imm(1, 1);
            break;
        }
        store_rv_reg(rd, 1);
        break;
    }

    case 0x33: /* OP: ADD, SUB, SLL, SLT, SLTU, XOR, SRL, SRA, OR, AND, MUL... */
    {
        load_rv_reg_raw(1, rs1);
        load_rv_reg_raw(2, rs2);

        if (funct7 == 0x01) {
            /* M extension */
            switch (funct3) {
            case 0: /* MUL */
                emit_mul64_reg(1, 2);
                break;
            case 4: /* DIV */
                emit_svm(ALU64_REG | OP_SDIV, 1, 2, 0, 0);
                break;
            case 5: /* DIVU */
                emit_svm(ALU64_REG | OP_DIV, 1, 2, 0, 0);
                break;
            case 6: /* REM */
                emit_svm(ALU64_REG | OP_MOD, 1, 2, 0, 0);  /* Simplified */
                break;
            case 7: /* REMU */
                emit_svm(ALU64_REG | OP_MOD, 1, 2, 0, 0);
                break;
            }
        } else {
            switch (funct3) {
            case 0: /* ADD or SUB */
                if (funct7 == 0x20) {
                    emit_sub64_reg(1, 2);
                } else {
                    emit_add64_reg(1, 2);
                }
                break;
            case 1: /* SLL */
                emit_svm(ALU64_REG | OP_LSH, 1, 2, 0, 0);
                break;
            case 2: /* SLT */
                load_rv_reg(1, rs1);
                load_rv_reg(2, rs2);
                emit_sub64_reg(1, 2);
                emit_arsh64_imm(1, 63);
                emit_and64_imm(1, 1);
                break;
            case 3: /* SLTU */
                emit_sub64_reg(1, 2);
                emit_svm(ALU64_IMM | OP_RSH, 1, 0, 0, 63);
                emit_and64_imm(1, 1);
                break;
            case 4: /* XOR */
                emit_xor64_reg(1, 2);
                break;
            case 5: /* SRL or SRA */
                if (funct7 == 0x20) {
                    /* SRA */
                    emit_lsh64_imm(1, 32);
                    emit_svm(ALU64_REG | OP_ARSH, 1, 2, 0, 0);
                    emit_arsh64_imm(1, 32);
                } else {
                    /* SRL */
                    emit_svm(ALU64_REG | OP_RSH, 1, 2, 0, 0);
                }
                break;
            case 6: /* OR */
                emit_or64_reg(1, 2);
                break;
            case 7: /* AND */
                emit_svm(ALU64_REG | OP_AND, 1, 2, 0, 0);
                break;
            }
        }
        store_rv_reg(rd, 1);
        break;
    }

    case 0x37: /* LUI */
    {
        int imm = get_imm_u(inst);
        emit_mov64_imm(1, imm);
        store_rv_reg(rd, 1);
        break;
    }

    case 0x17: /* AUIPC */
    {
        int imm = get_imm_u(inst);
        int addr = rv_pc * 4 + imm;
        emit_mov64_imm(1, addr);
        store_rv_reg(rd, 1);
        break;
    }

    case 0x63: /* BRANCH: BEQ, BNE, BLT, BGE, BLTU, BGEU */
    {
        int imm = get_imm_b(inst);
        int target_rv = rv_pc + imm / 4;

        /* Load operands */
        if (funct3 == 4 || funct3 == 5) {
            /* BLT, BGE need sign extension */
            load_rv_reg(1, rs1);
            load_rv_reg(2, rs2);
        } else {
            load_rv_reg_raw(1, rs1);
            load_rv_reg_raw(2, rs2);
        }

        /* Emit jump (offset is placeholder, fixed up later) */
        unsigned char jmp_op;
        switch (funct3) {
        case 0: jmp_op = JMP_JEQ; break;
        case 1: jmp_op = JMP_JNE; break;
        case 4: jmp_op = JMP_JSLT; break;
        case 5: jmp_op = JMP_JSGE; break;
        case 6: jmp_op = JMP_JLT; break;
        case 7: jmp_op = JMP_JGE; break;
        default: jmp_op = JMP_JEQ; break;
        }
        /* Store placeholder - will be fixed up */
        emit_jmp_reg(jmp_op, 1, 2, (short)target_rv);
        break;
    }

    case 0x6f: /* JAL */
    {
        int imm = get_imm_j(inst);
        int target_rv = rv_pc + imm / 4;

        /* Save return address */
        int ret_addr = (rv_pc + 1) * 4;
        emit_mov64_imm(1, ret_addr);
        store_rv_reg(rd, 1);

        /* Jump (placeholder offset) */
        emit_ja((short)target_rv);
        break;
    }

    case 0x67: /* JALR */
    {
        int imm = get_imm_i(inst);

        /* Calculate target: (rs1 + imm) & ~1 */
        load_rv_reg_raw(1, rs1);
        emit_add64_imm(1, imm);
        emit_and64_imm(1, ~1);

        /* Save return address */
        int ret_addr = (rv_pc + 1) * 4;
        emit_mov64_imm(2, ret_addr);
        store_rv_reg(rd, 2);

        /* For now, just exit - dynamic jumps are complex */
        emit_exit();
        break;
    }

    case 0x03: /* LOAD: LB, LH, LW, LBU, LHU */
    {
        int imm = get_imm_i(inst);
        load_rv_reg_raw(1, rs1);
        emit_add64_imm(1, imm);

        /* Load from computed address */
        switch (funct3) {
        case 2: /* LW */
            emit_ldxw(1, 1, 0);
            /* Sign extend */
            emit_lsh64_imm(1, 32);
            emit_arsh64_imm(1, 32);
            break;
        default:
            /* Simplified: treat all as word loads */
            emit_ldxw(1, 1, 0);
            break;
        }
        store_rv_reg(rd, 1);
        break;
    }

    case 0x23: /* STORE: SB, SH, SW */
    {
        int imm = get_imm_s(inst);
        load_rv_reg_raw(1, rs1);
        emit_add64_imm(1, imm);
        load_rv_reg_raw(2, rs2);
        emit_stxw(1, 2, 0);
        break;
    }

    case 0x73: /* SYSTEM: ECALL, EBREAK */
    {
        /* Load a0 and exit */
        load_rv_reg(0, 10);  /* a0 = x10 */
        emit_exit();
        break;
    }

    default:
        /* Unknown instruction - emit nop equivalent */
        emit_mov64_imm(1, 0);
        break;
    }

    return 0;
}

/* Fix up branch offsets after compilation */
static void fixup_branches(void) {
    for (int i = 0; i < svm_len; i++) {
        unsigned char opcode = svm_output[i].opcode;
        unsigned char class = opcode & 0x07;

        if (class == 0x05 || class == 0x0d) {
            /* Jump instruction - check if offset is an rv_pc placeholder */
            short target_rv = svm_output[i].offset;
            if (target_rv >= 0 && target_rv < rv_count) {
                /* Convert rv_pc to svm_pc offset */
                int target_svm = pc_map[target_rv];
                svm_output[i].offset = (short)(target_svm - i - 1);
            }
        }
    }
}

/* Main compilation function */
int compile_rv32im(unsigned int *instructions, int count) {
    svm_len = 0;
    rv_count = count;

    /* Emit prologue */
    compile_prologue();

    /* Compile each instruction, recording PC mapping */
    for (int i = 0; i < count; i++) {
        pc_map[i] = svm_len;
        compile_instruction(instructions[i], i);
    }

    /* Fix up branches */
    fixup_branches();

    return svm_len;
}

/* Compute checksum of output */
static unsigned int compute_checksum(void) {
    unsigned int sum = 0;
    for (int i = 0; i < svm_len; i++) {
        unsigned int word = svm_output[i].opcode |
                           (svm_output[i].dst_src << 8) |
                           ((unsigned int)svm_output[i].offset << 16);
        sum += word;
        sum ^= svm_output[i].imm;
    }
    return sum;
}

/* Test program: factorial(10) in rv32im */
static unsigned int factorial_program[] = {
    0x00a00593,  /* addi a1, zero, 10 */
    0x00100513,  /* addi a0, zero, 1 */
    0x00200613,  /* addi a2, zero, 2 */
    0x00c5c863,  /* blt a1, a2, 16 (skip to end if n < 2) */
    0x02b50533,  /* mul a0, a0, a1 */
    0xfff58593,  /* addi a1, a1, -1 */
    0xfec04ae3,  /* blt zero, a2, -12 (loop back) */
    0x00000073,  /* ecall */
};

/* Benchmark: compile factorial multiple times */
int real_compiler_benchmark(void) {
    unsigned int total_svm = 0;
    unsigned int checksum = 0;

    /* Compile factorial 50 times */
    for (int i = 0; i < 50; i++) {
        int svm_count = compile_rv32im(factorial_program, 8);
        total_svm += svm_count;
        checksum ^= compute_checksum();
    }

    /* Return combined result */
    return ((total_svm & 0xFFFF) << 16) | (checksum & 0xFFFF);
}
