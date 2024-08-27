use std::{cell::RefCell, rc::Rc};

use afs_primitives::range_gate::RangeCheckerGateChip;
use air::LongArithmeticAir;
use itertools::Itertools;
use p3_field::PrimeField32;

use crate::{
    arch::{
        bus::ExecutionBus,
        chips::InstructionExecutor,
        columns::ExecutionState,
        instructions::{Opcode, LONG_ARITHMETIC_INSTRUCTIONS},
    },
    cpu::trace::Instruction,
    memory::manager::{trace_builder::MemoryTraceBuilder, MemoryManager},
};

#[cfg(test)]
pub mod tests;

pub mod air;
pub mod bridge;
pub mod columns;
pub mod trace;

pub const fn num_limbs<const ARG_SIZE: usize, const LIMB_SIZE: usize>() -> usize {
    (ARG_SIZE + LIMB_SIZE - 1) / LIMB_SIZE
}

#[derive(Clone, Default)]
pub struct Address<T> {
    pub address_space: T,
    pub address: T,
}

impl<T: Clone> Address<T> {
    pub fn from_iter<I: Iterator<Item = T>>(iter: &mut I) -> Self {
        Self {
            address_space: iter.next().unwrap(),
            address: iter.next().unwrap(),
        }
    }

    pub fn flatten(&self) -> Vec<T> {
        vec![self.address_space.clone(), self.address.clone()]
    }
}

#[derive(Clone)]
pub struct LongArithmeticInstruction<F> {
    pub opcode: F,
    pub from_state: ExecutionState<F>,
    pub x_address: Address<F>,
    pub y_address: Address<F>,
    pub z_address: Address<F>,
}

impl<F: Clone> LongArithmeticInstruction<F> {
    pub fn from_iter<I: Iterator<Item = F>>(iter: &mut I) -> Self {
        Self {
            opcode: iter.next().unwrap(),
            from_state: ExecutionState::from_iter(iter),
            x_address: Address::from_iter(iter),
            y_address: Address::from_iter(iter),
            z_address: Address::from_iter(iter),
        }
    }

    pub fn flatten(&self) -> Vec<F> {
        [
            vec![self.opcode.clone()],
            self.from_state.clone().flatten().to_vec(),
            self.x_address.flatten(),
            self.y_address.flatten(),
            self.z_address.flatten(),
        ]
        .concat()
    }
}

/// Whatever is needed to compile it into a trace row later
pub struct LongArithmeticOperation<F> {
    pub instruction: LongArithmeticInstruction<F>,
    pub operand1: Vec<u32>,
    pub operand2: Vec<u32>,
    pub result: CalculationResult,
}

pub struct LongArithmeticChip<const ARG_SIZE: usize, const LIMB_SIZE: usize, F: PrimeField32> {
    pub air: LongArithmeticAir<ARG_SIZE, LIMB_SIZE>,
    pub range_checker_chip: RangeCheckerGateChip,
    operations: Vec<LongArithmeticOperation<F>>,

    pub memory_manager: Rc<RefCell<MemoryManager<F>>>,
    pub memory: MemoryTraceBuilder<F>,
}

impl<const ARG_SIZE: usize, const LIMB_SIZE: usize, F: PrimeField32> InstructionExecutor<F>
    for LongArithmeticChip<ARG_SIZE, LIMB_SIZE, F>
{
    fn execute(
        &mut self,
        instruction: &Instruction<F>,
        from_state: ExecutionState<usize>,
    ) -> ExecutionState<usize> {
        let Instruction {
            opcode,
            op_a: z_address,
            op_b: x_address,
            op_c: y_address,
            d: z_as,
            e: x_as,
            op_f: y_as,
            ..
        } = instruction.clone();
        assert!(LONG_ARITHMETIC_INSTRUCTIONS.contains(&opcode));
        let num_limbs = num_limbs::<ARG_SIZE, LIMB_SIZE>();
        // TODO: if we ever read more than one element at a time,
        // don't forget to update the timestamp change everywhere
        let x = (0..num_limbs)
            .map(|i| {
                self.memory
                    .read_elem(x_as, x_address + F::from_canonical_usize(i))
                    .as_canonical_u32()
            })
            .collect_vec();
        let y = (0..num_limbs)
            .map(|i| {
                self.memory
                    .read_elem(y_as, y_address + F::from_canonical_usize(i))
                    .as_canonical_u32()
            })
            .collect_vec();
        let result = LongArithmetic::calculate::<ARG_SIZE, LIMB_SIZE, F>(opcode, &x[..], &y[..]);
        (0..num_limbs).for_each(|i| {
            self.memory.write_elem(
                z_as,
                z_address + F::from_canonical_usize(i),
                F::from_canonical_u32(result.result_limbs[i]),
            );
        });
        self.operations.push(LongArithmeticOperation::<F> {
            instruction: LongArithmeticInstruction {
                opcode: F::from_canonical_u8(opcode as u8),
                from_state: from_state.map(F::from_canonical_usize),
                x_address: Address {
                    address_space: x_as,
                    address: x_address,
                },
                y_address: Address {
                    address_space: y_as,
                    address: y_address,
                },
                z_address: Address {
                    address_space: z_as,
                    address: z_address,
                },
            },
            operand1: x,
            operand2: y,
            result,
        });
        ExecutionState {
            pc: from_state.pc + 1,
            timestamp: from_state.timestamp
                + 2 * num_limbs
                + if opcode == Opcode::ADD256 || opcode == Opcode::SUB256 {
                    num_limbs
                } else {
                    1
                },
        }
    }
}

impl<const ARG_SIZE: usize, const LIMB_SIZE: usize, F: PrimeField32>
    LongArithmeticChip<ARG_SIZE, LIMB_SIZE, F>
{
    pub fn new(
        bus_index: usize,
        execution_bus: ExecutionBus,
        memory_manager: Rc<RefCell<MemoryManager<F>>>,
    ) -> Self {
        let mem_oc = memory_manager.borrow().make_offline_checker();
        Self {
            air: LongArithmeticAir {
                execution_bus,
                mem_oc,
                bus_index,
                base_op: Opcode::ADD256,
            },
            range_checker_chip: RangeCheckerGateChip::new(bus_index, 1 << LIMB_SIZE),
            operations: vec![],
            memory: MemoryTraceBuilder::new(memory_manager.clone()),
            memory_manager,
        }
    }
}

pub struct CalculationResult {
    result_limbs: Vec<u32>,
    buffer_limbs: Vec<u32>,
    cmp_result: bool,
}

struct LongArithmetic;
impl LongArithmetic {
    fn calculate<const ARG_SIZE: usize, const LIMB_SIZE: usize, F: PrimeField32>(
        opcode: Opcode,
        x: &[u32],
        y: &[u32],
    ) -> CalculationResult {
        match opcode {
            Opcode::ADD256 => {
                let (sum, carry) = Self::calc_sum::<ARG_SIZE, LIMB_SIZE>(x, y);
                CalculationResult {
                    result_limbs: sum,
                    buffer_limbs: carry,
                    cmp_result: false,
                }
            }
            Opcode::SUB256 => {
                let (diff, carry) = Self::calc_diff::<ARG_SIZE, LIMB_SIZE>(x, y);
                CalculationResult {
                    result_limbs: diff,
                    buffer_limbs: carry,
                    cmp_result: false,
                }
            }
            Opcode::LT256 => {
                let (diff, carry) = Self::calc_diff::<ARG_SIZE, LIMB_SIZE>(x, y);
                let cmp_result = *carry.last().unwrap() == 1;
                CalculationResult {
                    result_limbs: diff,
                    buffer_limbs: carry,
                    cmp_result,
                }
            }
            Opcode::EQ256 => {
                let num_limbs = num_limbs::<ARG_SIZE, LIMB_SIZE>();
                let mut inverse = vec![0u32; num_limbs];
                for i in 0..num_limbs {
                    if x[i] != y[i] {
                        inverse[i] = (F::from_canonical_u32(x[i]) - F::from_canonical_u32(y[i]))
                            .inverse()
                            .as_canonical_u32();
                        break;
                    }
                }
                CalculationResult {
                    result_limbs: vec![0u32; num_limbs],
                    buffer_limbs: inverse,
                    cmp_result: x.iter().zip(y).all(|(x, y)| x == y),
                }
            }
            _ => unreachable!(),
        }
    }

    fn calc_sum<const ARG_SIZE: usize, const LIMB_SIZE: usize>(
        x: &[u32],
        y: &[u32],
    ) -> (Vec<u32>, Vec<u32>) {
        let num_limbs = num_limbs::<ARG_SIZE, LIMB_SIZE>();
        let mut result = vec![0u32; num_limbs];
        let mut carry = vec![0u32; num_limbs];
        for i in 0..num_limbs {
            result[i] = x[i] + y[i] + if i > 0 { carry[i - 1] } else { 0 };
            carry[i] = result[i] >> LIMB_SIZE;
            result[i] &= (1 << LIMB_SIZE) - 1;
        }
        (result, carry)
    }

    fn calc_diff<const ARG_SIZE: usize, const LIMB_SIZE: usize>(
        x: &[u32],
        y: &[u32],
    ) -> (Vec<u32>, Vec<u32>) {
        let num_limbs = num_limbs::<ARG_SIZE, LIMB_SIZE>();
        let mut result = vec![0u32; num_limbs];
        let mut carry = vec![0u32; num_limbs];
        for i in 0..num_limbs {
            let rhs = y[i] + if i > 0 { carry[i - 1] } else { 0 };
            if x[i] >= rhs {
                result[i] = x[i] - rhs;
                carry[i] = 0;
            } else {
                result[i] = x[i] + (1 << LIMB_SIZE) - rhs;
                carry[i] = 1;
            }
        }
        (result, carry)
    }
}
