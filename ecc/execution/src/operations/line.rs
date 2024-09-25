use halo2curves_axiom::ff::Field;

use crate::{common::field::FieldExtension, curves::bn254::conv_fp2_coeffs_to_fq12};

pub fn evaluate_line<Fp, Fp2>(line: [Fp2; 2], x_over_y: Fp, y_inv: Fp) -> [Fp2; 2]
where
    Fp: Field,
    Fp2: FieldExtension<2, BaseField = Fp>,
{
    let b_prime = line[0];
    let c_prime = line[1];
    let b = b_prime.mul_base(&x_over_y);
    let c = c_prime.mul_base(&y_inv);
    [b, c]
}

/// Multiplies two elements in 013 form and outputs the product in 01234 form
pub fn mul_013_by_013<Fp, Fp2>(
    line_0: [Fp2; 2],
    line_1: [Fp2; 2],
    // TODO[yj]: once this function is moved into a chip, we can use the xi property instead of passing in this argument
    xi: Fp2,
) -> [Fp2; 5]
where
    Fp: Field,
    Fp2: FieldExtension<2, BaseField = Fp>,
{
    let b0 = line_0[0];
    let c0 = line_0[1];
    let b1 = line_1[0];
    let c1 = line_1[1];

    // where w⁶ = xi
    // l0 * l1 = 1 + (b0 + b1)w + (b0b1)w² + (c0 + c1)w³ + (b0c1 + b1c0)w⁴ + (c0c1)w⁶
    //         = (1 + c0c1 * xi) + (b0 + b1)w + (b0b1)w² + (c0 + c1)w³ + (b0c1 + b1c0)w⁴
    let l0 = Fp2::ONE + c0 * c1 * xi;
    let l1 = b0 + b1;
    let l2 = b0 * b1;
    let l3 = c0 + c1;
    let l4 = b0 * c1 + b1 * c0;

    [l0, l1, l2, l3, l4]
}

pub fn mul_by_013<Fp, Fp2, Fp6, Fp12>(f: Fp12, line: [Fp2; 2]) -> Fp12
where
    Fp: Field,
    Fp2: FieldExtension<2, BaseField = Fp>,
    Fp6: FieldExtension<3, BaseField = Fp2>,
    Fp12: FieldExtension<2, BaseField = Fp6>,
{
    mul_by_01234(f, [Fp2::ONE, line[0], Fp2::ZERO, line[1], Fp2::ZERO])
}

pub fn mul_by_01234<Fp, Fp2, Fp6, Fp12>(f: Fp12, x: [Fp2; 5]) -> Fp12
where
    Fp: Field,
    Fp2: FieldExtension<2, BaseField = Fp>,
    Fp6: FieldExtension<3, BaseField = Fp2>,
    Fp12: FieldExtension<2, BaseField = Fp6>,
{
    let x_fp6_c0 = Fp6::from_coeffs([x[0], x[2], x[4]]);
    let x_fp6_c1 = Fp6::from_coeffs([x[1], x[3], Fp2::ZERO]);
    let x_fp12 = Fp12::from_coeffs([x_fp6_c0, x_fp6_c1]);
    f * x_fp12
}
