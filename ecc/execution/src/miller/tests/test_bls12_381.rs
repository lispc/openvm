use halo2curves_axiom::bls12_381::{
    Fq, Fq12, Fq2, G1Affine, G2Affine, G2Prepared, MillerLoopResult,
};
use rand::{rngs::StdRng, SeedableRng};
use subtle::{Choice, ConditionallySelectable};

use crate::{
    common::EcPoint,
    curves::bls12_381::{BLS12_381_XI, GNARK_BLS12_381_PBE},
    miller::{
        miller_add_step, miller_double_and_add_step, miller_double_step, multi_miller_loop,
        multi_miller_loop_separate_double_plus_add,
    },
    operations::{evaluate_line, mul_013_by_013, mul_by_01234, mul_by_013},
};

#[test]
#[allow(non_snake_case)]
fn test_multi_miller_loop_bls12_381() {
    // Generate random G1 and G2 points
    // let mut rng0 = StdRng::seed_from_u64(8);
    // let P = G1Affine::random(&mut rng0);
    // let mut rng1 = StdRng::seed_from_u64(8 * 2);
    // let Q = G2Affine::random(&mut rng1);
    // let either_identity = P.is_identity() | Q.is_identity();
    // let P = G1Affine::conditional_select(&P, &G1Affine::generator(), either_identity);
    // let Q = G2Affine::conditional_select(&Q, &G2Affine::generator(), either_identity);

    let P = G1Affine::generator();
    let Q = G2Affine::generator();

    println!("P.x: {:x?}", P.x.0);
    println!("Q.c0.x: {:x?}", Q.x.c0.0);

    let P_is_on_curve: bool = P.is_on_curve().into();
    let Q_is_on_curve: bool = Q.is_on_curve().into();
    assert!(P_is_on_curve);
    assert!(Q_is_on_curve);

    let P_ecpoint = EcPoint { x: P.x, y: P.y };
    let Q_ecpoint = EcPoint { x: Q.x, y: Q.y };

    // Compare against halo2curves implementation
    let g2_prepared = G2Prepared::from(Q);
    let compare_miller = halo2curves_axiom::bls12_381::multi_miller_loop(&[(&P, &g2_prepared)]);
    let compare_final = compare_miller.final_exponentiation();
    // let compare_final = halo2curves_axiom::bls12_381::pairing(&P, &Q);

    // Run the multi-miller loop
    let f = multi_miller_loop::<Fq, Fq2, Fq12>(
        // let f = multi_miller_loop_separate_double_plus_add::<Fq, Fq2, Fq12>(
        &[P_ecpoint],
        &[Q_ecpoint],
        GNARK_BLS12_381_PBE.as_slice(),
        BLS12_381_XI,
    );
    println!("{:#?}", f);
    let wrapped_f = MillerLoopResult(f);
    let final_f = wrapped_f.final_exponentiation();

    let cf = compare_final.0;
    println!("cf.c0.c0.c0: {:?}", cf.c0.c0.c0.0);
    println!("cf.c0.c0.c1: {:?}", cf.c0.c0.c1.0);
    println!("cf.c0.c1.c0: {:?}", cf.c0.c1.c0.0);
    println!("cf.c0.c1.c1: {:?}", cf.c0.c1.c1.0);
    println!("cf.c0.c2.c0: {:?}", cf.c0.c2.c0.0);
    println!("cf.c0.c2.c1: {:?}", cf.c0.c2.c1.0);
    println!("cf.c1.c0.c0: {:?}", cf.c1.c0.c0.0);
    println!("cf.c1.c0.c1: {:?}", cf.c1.c0.c1.0);
    println!("cf.c1.c1.c0: {:?}", cf.c1.c1.c0.0);
    println!("cf.c1.c1.c1: {:?}", cf.c1.c1.c1.0);
    println!("cf.c1.c2.c0: {:?}", cf.c1.c2.c0.0);
    println!("cf.c1.c2.c1: {:?}", cf.c1.c2.c1.0);

    // Run halo2curves final exponentiation on our multi_miller_loop output
    assert_eq!(final_f, compare_final);
}

#[test]
#[allow(non_snake_case)]
fn test_on_curve() {
    let two = Fq::one() + Fq::one();
    let three = Fq::one() + Fq::one() + Fq::one();
    let mut P = G1Affine::default();
    P.x = Fq::zero();
    P.y = Fq::one();
    let mut Q: G2Affine = G2Affine::default();
    Q.x = Fq2 {
        c0: Fq::zero(),
        c1: Fq::one(),
    };
    Q.y = Fq2 { c0: two, c1: three };
    let P_on_curve: bool = P.is_on_curve().into();
    let Q_on_curve: bool = Q.is_on_curve().into();
    assert!(P_on_curve);
    assert!(Q_on_curve);
}

#[test]
#[allow(non_snake_case)]
fn test_f_mul() {
    // Generate random G1 and G2 points
    // let mut rng0 = StdRng::seed_from_u64(8);
    // let P = G1Affine::random(&mut rng0);
    // let mut rng1 = StdRng::seed_from_u64(8 * 2);
    // let Q = G2Affine::random(&mut rng1);
    // let either_identity = P.is_identity() | Q.is_identity();
    // let P = G1Affine::conditional_select(&P, &G1Affine::generator(), either_identity);
    // let Q = G2Affine::conditional_select(&Q, &G2Affine::generator(), either_identity);

    // let two = Fq::one() + Fq::one();
    // let three = Fq::one() + Fq::one() + Fq::one();
    // let mut P = G1Affine::default();
    // P.x = Fq::zero();
    // P.y = Fq::one();
    // let mut Q: G2Affine = G2Affine::default();
    // Q.x = Fq2 {
    //     c0: Fq::zero(),
    //     c1: Fq::one(),
    // };
    // Q.y = Fq2 { c0: two, c1: three };

    let P = G1Affine::generator();
    let Q = G2Affine::generator();

    let P_ecpoint = EcPoint { x: P.x, y: P.y };
    let Q_ecpoint = EcPoint { x: Q.x, y: Q.y };

    // Setup constants
    let y_inv = P_ecpoint.y.invert().unwrap();
    let x_over_y = P_ecpoint.x * y_inv;

    // We want to check that Fp12 * (l_(S+Q+S) is equal to Fp12 * (l_(2S) * l_(S+Q))
    let mut f = Fq12::one();
    let mut Q_acc = Q_ecpoint.clone();

    // Initial step: double
    let (Q_acc_init, l_init) = miller_double_step::<Fq, Fq2>(Q_ecpoint.clone());
    let l_init = evaluate_line::<Fq, Fq2>(l_init, x_over_y.clone(), y_inv.clone());
    f = mul_by_013::<Fq, Fq2, Fq12>(f, l_init);
    Q_acc = Q_acc_init;

    // Now f is in a state where we can do a left vs right side test of double-and-add vs double then add:

    // Left side test: Double and add
    let (Q_acc_daa, l_S_plus_Q, l_S_plus_Q_plus_S) =
        miller_double_and_add_step::<Fq, Fq2>(Q_acc.clone(), Q_ecpoint.clone());
    let l_S_plus_Q_plus_S =
        evaluate_line::<Fq, Fq2>(l_S_plus_Q_plus_S, x_over_y.clone(), y_inv.clone());
    // let l_S_plus_Q = evaluate_line::<Fq, Fq2>(l_S_plus_Q, x_over_y.clone(), y_inv.clone());
    // let l_prod0 = mul_013_by_013(l_S_plus_Q, l_S_plus_Q_plus_S, BLS12_381_XI);
    // let f_mul = mul_by_01234::<Fq, Fq2, Fq12>(f.clone(), l_prod0);
    let f_mul = mul_by_013::<Fq, Fq2, Fq12>(f.clone(), l_S_plus_Q_plus_S);
    // let f_mul = f_mul.conjugate();

    // Right side test: Double, then add
    let (Q_acc_d, l_2S) = miller_double_step::<Fq, Fq2>(Q_acc.clone());
    let (Q_acc_a, l_S_plus_Q) = miller_add_step::<Fq, Fq2>(Q_acc_d, Q_ecpoint.clone());
    let l_2S = evaluate_line::<Fq, Fq2>(l_2S, x_over_y.clone(), y_inv.clone());
    let l_S_plus_Q = evaluate_line::<Fq, Fq2>(l_S_plus_Q, x_over_y.clone(), y_inv.clone());
    let l_prod1 = mul_013_by_013(l_2S, l_S_plus_Q, BLS12_381_XI);
    let f_prod_mul = mul_by_01234::<Fq, Fq2, Fq12>(f.clone(), l_prod1);

    // assert_eq!(f_mul, f_prod_mul);
    assert_eq!(Q_acc_daa.x, Q_acc_a.x);
    assert_eq!(Q_acc_daa.y, Q_acc_a.y);

    let wrapped_f_mul = MillerLoopResult(f_mul);
    let final_f_mul = wrapped_f_mul.final_exponentiation();

    let wrapped_f_prod_mul = MillerLoopResult(f_prod_mul);
    let final_f_prod_mul = wrapped_f_prod_mul.final_exponentiation();

    assert_eq!(final_f_mul, final_f_prod_mul);
}
