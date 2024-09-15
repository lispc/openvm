use std::{iter::repeat, sync::Arc};

use afs_stark_backend::interaction::InteractionBuilder;
use num_bigint_dig::{BigInt, BigUint};
use p3_field::PrimeField64;

use super::OverflowIntPair;
use crate::{
    bigint::{
        check_carry_mod_to_zero::{CheckCarryModToZeroCols, CheckCarryModToZeroSubAir},
        check_carry_to_zero::get_carry_max_abs_and_bits,
        utils::big_int_to_limbs,
        CanonicalUint, DefaultLimbConfig, OverflowInt,
    },
    var_range::VariableRangeCheckerChip,
};

pub mod add;
// pub mod div;
// pub mod mul;
// pub mod sub;

#[cfg(test)]
mod tests;

// Op(x, y) = r (mod p), where Op is one of +, -, *, /
#[derive(Clone)]
pub struct Fp2ArithmeticCols<T> {
    pub is_valid: T,
    pub x: (Vec<T>, Vec<T>),
    pub y: (Vec<T>, Vec<T>),
    pub q: (Vec<T>, Vec<T>),
    pub r: (Vec<T>, Vec<T>),
    pub carries: (Vec<T>, Vec<T>),
}

impl<T: Clone> Fp2ArithmeticCols<T> {
    pub fn from_slice(slc: &[T], num_limbs: usize, q_limbs: usize, carry_limbs: usize) -> Self {
        // The modulus p has num_limbs limbs.
        // So the numbers (x, y, r) we operate on have num_limbs limbs.
        // The carries are for the expression will be 2 * num_limbs - 1 for mul and div, and num_limbs for add and sub.
        // q limbs will be num_limbs for mul and div, and 1 for add and sub.
        let x = (
            slc[0..num_limbs].to_vec(),
            slc[num_limbs..2 * num_limbs].to_vec(),
        );
        let y = (
            slc[2 * num_limbs..3 * num_limbs].to_vec(),
            slc[3 * num_limbs..4 * num_limbs].to_vec(),
        );
        let r = (
            slc[4 * num_limbs..5 * num_limbs].to_vec(),
            slc[5 * num_limbs..6 * num_limbs].to_vec(),
        );
        let carries = (
            slc[6 * num_limbs..6 * num_limbs + carry_limbs].to_vec(),
            slc[6 * num_limbs + carry_limbs..6 * num_limbs + 2 * carry_limbs].to_vec(),
        );
        let q = (
            slc[6 * num_limbs + 2 * carry_limbs..6 * num_limbs + 2 * carry_limbs + q_limbs]
                .to_vec(),
            slc[6 * num_limbs + 2 * carry_limbs + q_limbs
                ..6 * num_limbs + 2 * carry_limbs + 2 * q_limbs]
                .to_vec(),
        );
        let is_valid = slc[6 * num_limbs + 2 * carry_limbs + 2 * q_limbs].clone();
        Self {
            x,
            y,
            q,
            r,
            carries,
            is_valid,
        }
    }

    pub fn flatten(&self) -> Vec<T> {
        let mut flattened = vec![];

        flattened.extend_from_slice(&self.x.0);
        flattened.extend_from_slice(&self.x.1);
        flattened.extend_from_slice(&self.y.0);
        flattened.extend_from_slice(&self.y.1);
        flattened.extend_from_slice(&self.r.0);
        flattened.extend_from_slice(&self.r.1);
        flattened.extend_from_slice(&self.carries.0);
        flattened.extend_from_slice(&self.carries.1);
        flattened.extend_from_slice(&self.q.0);
        flattened.extend_from_slice(&self.q.1);
        flattened.push(self.is_valid.clone());
        flattened
    }
}

type Equation3<T, S> = fn(S, S, S) -> OverflowIntPair<T>;
type Equation5<T, S, M> = fn(S, S, S, M, S) -> OverflowIntPair<T>;

#[derive(Clone, Debug)]
pub struct Fp2ArithmeticAir {
    pub check_carry_sub_air: CheckCarryModToZeroSubAir,
    // The modulus p
    pub modulus: BigUint,
    // The number of limbs of the big numbers we operate on. Should be the number of limbs of modulus.
    pub num_limbs: usize,
    // q and carry limbs can be different depends on the operation.
    pub q_limbs: usize,
    pub carry_limbs: usize,
    pub limb_bits: usize,
    pub range_decomp: usize,
}

impl Fp2ArithmeticAir {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        modulus: BigUint,
        limb_bits: usize,
        field_element_bits: usize,
        num_limbs: usize,
        q_limbs: usize,
        carry_limbs: usize,
        range_bus: usize,
        range_decomp: usize,
    ) -> Self {
        let check_carry_sub_air = CheckCarryModToZeroSubAir::new(
            modulus.clone(),
            limb_bits,
            range_bus,
            range_decomp,
            field_element_bits,
        );

        Self {
            check_carry_sub_air,
            modulus,
            num_limbs,
            q_limbs,
            carry_limbs,
            limb_bits,
            range_decomp,
        }
    }

    pub fn width(&self) -> usize {
        2 * (3 * self.num_limbs + self.q_limbs + self.carry_limbs) + 1
    }

    // Converting limb from an isize to a field element.
    fn to_f<F: PrimeField64>(x: isize) -> F {
        F::from_canonical_usize(x.unsigned_abs()) * if x >= 0 { F::one() } else { F::neg_one() }
    }

    pub fn eval<AB: InteractionBuilder>(
        &self,
        builder: &mut AB,
        cols: Fp2ArithmeticCols<AB::Var>,
        equation: Equation3<AB::Expr, OverflowIntPair<AB::Expr>>,
    ) {
        let Fp2ArithmeticCols {
            x,
            y,
            q,
            r,
            carries,
            is_valid,
        } = cols;

        let x_overflow = OverflowIntPair {
            x: OverflowInt::<AB::Expr>::from_var_vec::<AB, AB::Var>(x.0, self.limb_bits),
            y: OverflowInt::<AB::Expr>::from_var_vec::<AB, AB::Var>(x.1, self.limb_bits),
        };
        let y_overflow = OverflowIntPair {
            x: OverflowInt::<AB::Expr>::from_var_vec::<AB, AB::Var>(y.0, self.limb_bits),
            y: OverflowInt::<AB::Expr>::from_var_vec::<AB, AB::Var>(y.1, self.limb_bits),
        };
        let r_overflow = OverflowIntPair {
            x: OverflowInt::<AB::Expr>::from_var_vec::<AB, AB::Var>(r.0, self.limb_bits),
            y: OverflowInt::<AB::Expr>::from_var_vec::<AB, AB::Var>(r.1, self.limb_bits),
        };
        let expr = equation(x_overflow, y_overflow, r_overflow);

        self.check_carry_sub_air.constrain_carry_mod_to_zero(
            builder,
            expr.x,
            CheckCarryModToZeroCols {
                carries: carries.0,
                quotient: q.0,
            },
            is_valid,
        );
        self.check_carry_sub_air.constrain_carry_mod_to_zero(
            builder,
            expr.y,
            CheckCarryModToZeroCols {
                carries: carries.1,
                quotient: q.1,
            },
            is_valid,
        );
    }

    pub fn generate_trace_row<F: PrimeField64>(
        &self,
        x: (BigUint, BigUint),
        y: (BigUint, BigUint),
        q: (BigInt, BigInt),
        r: (BigUint, BigUint),
        equation: Equation5<isize, OverflowIntPair<isize>, OverflowInt<isize>>,
        range_checker: Arc<VariableRangeCheckerChip>,
    ) -> Fp2ArithmeticCols<F> {
        // Quotient and result can be smaller, but padding to the desired length.
        let q_limbs: (Vec<isize>, Vec<isize>) = (
            big_int_to_limbs(q.0.clone(), self.limb_bits)
                .iter()
                .chain(repeat(&0))
                .take(self.q_limbs)
                .copied()
                .collect(),
            big_int_to_limbs(q.1.clone(), self.limb_bits)
                .iter()
                .chain(repeat(&0))
                .take(self.q_limbs)
                .copied()
                .collect(),
        );
        for &q in q_limbs.0.iter() {
            range_checker.add_count((q + (1 << self.limb_bits)) as u32, self.limb_bits + 1);
        }
        for &q in q_limbs.1.iter() {
            range_checker.add_count((q + (1 << self.limb_bits)) as u32, self.limb_bits + 1);
        }
        let q_f: (Vec<F>, Vec<F>) = (
            q_limbs.0.iter().map(|&x| Self::to_f(x)).collect(),
            q_limbs.1.iter().map(|&x| Self::to_f(x)).collect(),
        );
        let r_canonical = OverflowIntPair {
            x: CanonicalUint::<isize, DefaultLimbConfig>::from_big_uint(&r.0, Some(self.num_limbs))
                .into(),
            y: CanonicalUint::<isize, DefaultLimbConfig>::from_big_uint(&r.1, Some(self.num_limbs))
                .into(),
        };
        let r_f: (Vec<F>, Vec<F>) = (
            r_canonical
                .x
                .limbs
                .iter()
                .map(|&x| F::from_canonical_usize(x as usize))
                .collect(),
            r_canonical
                .y
                .limbs
                .iter()
                .map(|&x| F::from_canonical_usize(x as usize))
                .collect(),
        );

        let x_canonical = OverflowIntPair {
            x: CanonicalUint::<isize, DefaultLimbConfig>::from_big_uint(&x.0, Some(self.num_limbs))
                .into(),
            y: CanonicalUint::<isize, DefaultLimbConfig>::from_big_uint(&x.1, Some(self.num_limbs))
                .into(),
        };
        let y_canonical = OverflowIntPair {
            x: CanonicalUint::<isize, DefaultLimbConfig>::from_big_uint(&y.0, Some(self.num_limbs))
                .into(),
            y: CanonicalUint::<isize, DefaultLimbConfig>::from_big_uint(&y.1, Some(self.num_limbs))
                .into(),
        };
        let p_canonical = CanonicalUint::<isize, DefaultLimbConfig>::from_big_uint(
            &self.modulus,
            Some(self.num_limbs),
        );
        let q_overflow = OverflowIntPair {
            x: OverflowInt {
                limbs: q_limbs.0,
                max_overflow_bits: self.limb_bits + 1,
                limb_max_abs: (1 << self.limb_bits),
            },
            y: OverflowInt {
                limbs: q_limbs.1,
                max_overflow_bits: self.limb_bits + 1,
                limb_max_abs: (1 << self.limb_bits),
            },
        };
        let expr = equation(
            x_canonical.clone(),
            y_canonical.clone(),
            r_canonical,
            p_canonical.into(),
            q_overflow,
        );
        let carries = expr.calculate_carries(self.limb_bits);
        let mut carries_f = (
            vec![F::zero(); carries.0.len()],
            vec![F::zero(); carries.1.len()],
        );
        let (carry_min_abs, carry_bits) =
            get_carry_max_abs_and_bits(expr.x.max_overflow_bits, self.limb_bits);
        for (i, &carry) in carries.0.iter().enumerate() {
            range_checker.add_count((carry + carry_min_abs as isize) as u32, carry_bits);
            carries_f.0[i] = Self::to_f(carry);
        }

        for (i, &carry) in carries.1.iter().enumerate() {
            range_checker.add_count((carry + carry_min_abs as isize) as u32, carry_bits);
            carries_f.1[i] = Self::to_f(carry);
        }

        Fp2ArithmeticCols {
            x: (
                x_canonical
                    .x
                    .limbs
                    .iter()
                    .map(|x| F::from_canonical_usize(*x as usize))
                    .collect(),
                x_canonical
                    .y
                    .limbs
                    .iter()
                    .map(|x| F::from_canonical_usize(*x as usize))
                    .collect(),
            ),
            y: (
                y_canonical
                    .x
                    .limbs
                    .iter()
                    .map(|x| F::from_canonical_usize(*x as usize))
                    .collect(),
                y_canonical
                    .y
                    .limbs
                    .iter()
                    .map(|x| F::from_canonical_usize(*x as usize))
                    .collect(),
            ),
            q: q_f,
            r: r_f,
            carries: carries_f,
            is_valid: F::one(),
        }
    }
}
