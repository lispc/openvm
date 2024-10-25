use std::array;

use num_bigint_dig::BigUint;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use p3_baby_bear::BabyBear;
use p3_field::PrimeField32;
use rand::{rngs::StdRng, Rng};

use crate::{
    arch::testing::VmChipTestBuilder,
    rv32im::adapters::{RV32_REGISTER_NUM_LIMBS, RV_IS_TYPE_IMM_BITS},
    system::program::Instruction,
};

pub fn i32_to_f<F: PrimeField32>(val: i32) -> F {
    if val.signum() == -1 {
        -F::from_canonical_u32(val.unsigned_abs())
    } else {
        F::from_canonical_u32(val as u32)
    }
}

// little endian.
// Warning: This function only returns the last NUM_LIMBS*LIMB_BITS bits of
//          the input, while the input can have more than that.
pub fn biguint_to_limbs<const NUM_LIMBS: usize>(
    mut x: BigUint,
    limb_size: usize,
) -> [u32; NUM_LIMBS] {
    let mut result = [0; NUM_LIMBS];
    let base = BigUint::from_u32(1 << limb_size).unwrap();
    for r in result.iter_mut() {
        *r = (x.clone() % &base).to_u32().unwrap();
        x /= &base;
    }
    assert!(x.is_zero());
    result
}

pub fn rv32_write_heap_default<const NUM_LIMBS: usize>(
    tester: &mut VmChipTestBuilder<BabyBear>,
    addr1_writes: Vec<[BabyBear; NUM_LIMBS]>,
    addr2_writes: Vec<[BabyBear; NUM_LIMBS]>,
    opcode_with_offset: usize,
) -> Instruction<BabyBear> {
    let (reg1, _) =
        tester.write_heap_default::<NUM_LIMBS>(RV32_REGISTER_NUM_LIMBS, 128, addr1_writes);
    let reg2 = if addr2_writes.is_empty() {
        0
    } else {
        let (reg2, _) =
            tester.write_heap_default::<NUM_LIMBS>(RV32_REGISTER_NUM_LIMBS, 128, addr2_writes);
        reg2
    };
    let (reg3, _) = tester.write_heap_pointer_default(RV32_REGISTER_NUM_LIMBS, 128);

    Instruction::from_isize(
        opcode_with_offset,
        reg3 as isize,
        reg1 as isize,
        reg2 as isize,
        1_isize,
        2_isize,
    )
}

pub fn generate_long_number<const NUM_LIMBS: usize, const LIMB_BITS: usize>(
    rng: &mut StdRng,
) -> [u32; NUM_LIMBS] {
    array::from_fn(|_| rng.gen_range(0..(1 << LIMB_BITS)))
}

pub fn generate_rv32_is_type_immediate(
    rng: &mut StdRng,
) -> (usize, [u32; RV32_REGISTER_NUM_LIMBS]) {
    let mut imm: u32 = rng.gen_range(0..(1 << RV_IS_TYPE_IMM_BITS));
    if (imm & 0x800) != 0 {
        imm |= !0xFFF
    }
    (
        (imm & 0xFFFFFF) as usize,
        [
            imm as u8,
            (imm >> 8) as u8,
            (imm >> 16) as u8,
            (imm >> 16) as u8,
        ]
        .map(|x| x as u32),
    )
}
