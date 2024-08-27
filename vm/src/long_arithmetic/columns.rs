use super::{air::LongArithmeticAir, num_limbs, LongArithmeticInstruction};
use crate::memory::offline_checker::columns::MemoryOfflineCheckerAuxCols;

pub struct LongArithmeticCols<const ARG_SIZE: usize, const LIMB_SIZE: usize, T> {
    pub io: LongArithmeticIoCols<ARG_SIZE, LIMB_SIZE, T>,
    pub aux: LongArithmeticAuxCols<ARG_SIZE, LIMB_SIZE, T>,
}

pub struct LongArithmeticIoCols<const ARG_SIZE: usize, const LIMB_SIZE: usize, T> {
    pub instruction: LongArithmeticInstruction<T>,

    pub x_limbs: Vec<T>,
    pub y_limbs: Vec<T>,
    pub z_limbs: Vec<T>,
    pub cmp_result: T,
}

pub struct LongArithmeticAuxCols<const ARG_SIZE: usize, const LIMB_SIZE: usize, T> {
    pub is_valid: T,
    pub opcode_add_flag: T, // 1 if z_limbs should contain the result of addition
    pub opcode_sub_flag: T, // 1 if z_limbs should contain the result of subtraction (means that opcode is SUB or LT)
    pub opcode_lt_flag: T,  // 1 if opcode is LT
    pub opcode_eq_flag: T,  // 1 if opcode is EQ
    // buffer is the carry of the addition/subtraction,
    // or may serve as a single-nonzero-inverse helper vector for EQ256.
    // Refer to air.rs for more details.
    pub buffer: Vec<T>,

    pub mem_oc_aux_cols: Vec<MemoryOfflineCheckerAuxCols<1, T>>,
}

impl<const ARG_SIZE: usize, const LIMB_SIZE: usize, T: Clone>
    LongArithmeticCols<ARG_SIZE, LIMB_SIZE, T>
{
    pub fn from_iter<I: Iterator<Item = T>>(
        iter: &mut I,
        air: &LongArithmeticAir<ARG_SIZE, LIMB_SIZE>,
    ) -> Self {
        let io = LongArithmeticIoCols::<ARG_SIZE, LIMB_SIZE, T>::from_iter(iter);
        let aux = LongArithmeticAuxCols::<ARG_SIZE, LIMB_SIZE, T>::from_iter(iter, air);

        Self { io, aux }
    }

    pub fn flatten(&self) -> Vec<T> {
        [self.io.flatten(), self.aux.flatten()].concat()
    }

    pub const fn get_width() -> usize {
        LongArithmeticIoCols::<ARG_SIZE, LIMB_SIZE, T>::get_width()
            + LongArithmeticAuxCols::<ARG_SIZE, LIMB_SIZE, T>::get_width()
    }
}

impl<const ARG_SIZE: usize, const LIMB_SIZE: usize, T: Clone>
    LongArithmeticIoCols<ARG_SIZE, LIMB_SIZE, T>
{
    pub const fn get_width() -> usize {
        3 * num_limbs::<ARG_SIZE, LIMB_SIZE>() + 10
    }

    pub fn from_iter<I: Iterator<Item = T>>(iter: &mut I) -> Self {
        let num_limbs = num_limbs::<ARG_SIZE, LIMB_SIZE>();
        Self {
            instruction: LongArithmeticInstruction::from_iter(iter),
            x_limbs: iter.take(num_limbs).collect(),
            y_limbs: iter.take(num_limbs).collect(),
            z_limbs: iter.take(num_limbs).collect(),
            cmp_result: iter.next().unwrap(),
        }
    }

    pub fn flatten(&self) -> Vec<T> {
        [
            self.instruction.flatten(),
            self.x_limbs.clone(),
            self.y_limbs.clone(),
            self.z_limbs.clone(),
            vec![self.cmp_result.clone()],
        ]
        .concat()
    }
}

impl<const ARG_SIZE: usize, const LIMB_SIZE: usize, T: Clone>
    LongArithmeticAuxCols<ARG_SIZE, LIMB_SIZE, T>
{
    pub const fn get_width() -> usize {
        5 + num_limbs::<ARG_SIZE, LIMB_SIZE>()
    }

    pub fn from_iter<I: Iterator<Item = T>>(
        iter: &mut I,
        air: &LongArithmeticAir<ARG_SIZE, LIMB_SIZE>,
    ) -> Self {
        let num_limbs = num_limbs::<ARG_SIZE, LIMB_SIZE>();

        let is_valid = iter.next().unwrap();
        let opcode_add_flag = iter.next().unwrap();
        let opcode_sub_flag = iter.next().unwrap();
        let opcode_lt_flag = iter.next().unwrap();
        let opcode_eq_flag = iter.next().unwrap();
        let buffer = iter.take(num_limbs).collect();
        let mem_oc_aux_cols = (0..num_limbs)
            .map(|_| MemoryOfflineCheckerAuxCols::try_from_iter(iter, &air.mem_oc.timestamp_lt_air))
            .collect();

        Self {
            is_valid,
            opcode_add_flag,
            opcode_sub_flag,
            opcode_lt_flag,
            opcode_eq_flag,
            buffer,
            mem_oc_aux_cols,
        }
    }

    pub fn flatten(&self) -> Vec<T> {
        [
            vec![
                self.is_valid.clone(),
                self.opcode_add_flag.clone(),
                self.opcode_sub_flag.clone(),
                self.opcode_lt_flag.clone(),
                self.opcode_eq_flag.clone(),
            ],
            self.buffer.clone(),
        ]
        .concat()
    }
}
