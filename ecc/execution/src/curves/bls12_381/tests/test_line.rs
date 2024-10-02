use halo2curves_axiom::{
    bls12_381::{Fq, Fq12, Fq2, G1Affine},
    ff::Field,
};
use rand::{rngs::StdRng, SeedableRng};

use crate::{
    common::EcPoint,
    curves::bls12_381::{
        conv_023_to_fp12, conv_fp2_coeffs_to_fp12, fp12_square, mul_023_by_023, mul_by_023,
        mul_by_02345, point_to_023, BLS12_381_XI,
    },
};

// TODO[yj]: Probably should refactor these tests so that they don't repeat the ones in BN254
#[test]
fn test_fp12_square() {
    let mut rng = StdRng::seed_from_u64(8);
    let rnd = Fq12::random(&mut rng);
    let sq = fp12_square::<Fq12>(rnd);
    let sq_native = rnd.square();
    assert_eq!(sq, sq_native);
}

#[test]
#[ignore]
fn test_evaluate_line() {
    // NOTE: There is probably not a simple way to test this
}

#[test]
fn test_mul_023_by_023() {
    // Generate random curve points
    let mut rng = StdRng::seed_from_u64(8);
    let rnd_pt_0 = G1Affine::random(&mut rng);
    let rnd_pt_1 = G1Affine::random(&mut rng);
    let ec_point_0 = EcPoint::<Fq> {
        x: rnd_pt_0.x,
        y: rnd_pt_0.y,
    };
    let ec_point_1 = EcPoint::<Fq> {
        x: rnd_pt_1.x,
        y: rnd_pt_1.y,
    };

    // Get lines evaluated at rnd_pt_0 and rnd_pt_1
    let line_0 = point_to_023::<Fq, Fq2>(ec_point_0);
    let line_1 = point_to_023::<Fq, Fq2>(ec_point_1);

    // Multiply the two line functions & convert to Fq12 to compare
    let mul_023_by_023 = mul_023_by_023::<Fq, Fq2>(line_0, line_1, BLS12_381_XI);
    let mul_023_by_023 = conv_fp2_coeffs_to_fp12::<Fq, Fq2, Fq12>(&mul_023_by_023);

    // Compare with the result of multiplying two Fp12 elements
    let fp12_0 = conv_023_to_fp12::<Fq, Fq2, Fq12>(line_0);
    let fp12_1 = conv_023_to_fp12::<Fq, Fq2, Fq12>(line_1);
    let check_mul_fp12 = fp12_0 * fp12_1;
    assert_eq!(mul_023_by_023, check_mul_fp12);
}

#[test]
fn test_mul_by_023() {
    let mut rng = StdRng::seed_from_u64(8);
    let f = Fq12::random(&mut rng);
    let rnd_pt = G1Affine::random(&mut rng);
    let ec_point = EcPoint::<Fq> {
        x: rnd_pt.x,
        y: rnd_pt.y,
    };
    let line = point_to_023::<Fq, Fq2>(ec_point);
    let mul_by_023 = mul_by_023::<Fq, Fq2, Fq12>(f, line);
    println!("{:#?}", mul_by_023);

    let check_mul_fp12 = conv_023_to_fp12::<Fq, Fq2, Fq12>(line) * f;
    assert_eq!(mul_by_023, check_mul_fp12);
}

#[test]
fn test_mul_by_02345() {
    let mut rng = StdRng::seed_from_u64(8);
    let f = Fq12::random(&mut rng);
    let x = [
        Fq2::random(&mut rng),
        Fq2::random(&mut rng),
        Fq2::random(&mut rng),
        Fq2::random(&mut rng),
        Fq2::random(&mut rng),
        Fq2::random(&mut rng),
    ];
    let mul_by_02345 = mul_by_02345::<Fq, Fq2, Fq12>(f, x);

    let x_f12 = conv_fp2_coeffs_to_fp12::<Fq, Fq2, Fq12>(&x);
    assert_eq!(mul_by_02345, f * x_f12);
}
