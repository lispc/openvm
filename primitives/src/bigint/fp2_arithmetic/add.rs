use std::{ops::Deref, sync::Arc};

use afs_stark_backend::interaction::InteractionBuilder;
use num_bigint_dig::{BigInt, BigUint, Sign};
use p3_air::{Air, BaseAir};
use p3_field::{Field, PrimeField64};
use p3_matrix::Matrix;

use super::{Equation3, Equation5, Fp2ArithmeticAir, Fp2ArithmeticCols, OverflowInt};
use crate::{
    bigint::OverflowIntPair,
    sub_chip::{AirConfig, LocalTraceInstructions, SubAir},
    var_range::VariableRangeCheckerChip,
};

#[derive(Clone, Debug)]
pub struct Fp2AdditionAir {
    pub arithmetic: Fp2ArithmeticAir,
}

impl Deref for Fp2AdditionAir {
    type Target = Fp2ArithmeticAir;

    fn deref(&self) -> &Self::Target {
        &self.arithmetic
    }
}

impl AirConfig for Fp2AdditionAir {
    type Cols<T> = Fp2ArithmeticCols<T>;
}

impl<F: Field> BaseAir<F> for Fp2AdditionAir {
    fn width(&self) -> usize {
        self.arithmetic.width()
    }
}

impl<AB: InteractionBuilder> Air<AB> for Fp2AdditionAir {
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local = main.row_slice(0);
        let local = Fp2ArithmeticCols::<AB::Var>::from_slice(
            &local,
            self.num_limbs,
            self.q_limbs,
            self.carry_limbs,
        );
        SubAir::eval(self, builder, local, ());
    }
}

impl<AB: InteractionBuilder> SubAir<AB> for Fp2AdditionAir {
    type IoView = Fp2ArithmeticCols<AB::Var>;
    type AuxView = ();

    fn eval(&self, builder: &mut AB, io: Self::IoView, _aux: Self::AuxView) {
        let equation: Equation3<AB::Expr, OverflowIntPair<AB::Expr>> = |x, y, r| x + y - r;
        self.arithmetic.eval(builder, io, equation);
    }
}

impl<F: PrimeField64> LocalTraceInstructions<F> for Fp2AdditionAir {
    type LocalInput = (
        (BigUint, BigUint),
        (BigUint, BigUint),
        Arc<VariableRangeCheckerChip>,
    );

    fn generate_trace_row(&self, input: Self::LocalInput) -> Self::Cols<F> {
        let (x, y, range_checker) = input;
        let raw_sum = (x.0.clone() + y.0.clone(), x.1.clone() + y.1.clone());
        let sign = (
            if raw_sum.0 < self.modulus {
                // x + y - r == 0
                Sign::NoSign
            } else {
                Sign::Plus
            },
            if raw_sum.1 < self.modulus {
                // x + y - r == 0
                Sign::NoSign
            } else {
                Sign::Plus
            },
        );
        let r = (
            raw_sum.0.clone() % self.modulus.clone(),
            raw_sum.1.clone() % self.modulus.clone(),
        );
        let q = (
            BigInt::from_biguint(sign.0, (raw_sum.0 - r.0.clone()) / self.modulus.clone()),
            BigInt::from_biguint(sign.1, (raw_sum.1 - r.1.clone()) / self.modulus.clone()),
        );
        let equation: Equation5<isize, OverflowIntPair<isize>, OverflowInt<isize>> =
            |x, y, r, p, q| x + y - r - q.mul_by_int(p);
        self.arithmetic
            .generate_trace_row(x, y, q, r, equation, range_checker)
    }
}
