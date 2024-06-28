use p3_field::{ExtensionField, PrimeField64};

use crate::asm::{AsmInstruction, AssemblyCode};

use stark_vm::cpu::trace::Instruction;
use stark_vm::cpu::OpCode;
use stark_vm::cpu::OpCode::*;

const BETA: usize = 11;

#[derive(Clone, Copy)]
pub struct CompilerOptions {
    pub compile_prints: bool,
    pub field_arithmetic_enabled: bool,
    pub field_extension_enabled: bool,
}

fn inst<F: PrimeField64>(
    opcode: OpCode,
    op_a: F,
    op_b: F,
    op_c: F,
    d: AS,
    e: AS,
) -> Instruction<F> {
    Instruction {
        opcode,
        op_a,
        op_b,
        op_c,
        d: d.to_field(),
        e: e.to_field(),
    }
}

enum AS {
    Immediate,
    Register,
    Memory,
}

impl AS {
    fn to_field<F: PrimeField64>(&self) -> F {
        match self {
            AS::Immediate => F::zero(),
            AS::Register => F::one(),
            AS::Memory => F::two(),
        }
    }
}

fn register<F: PrimeField64>(value: i32) -> F {
    let value = 1 - value;
    //println!("register index: {}", value);
    assert!(value > 0);
    F::from_canonical_usize(value as usize)
}

fn convert_field_arithmetic_instruction<F: PrimeField64, EF: ExtensionField<F>>(
    instruction: AsmInstruction<F, EF>,
    utility_register: F,
) -> Vec<Instruction<F>> {
    match instruction {
        AsmInstruction::AddF(dst, lhs, rhs) => vec![
            // register[dst] <- register[lhs] + register[rhs]
            inst(
                FADD,
                register(dst),
                register(lhs),
                register(rhs),
                AS::Register,
                AS::Register,
            ),
        ],
        AsmInstruction::AddFI(dst, lhs, rhs) => vec![
            // register[dst] <- register[lhs] + rhs
            inst(
                FADD,
                register(dst),
                register(lhs),
                rhs,
                AS::Register,
                AS::Immediate,
            ),
        ],
        AsmInstruction::SubF(dst, lhs, rhs) => vec![
            // register[dst] <- register[lhs] - register[rhs]
            inst(
                FSUB,
                register(dst),
                register(lhs),
                register(rhs),
                AS::Register,
                AS::Register,
            ),
        ],
        AsmInstruction::SubFI(dst, lhs, rhs) => vec![
            // register[dst] <- register[lhs] - rhs
            inst(
                FSUB,
                register(dst),
                register(lhs),
                rhs,
                AS::Register,
                AS::Immediate,
            ),
        ],
        AsmInstruction::SubFIN(dst, lhs, rhs) => vec![
            // register[dst] <- register[rhs] - lhs
            inst(
                FSUB,
                register(dst),
                register(rhs),
                lhs,
                AS::Register,
                AS::Immediate,
            ),
            // register[dst] <- register[dst] * -1
            inst(
                FMUL,
                register(dst),
                register(dst),
                F::neg_one(),
                AS::Register,
                AS::Immediate,
            ),
        ],
        AsmInstruction::MulF(dst, lhs, rhs) => vec![
            // register[dst] <- register[lhs] * register[rhs]
            inst(
                FMUL,
                register(dst),
                register(lhs),
                register(rhs),
                AS::Register,
                AS::Register,
            ),
        ],
        AsmInstruction::MulFI(dst, lhs, rhs) => vec![
            // register[dst] <- register[lhs] * rhs
            inst(
                FMUL,
                register(dst),
                register(lhs),
                rhs,
                AS::Register,
                AS::Immediate,
            ),
        ],
        AsmInstruction::DivF(dst, lhs, rhs) => vec![
            // register[dst] <- register[lhs] / register[rhs]
            inst(
                FDIV,
                register(dst),
                register(lhs),
                register(rhs),
                AS::Register,
                AS::Register,
            ),
        ],
        AsmInstruction::DivFI(dst, lhs, rhs) => vec![
            // register[dst] <- register[lhs] / rhs
            inst(
                FDIV,
                register(dst),
                register(lhs),
                rhs,
                AS::Register,
                AS::Immediate,
            ),
        ],
        AsmInstruction::DivFIN(dst, lhs, rhs) => vec![
            // register[util] <- lhs
            inst(
                STOREW,
                lhs,
                F::zero(),
                utility_register,
                AS::Immediate,
                AS::Register,
            ),
            // register[dst] <- register[util] / register[rhs]
            inst(
                FDIV,
                register(dst),
                utility_register,
                register(rhs),
                AS::Register,
                AS::Register,
            ),
        ],
        _ => panic!(
            "Illegal argument to convert_field_arithmetic_instruction: {:?}",
            instruction
        ),
    }
}

fn convert_field_extension_mult<const WORD_SIZE: usize, F: PrimeField64>(
    dst: i32,
    lhs: i32,
    rhs: i32,
    x0: F,
) -> Vec<Instruction<F>> {
    let word_size_i32: i32 = WORD_SIZE as i32;
    let beta_f = F::from_canonical_usize(BETA);

    let a0 = dst;
    let a1 = dst + word_size_i32;
    let a2 = dst + 2 * word_size_i32;
    let a3 = dst + 3 * word_size_i32;

    let b0 = lhs;
    let b1 = lhs + word_size_i32;
    let b2 = lhs + 2 * word_size_i32;
    let b3 = lhs + 3 * word_size_i32;

    let c0 = rhs;
    let c1 = rhs + word_size_i32;
    let c2 = rhs + 2 * word_size_i32;
    let c3 = rhs + 3 * word_size_i32;

    let mut instructions: Vec<Instruction<F>> = vec![];

    // This computes the constant term of the resulting polynomial:
    // a_0 = b_0 * c_0 + BETA * (b_1 * c_3 + b_2 * c_2 + b_3 * c_1)
    let a0_inst = vec![
        inst(
            FMUL,
            register(a0),
            register(b1),
            register(c3),
            AS::Register,
            AS::Register,
        ),
        inst(
            FMUL,
            x0,
            register(b2),
            register(c2),
            AS::Register,
            AS::Register,
        ),
        inst(
            FADD,
            register(a0),
            register(a0),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(
            FMUL,
            x0,
            register(b3),
            register(c1),
            AS::Register,
            AS::Register,
        ),
        inst(
            FADD,
            register(a0),
            register(a0),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(
            FMUL,
            register(a0),
            register(a0),
            beta_f,
            AS::Register,
            AS::Immediate,
        ),
        inst(
            FMUL,
            x0,
            register(b0),
            register(c0),
            AS::Register,
            AS::Register,
        ),
        inst(
            FADD,
            register(a0),
            register(a0),
            x0,
            AS::Register,
            AS::Register,
        ),
    ];

    // This computes the coefficient of x in the resulting polynomial:
    // b_0 * c_1 + b_1 * c_0 + BETA * (b_2 * c_3 + b_3 * c_2)
    let a1_inst = vec![
        inst(
            FMUL,
            register(a1),
            register(b2),
            register(c3),
            AS::Register,
            AS::Register,
        ),
        inst(
            FMUL,
            x0,
            register(b3),
            register(c2),
            AS::Register,
            AS::Register,
        ),
        inst(
            FADD,
            register(a1),
            register(a1),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(
            FMUL,
            register(a1),
            register(a1),
            beta_f,
            AS::Register,
            AS::Immediate,
        ),
        inst(
            FMUL,
            x0,
            register(b0),
            register(c1),
            AS::Register,
            AS::Register,
        ),
        inst(
            FADD,
            register(a1),
            register(a1),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(
            FMUL,
            x0,
            register(b1),
            register(c0),
            AS::Register,
            AS::Register,
        ),
        inst(
            FADD,
            register(a1),
            register(a1),
            x0,
            AS::Register,
            AS::Register,
        ),
    ];

    // This computes the coefficient of x^2 in the resulting polynomial:
    // b_0 * c_2 + b_1 * c_1 + b_2 * c_0 + BETA * b_3 * c_3
    let a2_inst = vec![
        inst(
            FMUL,
            register(a2),
            register(b3),
            register(c3),
            AS::Register,
            AS::Register,
        ),
        inst(
            FMUL,
            register(a2),
            register(a2),
            beta_f,
            AS::Register,
            AS::Immediate,
        ),
        inst(
            FMUL,
            x0,
            register(b0),
            register(c2),
            AS::Register,
            AS::Register,
        ),
        inst(
            FADD,
            register(a2),
            register(a2),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(
            FMUL,
            x0,
            register(b1),
            register(c1),
            AS::Register,
            AS::Register,
        ),
        inst(
            FADD,
            register(a2),
            register(a2),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(
            FMUL,
            x0,
            register(b2),
            register(c0),
            AS::Register,
            AS::Register,
        ),
        inst(
            FADD,
            register(a2),
            register(a2),
            x0,
            AS::Register,
            AS::Register,
        ),
    ];

    // This computes the coefficient of x^3 in the resulting polynomial:
    // b_0 * c_3 + b_1 * c_2 + b_2 * c_1 + b_3 * c_0
    let a3_inst = vec![
        inst(
            FMUL,
            register(a3),
            register(b0),
            register(c3),
            AS::Register,
            AS::Register,
        ),
        inst(
            FMUL,
            x0,
            register(b1),
            register(c2),
            AS::Register,
            AS::Register,
        ),
        inst(
            FADD,
            register(a3),
            register(a3),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(
            FMUL,
            x0,
            register(b2),
            register(c1),
            AS::Register,
            AS::Register,
        ),
        inst(
            FADD,
            register(a3),
            register(a3),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(
            FMUL,
            x0,
            register(b3),
            register(c0),
            AS::Register,
            AS::Register,
        ),
        inst(
            FADD,
            register(a3),
            register(a3),
            x0,
            AS::Register,
            AS::Register,
        ),
    ];

    instructions.extend(a0_inst);
    instructions.extend(a1_inst);
    instructions.extend(a2_inst);
    instructions.extend(a3_inst);

    instructions
}

fn convert_field_extension_mult_immediate<
    const WORD_SIZE: usize,
    F: PrimeField64,
    EF: ExtensionField<F>,
>(
    dst: i32,
    lhs: i32,
    rhs: EF,
    x0: F,
) -> Vec<Instruction<F>> {
    let word_size_i32: i32 = WORD_SIZE as i32;
    let beta_f = F::from_canonical_usize(BETA);

    let a0 = dst;
    let a1 = dst + word_size_i32;
    let a2 = dst + 2 * word_size_i32;
    let a3 = dst + 3 * word_size_i32;

    let b0 = lhs;
    let b1 = lhs + word_size_i32;
    let b2 = lhs + 2 * word_size_i32;
    let b3 = lhs + 3 * word_size_i32;

    let slc = rhs.as_base_slice();
    let c0 = slc[0];
    let c1 = slc[1];
    let c2 = slc[2];
    let c3 = slc[3];

    let mut instructions: Vec<Instruction<F>> = vec![];

    // This computes the constant term of the resulting polynomial:
    // a_0 = b_0 * c_0 + BETA * (b_1 * c_3 + b_2 * c_2 + b_3 * c_1)
    let a0_inst = vec![
        inst(
            FMUL,
            register(a0),
            register(b1),
            c3,
            AS::Register,
            AS::Immediate,
        ),
        inst(FMUL, x0, register(b2), c2, AS::Register, AS::Immediate),
        inst(
            FADD,
            register(a0),
            register(a0),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(FMUL, x0, register(b3), c1, AS::Register, AS::Immediate),
        inst(
            FADD,
            register(a0),
            register(a0),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(
            FMUL,
            register(a0),
            register(a0),
            beta_f,
            AS::Register,
            AS::Immediate,
        ),
        inst(FMUL, x0, register(b0), c0, AS::Register, AS::Immediate),
        inst(
            FADD,
            register(a0),
            register(a0),
            x0,
            AS::Register,
            AS::Register,
        ),
    ];

    // This computes the coefficient of x in the resulting polynomial:
    // b_0 * c_1 + b_1 * c_0 + BETA * (b_2 * c_3 + b_3 * c_2)
    let a1_inst = vec![
        inst(
            FMUL,
            register(a1),
            register(b2),
            c3,
            AS::Register,
            AS::Immediate,
        ),
        inst(FMUL, x0, register(b3), c2, AS::Register, AS::Immediate),
        inst(
            FADD,
            register(a1),
            register(a1),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(
            FMUL,
            register(a1),
            register(a1),
            beta_f,
            AS::Register,
            AS::Immediate,
        ),
        inst(FMUL, x0, register(b0), c1, AS::Register, AS::Immediate),
        inst(
            FADD,
            register(a1),
            register(a1),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(FMUL, x0, register(b1), c0, AS::Register, AS::Immediate),
        inst(
            FADD,
            register(a1),
            register(a1),
            x0,
            AS::Register,
            AS::Register,
        ),
    ];

    // This computes the coefficient of x^2 in the resulting polynomial:
    // b_0 * c_2 + b_1 * c_1 + b_2 * c_0 + BETA * b_3 * c_3
    let a2_inst = vec![
        inst(
            FMUL,
            register(a2),
            register(b3),
            c3,
            AS::Register,
            AS::Immediate,
        ),
        inst(
            FMUL,
            register(a2),
            register(a2),
            beta_f,
            AS::Register,
            AS::Immediate,
        ),
        inst(FMUL, x0, register(b0), c2, AS::Register, AS::Immediate),
        inst(
            FADD,
            register(a2),
            register(a2),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(FMUL, x0, register(b1), c1, AS::Register, AS::Immediate),
        inst(
            FADD,
            register(a2),
            register(a2),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(FMUL, x0, register(b2), c0, AS::Register, AS::Immediate),
        inst(
            FADD,
            register(a2),
            register(a2),
            x0,
            AS::Register,
            AS::Register,
        ),
    ];

    // This computes the coefficient of x^3 in the resulting polynomial:
    // b_0 * c_3 + b_1 * c_2 + b_2 * c_1 + b_3 * c_0
    let a3_inst = vec![
        inst(
            FMUL,
            register(a3),
            register(b0),
            c3,
            AS::Register,
            AS::Immediate,
        ),
        inst(FMUL, x0, register(b1), c2, AS::Register, AS::Immediate),
        inst(
            FADD,
            register(a3),
            register(a3),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(FMUL, x0, register(b2), c1, AS::Register, AS::Immediate),
        inst(
            FADD,
            register(a3),
            register(a3),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(FMUL, x0, register(b3), c0, AS::Register, AS::Immediate),
        inst(
            FADD,
            register(a3),
            register(a3),
            x0,
            AS::Register,
            AS::Register,
        ),
    ];

    instructions.extend(a0_inst);
    instructions.extend(a1_inst);
    instructions.extend(a2_inst);
    instructions.extend(a3_inst);

    instructions
}

fn convert_field_extension_inv<const WORD_SIZE: usize, F: PrimeField64>(
    dst: i32,
    src: i32,
    utility_registers: [F; 4],
) -> Vec<Instruction<F>> {
    let word_size_i32: i32 = WORD_SIZE as i32;
    let beta_f = F::from_canonical_usize(BETA);

    let x0 = utility_registers[0];
    let x1 = utility_registers[1];
    let x2 = utility_registers[2];
    let x3 = utility_registers[3];

    let a0 = dst;
    let a1 = dst + word_size_i32;
    let a2 = dst + 2 * word_size_i32;
    let a3 = dst + 3 * word_size_i32;

    let b0 = src;
    let b1 = src + word_size_i32;
    let b2 = src + 2 * word_size_i32;
    let b3 = src + 3 * word_size_i32;

    let mut instructions = vec![];

    // First we compute the term b_0^2 - 11 * (2b_1 * b_3 - b_2^2), call this n
    let n_inst = vec![
        inst(
            FMUL,
            x0,
            register(b1),
            register(b3),
            AS::Register,
            AS::Register,
        ),
        inst(FMUL, x0, x0, F::two(), AS::Register, AS::Immediate),
        inst(
            FMUL,
            x1,
            register(b2),
            register(b2),
            AS::Register,
            AS::Register,
        ),
        inst(FSUB, x0, x0, x1, AS::Register, AS::Register),
        inst(FMUL, x0, x0, beta_f, AS::Register, AS::Immediate),
        inst(
            FMUL,
            x1,
            register(b0),
            register(b0),
            AS::Register,
            AS::Register,
        ),
        inst(FSUB, x0, x1, x0, AS::Register, AS::Register),
    ];

    // Next we compute the term 2 * b_0 * b_2 - b_1^2 - 11 * b_3^2, call this m
    let m_inst = vec![
        inst(
            FMUL,
            x1,
            register(b0),
            register(b2),
            AS::Register,
            AS::Register,
        ),
        inst(FMUL, x1, x1, F::two(), AS::Register, AS::Immediate),
        inst(
            FMUL,
            x2,
            register(b1),
            register(b1),
            AS::Register,
            AS::Register,
        ),
        inst(FSUB, x1, x1, x2, AS::Register, AS::Register),
        inst(
            FMUL,
            x2,
            register(b3),
            register(b3),
            AS::Register,
            AS::Register,
        ),
        inst(FMUL, x2, x2, beta_f, AS::Register, AS::Immediate),
        inst(FSUB, x1, x1, x2, AS::Register, AS::Register),
    ];

    // Now, we compute the term c = n^2 - 11*m^2, and then take the inverse, call this inv_c
    let inv_c_inst = vec![
        inst(FMUL, x2, x0, x0, AS::Register, AS::Register),
        inst(FMUL, x3, x1, x1, AS::Register, AS::Register),
        inst(FMUL, x3, x3, beta_f, AS::Register, AS::Immediate),
        inst(FSUB, x2, x2, x3, AS::Register, AS::Register),
        inst(STOREW, F::one(), F::zero(), x3, AS::Immediate, AS::Register),
        inst(FDIV, x2, x3, x2, AS::Register, AS::Register),
    ];

    // Now, we multiply n and m by inv_c
    let mul_inst = vec![
        inst(FMUL, x0, x0, x2, AS::Register, AS::Register),
        inst(FMUL, x1, x1, x2, AS::Register, AS::Register),
    ];

    // We compute the constant term of the result: b_0 * n - 11 * b_2 * m
    let a0_inst = vec![
        inst(
            FMUL,
            register(a0),
            register(b0),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(FMUL, x2, register(b2), x1, AS::Register, AS::Register),
        inst(FMUL, x2, x2, beta_f, AS::Register, AS::Immediate),
        inst(
            FSUB,
            register(a0),
            register(a0),
            x2,
            AS::Register,
            AS::Register,
        ),
    ];

    // We compute the coefficient of x: -b_1 * n + 11 * b_3 * m
    let a1_inst = vec![
        inst(
            FMUL,
            register(a1),
            register(b1),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(FMUL, x2, register(b3), x1, AS::Register, AS::Register),
        inst(FMUL, x2, x2, beta_f, AS::Register, AS::Immediate),
        inst(
            FSUB,
            register(a1),
            x2,
            register(a1),
            AS::Register,
            AS::Register,
        ),
    ];

    // Here, we compute the coefficient of x^2: b_2 * n - b_0 * m
    let a2_inst = vec![
        inst(
            FMUL,
            register(a2),
            register(b2),
            x0,
            AS::Register,
            AS::Register,
        ),
        inst(FMUL, x2, register(b0), x1, AS::Register, AS::Register),
        inst(
            FSUB,
            register(a2),
            register(a2),
            x2,
            AS::Register,
            AS::Register,
        ),
    ];

    // Finally, we compute the coefficient of x^3: b_1 * m - b_3 * n
    let a3_inst = vec![
        inst(
            FMUL,
            register(a3),
            register(b1),
            x1,
            AS::Register,
            AS::Register,
        ),
        inst(FMUL, x2, register(b3), x0, AS::Register, AS::Register),
        inst(
            FSUB,
            register(a3),
            register(a3),
            x2,
            AS::Register,
            AS::Register,
        ),
    ];

    instructions.extend(n_inst);
    instructions.extend(m_inst);
    instructions.extend(inv_c_inst);
    instructions.extend(mul_inst);
    instructions.extend(a0_inst);
    instructions.extend(a1_inst);
    instructions.extend(a2_inst);
    instructions.extend(a3_inst);

    instructions
}

fn convert_field_extension_arithmetic_instruction<
    const WORD_SIZE: usize,
    F: PrimeField64,
    EF: ExtensionField<F>,
>(
    instruction: AsmInstruction<F, EF>,
    utility_registers: [F; 4],
) -> Vec<Instruction<F>> {
    let x0 = utility_registers[0];
    let x1 = utility_registers[1];
    let x2 = utility_registers[2];
    let x3 = utility_registers[3];

    let word_size_i32: i32 = WORD_SIZE as i32;

    match instruction {
        AsmInstruction::AddE(dst, lhs, rhs) => {
            let a0 = dst;
            let a1 = dst + word_size_i32;
            let a2 = dst + 2 * word_size_i32;
            let a3 = dst + 3 * word_size_i32;

            let b0 = lhs;
            let b1 = lhs + word_size_i32;
            let b2 = lhs + 2 * word_size_i32;
            let b3 = lhs + 3 * word_size_i32;

            let c0 = rhs;
            let c1 = rhs + word_size_i32;
            let c2 = rhs + 2 * word_size_i32;
            let c3 = rhs + 3 * word_size_i32;

            let instructions = vec![
                inst(
                    FADD,
                    register(a0),
                    register(b0),
                    register(c0),
                    AS::Register,
                    AS::Register,
                ),
                inst(
                    FADD,
                    register(a1),
                    register(b1),
                    register(c1),
                    AS::Register,
                    AS::Register,
                ),
                inst(
                    FADD,
                    register(a2),
                    register(b2),
                    register(c2),
                    AS::Register,
                    AS::Register,
                ),
                inst(
                    FADD,
                    register(a3),
                    register(b3),
                    register(c3),
                    AS::Register,
                    AS::Register,
                ),
            ];

            instructions
        }
        AsmInstruction::AddEI(dst, lhs, rhs) => {
            let a0 = dst;
            let a1 = dst + word_size_i32;
            let a2 = dst + 2 * word_size_i32;
            let a3 = dst + 3 * word_size_i32;

            let b0 = lhs;
            let b1 = lhs + word_size_i32;
            let b2 = lhs + 2 * word_size_i32;
            let b3 = lhs + 3 * word_size_i32;

            let slc = rhs.as_base_slice();
            let c0 = slc[0];
            let c1 = slc[1];
            let c2 = slc[2];
            let c3 = slc[3];

            let instructions = vec![
                inst(
                    FADD,
                    register(a0),
                    register(b0),
                    c0,
                    AS::Register,
                    AS::Immediate,
                ),
                inst(
                    FADD,
                    register(a1),
                    register(b1),
                    c1,
                    AS::Register,
                    AS::Immediate,
                ),
                inst(
                    FADD,
                    register(a2),
                    register(b2),
                    c2,
                    AS::Register,
                    AS::Immediate,
                ),
                inst(
                    FADD,
                    register(a3),
                    register(b3),
                    c3,
                    AS::Register,
                    AS::Immediate,
                ),
            ];

            instructions
        }
        AsmInstruction::SubE(dst, lhs, rhs) => {
            let a0 = dst;
            let a1 = dst + word_size_i32;
            let a2 = dst + 2 * word_size_i32;
            let a3 = dst + 3 * word_size_i32;

            let b0 = lhs;
            let b1 = lhs + word_size_i32;
            let b2 = lhs + 2 * word_size_i32;
            let b3 = lhs + 3 * word_size_i32;

            let c0 = rhs;
            let c1 = rhs + word_size_i32;
            let c2 = rhs + 2 * word_size_i32;
            let c3 = rhs + 3 * word_size_i32;

            let instructions = vec![
                inst(
                    FSUB,
                    register(a0),
                    register(b0),
                    register(c0),
                    AS::Register,
                    AS::Register,
                ),
                inst(
                    FSUB,
                    register(a1),
                    register(b1),
                    register(c1),
                    AS::Register,
                    AS::Register,
                ),
                inst(
                    FSUB,
                    register(a2),
                    register(b2),
                    register(c2),
                    AS::Register,
                    AS::Register,
                ),
                inst(
                    FSUB,
                    register(a3),
                    register(b3),
                    register(c3),
                    AS::Register,
                    AS::Register,
                ),
            ];

            instructions
        }
        AsmInstruction::SubEI(dst, lhs, rhs) => {
            let a0 = dst;
            let a1 = dst + word_size_i32;
            let a2 = dst + 2 * word_size_i32;
            let a3 = dst + 3 * word_size_i32;

            let b0 = lhs;
            let b1 = lhs + word_size_i32;
            let b2 = lhs + 2 * word_size_i32;
            let b3 = lhs + 3 * word_size_i32;

            let slc = rhs.as_base_slice();
            let c0 = slc[0];
            let c1 = slc[1];
            let c2 = slc[2];
            let c3 = slc[3];

            let instructions = vec![
                inst(
                    FSUB,
                    register(a0),
                    register(b0),
                    c0,
                    AS::Register,
                    AS::Immediate,
                ),
                inst(
                    FSUB,
                    register(a1),
                    register(b1),
                    c1,
                    AS::Register,
                    AS::Immediate,
                ),
                inst(
                    FSUB,
                    register(a2),
                    register(b2),
                    c2,
                    AS::Register,
                    AS::Immediate,
                ),
                inst(
                    FSUB,
                    register(a3),
                    register(b3),
                    c3,
                    AS::Register,
                    AS::Immediate,
                ),
            ];

            instructions
        }
        AsmInstruction::SubEIN(dst, lhs, rhs) => {
            let a0 = dst;
            let a1 = dst + word_size_i32;
            let a2 = dst + 2 * word_size_i32;
            let a3 = dst + 3 * word_size_i32;

            let slc = lhs.as_base_slice();
            let b0 = slc[0];
            let b1 = slc[1];
            let b2 = slc[2];
            let b3 = slc[3];

            let c0 = rhs;
            let c1 = rhs + word_size_i32;
            let c2 = rhs + 2 * word_size_i32;
            let c3 = rhs + 3 * word_size_i32;

            let instructions = vec![
                inst(STOREW, b0, F::zero(), x0, AS::Immediate, AS::Register),
                inst(STOREW, b1, F::zero(), x1, AS::Immediate, AS::Register),
                inst(STOREW, b2, F::zero(), x2, AS::Immediate, AS::Register),
                inst(STOREW, b3, F::zero(), x3, AS::Immediate, AS::Register),
                inst(
                    FSUB,
                    register(a0),
                    x0,
                    register(c0),
                    AS::Register,
                    AS::Register,
                ),
                inst(
                    FSUB,
                    register(a1),
                    x1,
                    register(c1),
                    AS::Register,
                    AS::Register,
                ),
                inst(
                    FSUB,
                    register(a2),
                    x2,
                    register(c2),
                    AS::Register,
                    AS::Register,
                ),
                inst(
                    FSUB,
                    register(a3),
                    x3,
                    register(c3),
                    AS::Register,
                    AS::Register,
                ),
            ];

            instructions
        }
        AsmInstruction::MulE(dst, lhs, rhs) => {
            convert_field_extension_mult::<WORD_SIZE, F>(dst, lhs, rhs, x0)
        }
        AsmInstruction::MulEI(dst, lhs, rhs) => {
            convert_field_extension_mult_immediate::<WORD_SIZE, F, EF>(dst, lhs, rhs, x0)
        }
        AsmInstruction::DivE(dst, lhs, rhs) => {
            let inv_instr =
                convert_field_extension_inv::<WORD_SIZE, F>(dst, rhs, utility_registers);
            let mul_instr = convert_field_extension_mult::<WORD_SIZE, F>(dst, lhs, dst, x0);

            inv_instr.into_iter().chain(mul_instr).collect()
        }
        _ => panic!(
            "Illegal argument to convert_field_extension_arithmetic_instruction: {:?}",
            instruction
        ),
    }
}

fn convert_print_instruction<F: PrimeField64, EF: ExtensionField<F>>(
    instruction: AsmInstruction<F, EF>,
) -> Vec<Instruction<F>> {
    match instruction {
        AsmInstruction::PrintV(src) => vec![inst(
            PRINTF,
            register(src),
            F::zero(),
            F::zero(),
            AS::Register,
            AS::Immediate,
        )],
        AsmInstruction::PrintF(src) => vec![inst(
            PRINTF,
            register(src),
            F::zero(),
            F::zero(),
            AS::Register,
            AS::Immediate,
        )],
        AsmInstruction::PrintE(..) => panic!("Unsupported operation: PrintE"),
        _ => panic!(
            "Illegal argument to convert_print_instruction: {:?}",
            instruction
        ),
    }
}

fn convert_instruction<F: PrimeField64, EF: ExtensionField<F>>(
    instruction: AsmInstruction<F, EF>,
    pc: F,
    labels: impl Fn(F) -> F,
    options: CompilerOptions,
) -> Vec<Instruction<F>> {
    let utility_register = F::zero();
    let utility_registers = [
        F::zero(),
        F::one(),
        F::two(),
        F::from_canonical_usize(3),
        F::from_canonical_usize(4),
    ];
    match instruction {
        AsmInstruction::Break(_) => panic!("Unresolved break instruction"),
        AsmInstruction::LoadF(dst, src, index, offset, size) => vec![
            // register[util] <- register[index] * size
            inst(
                FMUL,
                utility_register,
                register(index),
                size,
                AS::Register,
                AS::Immediate,
            ),
            // register[util] <- register[src] + register[util]
            inst(
                FADD,
                utility_register,
                register(src),
                utility_register,
                AS::Register,
                AS::Register,
            ),
            // register[dst] <- mem[register[util] + offset]
            inst(
                LOADW,
                register(dst),
                offset,
                utility_register,
                AS::Register,
                AS::Memory,
            ),
        ],
        AsmInstruction::LoadFI(dst, src, index, offset, size) => vec![
            // register[dst] <- mem[register[src] + ((index * size) + offset)]
            inst(
                LOADW,
                register(dst),
                (index * size) + offset,
                register(src),
                AS::Register,
                AS::Memory,
            ),
        ],
        AsmInstruction::StoreF(val, addr, index, offset, size) => vec![
            // register[util] <- register[index] * size
            inst(
                FMUL,
                utility_register,
                register(index),
                size,
                AS::Register,
                AS::Immediate,
            ),
            // register[util] <- register[src] + register[util]
            inst(
                FADD,
                utility_register,
                register(addr),
                utility_register,
                AS::Register,
                AS::Register,
            ),
            //  mem[register[util] + offset] <- register[val]
            inst(
                STOREW,
                register(val),
                offset,
                utility_register,
                AS::Register,
                AS::Memory,
            ),
        ],
        AsmInstruction::StoreFI(val, addr, index, offset, size) => vec![
            // mem[register[addr] + ((index * size) + offset)] <- register[val]
            inst(
                STOREW,
                register(val),
                (index * size) + offset,
                register(addr),
                AS::Register,
                AS::Memory,
            ),
        ],

        AsmInstruction::Jal(dst, label, offset) => {
            assert_eq!(offset, F::zero());
            vec![
                // pc <- labels[label] + offset, register[dst] <- pc
                inst(
                    JAL,
                    register(dst),
                    labels(label) - pc,
                    F::zero(),
                    AS::Register,
                    AS::Immediate,
                ),
            ]
        }
        AsmInstruction::JalR(_dst, _label, _offset) => panic!("Jalr should never be used"),
        AsmInstruction::Bne(label, lhs, rhs) => vec![
            // if register[lhs] != register[rhs], pc <- labels[label]
            inst(
                BNE,
                register(lhs),
                register(rhs),
                labels(label) - pc,
                AS::Register,
                AS::Register,
            ),
        ],
        AsmInstruction::BneInc(label, lhs, rhs) => vec![
            // register[lhs] += 1
            inst(
                FADD,
                register(lhs),
                register(lhs),
                F::one(),
                AS::Register,
                AS::Immediate,
            ),
            // if register[lhs] != register[rhs], pc <- labels[label]
            inst(
                BNE,
                register(lhs),
                register(rhs),
                labels(label) - (pc + F::one()),
                AS::Register,
                AS::Register,
            ),
        ],
        AsmInstruction::BneI(label, lhs, rhs) => vec![
            // if register[lhs] != rhs, pc <- labels[label]
            inst(
                BNE,
                register(lhs),
                rhs,
                labels(label) - pc,
                AS::Register,
                AS::Immediate,
            ),
        ],
        AsmInstruction::BneIInc(label, lhs, rhs) => vec![
            // register[lhs] += 1
            inst(
                FADD,
                register(lhs),
                register(lhs),
                F::one(),
                AS::Register,
                AS::Immediate,
            ),
            // if register[lhs] != rhs, pc <- labels[label]
            inst(
                BNE,
                register(lhs),
                rhs,
                labels(label) - (pc + F::one()),
                AS::Register,
                AS::Immediate,
            ),
        ],
        AsmInstruction::Beq(label, lhs, rhs) => vec![
            // if register[lhs] == register[rhs], pc <- labels[label]
            inst(
                BEQ,
                register(lhs),
                register(rhs),
                labels(label) - pc,
                AS::Register,
                AS::Register,
            ),
        ],
        AsmInstruction::BeqI(label, lhs, rhs) => vec![
            // if register[lhs] == rhs, pc <- labels[label]
            inst(
                BEQ,
                register(lhs),
                rhs,
                labels(label) - pc,
                AS::Register,
                AS::Immediate,
            ),
        ],
        AsmInstruction::Trap => vec![
            // pc <- -1 (causes trace generation to fail)
            inst(
                FAIL,
                F::zero(),
                F::zero(),
                F::zero(),
                AS::Immediate,
                AS::Immediate,
            ),
        ],
        AsmInstruction::Halt => vec![
            // terminate
            inst(
                TERMINATE,
                F::zero(),
                F::zero(),
                F::zero(),
                AS::Immediate,
                AS::Immediate,
            ),
        ],
        AsmInstruction::AddF(..)
        | AsmInstruction::SubF(..)
        | AsmInstruction::MulF(..)
        | AsmInstruction::DivF(..)
        | AsmInstruction::AddFI(..)
        | AsmInstruction::SubFI(..)
        | AsmInstruction::MulFI(..)
        | AsmInstruction::DivFI(..)
        | AsmInstruction::SubFIN(..)
        | AsmInstruction::DivFIN(..) => {
            if options.field_arithmetic_enabled {
                convert_field_arithmetic_instruction(instruction, utility_register)
            } else {
                panic!(
                    "Unsupported instruction {:?}, field arithmetic is disabled",
                    instruction
                )
            }
        }
        // AsmInstruction::AddE(..)
        // | AsmInstruction::AddEI(..)
        // | AsmInstruction::SubE(..)
        // | AsmInstruction::SubEI(..)
        // | AsmInstruction::SubEIN(..)
        // | AsmInstruction::MulE(..)
        // | AsmInstruction::MulEI(..)
        // | AsmInstruction::DivE(..) => {
        //     if options.field_extension_enabled {
        //         convert_field_extension_arithmetic_instruction::<WORD_SIZE, F>(
        //             instruction,
        //             utility_registers,
        //         )
        //     } else {
        //         panic!("Field extension is disabled")
        //     }
        // }
        AsmInstruction::PrintV(..) | AsmInstruction::PrintF(..) | AsmInstruction::PrintE(..) => {
            if options.compile_prints {
                convert_print_instruction(instruction)
            } else {
                vec![]
            }
        }
        _ => panic!("Unsupported instruction {:?}", instruction),
    }
}

pub fn convert_program<F: PrimeField64, EF: ExtensionField<F>>(
    program: AssemblyCode<F, EF>,
    options: CompilerOptions,
) -> Vec<Instruction<F>> {
    let mut block_start = vec![];
    let mut pc = 0;
    for block in program.blocks.iter() {
        block_start.push(pc);
        for instruction in block.0.iter() {
            let instructions = convert_instruction(
                instruction.clone(),
                F::from_canonical_usize(pc),
                |label| label,
                options,
            );
            pc += instructions.len();
        }
    }

    let mut result = vec![];
    for block in program.blocks.iter() {
        for instruction in block.0.iter() {
            let labels =
                |label: F| F::from_canonical_usize(block_start[label.as_canonical_u64() as usize]);
            result.extend(convert_instruction(
                instruction.clone(),
                F::from_canonical_usize(result.len()),
                labels,
                options,
            ));
        }
    }

    result
}
