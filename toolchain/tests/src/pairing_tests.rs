#![allow(non_snake_case)]

use ax_ecc_execution::axvm_ecc::{
    algebra::field::FieldExtension,
    halo2curves::ff::Field,
    pairing::{EvaluatedLine, FinalExp, LineMulDType, MultiMillerLoop},
    AffinePoint,
};
use ax_stark_sdk::ax_stark_backend::p3_field::AbstractField;
use axvm_circuit::arch::{EcCurve, PairingCurve, VmConfig, VmExecutor};
use eyre::Result;
use p3_baby_bear::BabyBear;
use rand::SeedableRng;

use crate::utils::build_example_program;

type F = BabyBear;

mod bn254 {
    use std::iter;

    use ax_ecc_execution::{
        axvm_ecc::{
            halo2curves::{
                bn256::{Fq12, Fq2, Fr, G1Affine, G2Affine},
                ff::Field,
            },
            pairing::MillerStep,
            AffineCoords,
        },
        curves::bn254::Bn254,
    };
    use axvm_ecc_constants::BN254;

    use super::*;

    #[test]
    fn test_bn254_fp12_mul() -> Result<()> {
        let elf = build_example_program("fp12_mul")?;
        let executor = VmExecutor::<F>::new(
            VmConfig::rv32im()
                .add_pairing_support(vec![PairingCurve::Bn254])
                .add_ecc_support(vec![EcCurve::Bn254])
                .add_modular_support(vec![BN254.MODULUS.clone()])
                .add_complex_ext_support(vec![BN254.MODULUS.clone()]),
        );

        let mut rng = rand::rngs::StdRng::seed_from_u64(2);
        let f0 = Fq12::random(&mut rng);
        let f1 = Fq12::random(&mut rng);
        let r = f0 * f1;

        let io = [f0, f1, r]
            .into_iter()
            .flat_map(|fp12| fp12.to_coeffs())
            .flat_map(|fp2| fp2.to_bytes())
            .map(AbstractField::from_canonical_u8)
            .collect::<Vec<_>>();

        executor.execute(elf, vec![io])?;
        Ok(())
    }

    #[test]
    fn test_bn254_line_functions() -> Result<()> {
        let elf = build_example_program("pairing_line")?;
        let executor = VmExecutor::<F>::new(VmConfig::rv32im().add_canonical_pairing_curves());

        let mut rng = rand::rngs::StdRng::seed_from_u64(2);
        let a = G2Affine::random(&mut rng);
        let b = G2Affine::random(&mut rng);
        let c = G2Affine::random(&mut rng);

        let f = Fq12::random(&mut rng);
        let l0 = EvaluatedLine::<Fq2> { b: a.x(), c: a.y() };
        let l1 = EvaluatedLine::<Fq2> { b: b.x(), c: b.y() };

        // Test mul_013_by_013
        let r0 = Bn254::mul_013_by_013(&l0, &l1);
        let io0 = [l0, l1]
            .into_iter()
            .flat_map(|fp2| fp2.into_iter())
            .chain(r0)
            .flat_map(|fp2| fp2.to_coeffs())
            .flat_map(|fp| fp.to_bytes())
            .map(AbstractField::from_canonical_u8)
            .collect::<Vec<_>>();

        // Test mul_by_01234
        let x = [c.x(), c.y(), b.x(), b.y(), a.x()];
        let r1 = Bn254::mul_by_01234(&f, &x);
        let io1 = iter::empty()
            .chain(f.to_coeffs())
            .chain(x)
            .chain(r1.to_coeffs())
            .flat_map(|fp2| fp2.to_coeffs())
            .flat_map(|fp| fp.to_bytes())
            .map(AbstractField::from_canonical_u8)
            .collect::<Vec<_>>();

        let io_all = io0.into_iter().chain(io1).collect::<Vec<_>>();

        executor.execute(elf, vec![io_all])?;
        Ok(())
    }

    #[test]
    fn test_bn254_miller_step() -> Result<()> {
        let elf = build_example_program("pairing_miller_step")?;
        let executor = VmExecutor::<F>::new(VmConfig::rv32im().add_canonical_pairing_curves());

        let mut rng = rand::rngs::StdRng::seed_from_u64(20);
        let S = G2Affine::random(&mut rng);
        let Q = G2Affine::random(&mut rng);

        let s = AffinePoint::new(S.x(), S.y());
        let q = AffinePoint::new(Q.x(), Q.y());

        // Test miller_double_step
        let (pt, l) = Bn254::miller_double_step(&s);
        let io0 = [s.x, s.y, pt.x, pt.y, l.b, l.c]
            .into_iter()
            .flat_map(|fp| fp.to_bytes())
            .map(AbstractField::from_canonical_u8)
            .collect::<Vec<_>>();

        // Test miller_double_and_add_step
        let (pt, l0, l1) = Bn254::miller_double_and_add_step(&s, &q);
        let io1 = [s.x, s.y, q.x, q.y, pt.x, pt.y, l0.b, l0.c, l1.b, l1.c]
            .into_iter()
            .flat_map(|fp| fp.to_bytes())
            .map(AbstractField::from_canonical_u8)
            .collect::<Vec<_>>();

        let io_all = io0.into_iter().chain(io1).collect::<Vec<_>>();

        executor.execute(elf, vec![io_all])?;
        Ok(())
    }

    #[test]
    fn test_bn254_miller_loop() -> Result<()> {
        let elf = build_example_program("pairing_miller_loop")?;

        // TODO[yj]: Unfortunate workaround until MOD_IDX issue is resolved
        // let exe = axvm_circuit::arch::instructions::exe::AxVmExe::<F>::from(elf.clone());
        // let mut enabled_moduli = exe
        //     .custom_op_config
        //     .intrinsics
        //     .field_arithmetic
        //     .primes
        //     .iter()
        //     .map(|s| num_bigint_dig::BigUint::from_str(s).unwrap())
        //     .collect::<Vec<_>>();
        let enabled_moduli = vec![
            BN254.MODULUS.clone() + num_bigint_dig::BigUint::from(3u64),
            BN254.MODULUS.clone() + num_bigint_dig::BigUint::from(2u64),
            BN254.MODULUS.clone() + num_bigint_dig::BigUint::from(1u64),
            BN254.MODULUS.clone() + num_bigint_dig::BigUint::from(0u64),
        ];

        let executor = VmExecutor::<F>::new(
            VmConfig::rv32im()
                .add_pairing_support(vec![PairingCurve::Bn254])
                .add_ecc_support(vec![EcCurve::Bn254])
                .add_modular_support(enabled_moduli.clone())
                .add_complex_ext_support(enabled_moduli),
        );

        let S = G1Affine::generator();
        let Q = G2Affine::generator();

        let mut S_mul = [S * Fr::from(1), S * Fr::from(2)];
        S_mul[1].y = -S_mul[1].y;
        let Q_mul = [Q * Fr::from(2), Q * Fr::from(1)];

        let s = S_mul.map(|s| AffinePoint::new(s.x, s.y));
        let q = Q_mul.map(|p| AffinePoint::new(p.x, p.y));

        // Test miller_loop
        let f = Bn254::multi_miller_loop(&s, &q);
        let io0 = s
            .into_iter()
            .flat_map(|pt| [pt.x, pt.y].into_iter().flat_map(|fp| fp.to_bytes()))
            .map(AbstractField::from_canonical_u8)
            .collect::<Vec<_>>();

        let io1 = q
            .into_iter()
            .flat_map(|pt| [pt.x, pt.y].into_iter())
            .chain(f.to_coeffs())
            .flat_map(|fp2| fp2.to_coeffs())
            .flat_map(|fp| fp.to_bytes())
            .map(AbstractField::from_canonical_u8)
            .collect::<Vec<_>>();

        let io_all = io0.into_iter().chain(io1).collect::<Vec<_>>();

        executor.execute(elf, vec![io_all])?;
        Ok(())
    }
}

mod bls12_381 {
    use ax_ecc_execution::{
        axvm_ecc::{
            halo2curves::bls12_381::{Fq12, Fq2, Fr, G1Affine, G2Affine},
            pairing::{LineMulMType, MillerStep},
            AffinePoint,
        },
        curves::bls12_381::Bls12_381,
    };
    use axvm_ecc_constants::BLS12381;

    use super::*;

    #[test]
    fn test_bls12_381_fp12_mul() -> Result<()> {
        let elf = build_example_program("fp12_mul")?;
        let executor = VmExecutor::<F>::new(
            VmConfig::rv32im()
                .add_pairing_support(vec![PairingCurve::Bls12_381])
                .add_ecc_support(vec![EcCurve::Bls12_381])
                .add_modular_support(vec![BLS12381.MODULUS.clone()])
                .add_complex_ext_support(vec![BLS12381.MODULUS.clone()]),
        );

        let mut rng = rand::rngs::StdRng::seed_from_u64(50);
        let f0 = Fq12::random(&mut rng);
        let f1 = Fq12::random(&mut rng);
        let r = f0 * f1;

        let io = [f0, f1, r]
            .into_iter()
            .flat_map(|fp12| fp12.to_coeffs())
            .flat_map(|fp2| fp2.to_bytes())
            .map(AbstractField::from_canonical_u8)
            .collect::<Vec<_>>();

        executor.execute(elf, vec![io])?;
        Ok(())
    }

    #[test]
    fn test_bls12_381_line_functions() -> Result<()> {
        let elf = build_example_program("pairing_line")?;
        let executor = VmExecutor::<F>::new(
            VmConfig::rv32im().add_pairing_support(vec![PairingCurve::Bls12_381]),
        );

        let mut rng = rand::rngs::StdRng::seed_from_u64(5);
        let a = G2Affine::random(&mut rng);
        let b = G2Affine::random(&mut rng);
        let c = G2Affine::random(&mut rng);

        let f = Fq12::random(&mut rng);
        let l0 = EvaluatedLine::<Fq2> { b: a.x, c: a.y };
        let l1 = EvaluatedLine::<Fq2> { b: b.x, c: b.y };

        // Test mul_023_by_023
        let r0 = Bls12_381::mul_023_by_023(&l0, &l1);
        let io0 = [l0, l1]
            .into_iter()
            .flat_map(|fp2| fp2.into_iter())
            .chain(r0)
            .flat_map(|fp2| fp2.to_coeffs())
            .flat_map(|fp| fp.to_bytes())
            .map(AbstractField::from_canonical_u8)
            .collect::<Vec<_>>();

        // Test mul_by_02345
        let x = [c.x, c.y, b.x, b.y, a.x];
        let r1 = Bls12_381::mul_by_02345(&f, &x);
        let io1 = f
            .to_coeffs()
            .into_iter()
            .chain(x)
            .chain(r1.to_coeffs())
            .flat_map(|fp2| fp2.to_coeffs())
            .flat_map(|fp| fp.to_bytes())
            .map(AbstractField::from_canonical_u8)
            .collect::<Vec<_>>();

        let io_all = io0.into_iter().chain(io1).collect::<Vec<_>>();

        executor.execute(elf, vec![io_all])?;
        Ok(())
    }

    #[test]
    fn test_bls12_381_miller_step() -> Result<()> {
        let elf = build_example_program("pairing_miller_step")?;
        let executor = VmExecutor::<F>::new(
            VmConfig::rv32im().add_pairing_support(vec![PairingCurve::Bls12_381]),
        );

        let mut rng = rand::rngs::StdRng::seed_from_u64(88);
        let S = G2Affine::random(&mut rng);
        let Q = G2Affine::random(&mut rng);

        let s = AffinePoint::new(S.x, S.y);
        let q = AffinePoint::new(Q.x, Q.y);

        // Test miller_double_step
        let (pt, l) = Bls12_381::miller_double_step(&s);
        let io0 = [s.x, s.y, pt.x, pt.y, l.b, l.c]
            .into_iter()
            .flat_map(|fp| fp.to_bytes())
            .map(AbstractField::from_canonical_u8)
            .collect::<Vec<_>>();

        // Test miller_double_and_add_step
        let (pt, l0, l1) = Bls12_381::miller_double_and_add_step(&s, &q);
        let io1 = [s.x, s.y, q.x, q.y, pt.x, pt.y, l0.b, l0.c, l1.b, l1.c]
            .into_iter()
            .flat_map(|fp| fp.to_bytes())
            .map(AbstractField::from_canonical_u8)
            .collect::<Vec<_>>();

        let io_all = io0.into_iter().chain(io1).collect::<Vec<_>>();

        executor.execute(elf, vec![io_all])?;
        Ok(())
    }

    #[test]
    fn test_bls12_381_miller_loop() -> Result<()> {
        let elf = build_example_program("pairing_miller_loop")?;

        // TODO[yj]: Unfortunate workaround until MOD_IDX issue is resolved
        // let exe = axvm_circuit::arch::instructions::exe::AxVmExe::<F>::from(elf.clone());
        // let mut enabled_moduli = exe
        //     .custom_op_config
        //     .intrinsics
        //     .field_arithmetic
        //     .primes
        //     .iter()
        //     .map(|s| num_bigint_dig::BigUint::from_str(s).unwrap())
        //     .collect::<Vec<_>>();
        let enabled_moduli = vec![
            BLS12381.MODULUS.clone() + num_bigint_dig::BigUint::from(3u64),
            BLS12381.MODULUS.clone() + num_bigint_dig::BigUint::from(2u64),
            BLS12381.MODULUS.clone() + num_bigint_dig::BigUint::from(0u64),
            BLS12381.MODULUS.clone() + num_bigint_dig::BigUint::from(1u64),
        ];

        let executor = VmExecutor::<F>::new(
            VmConfig::rv32im()
                .add_pairing_support(vec![PairingCurve::Bls12_381])
                .add_ecc_support(vec![EcCurve::Bls12_381])
                .add_modular_support(enabled_moduli.clone())
                .add_complex_ext_support(enabled_moduli),
        );

        let S = G1Affine::generator();
        let Q = G2Affine::generator();

        let mut S_mul = [
            G1Affine::from(S * Fr::from(1)),
            G1Affine::from(S * Fr::from(2)),
        ];
        S_mul[1].y = -S_mul[1].y;
        let Q_mul = [
            G2Affine::from(Q * Fr::from(2)),
            G2Affine::from(Q * Fr::from(1)),
        ];

        let s = S_mul.map(|s| AffinePoint::new(s.x, s.y));
        let q = Q_mul.map(|p| AffinePoint::new(p.x, p.y));

        // Test miller_loop
        let f = Bls12_381::multi_miller_loop(&s, &q);
        let io0 = s
            .into_iter()
            .flat_map(|pt| [pt.x, pt.y].into_iter().flat_map(|fp| fp.to_bytes()))
            .map(AbstractField::from_canonical_u8)
            .collect::<Vec<_>>();

        let io1 = q
            .into_iter()
            .flat_map(|pt| [pt.x, pt.y].into_iter())
            .chain(f.to_coeffs())
            .flat_map(|fp2| fp2.to_coeffs())
            .flat_map(|fp| fp.to_bytes())
            .map(AbstractField::from_canonical_u8)
            .collect::<Vec<_>>();

        let io_all = io0.into_iter().chain(io1).collect::<Vec<_>>();

        executor.execute(elf, vec![io_all])?;
        Ok(())
    }

    #[test]
    fn test_bls12_381_final_exp_hint() -> Result<()> {
        let elf = build_example_program("final_exp_hint")?;
        let executor = VmExecutor::<F>::new(VmConfig::rv32im());

        let P = G1Affine::generator();
        let Q = G2Affine::generator();
        let ps = vec![AffinePoint::new(P.x, P.y), AffinePoint::new(P.x, -P.y)];
        let qs = vec![AffinePoint::new(Q.x, Q.y), AffinePoint::new(Q.x, Q.y)];
        let f = Bls12_381::multi_miller_loop(&ps, &qs);
        let (c, s) = Bls12_381::final_exp_hint(&f);
        let io = [f, c, s]
            .into_iter()
            .flat_map(|fp12| fp12.to_coeffs())
            .flat_map(|fp2| fp2.to_coeffs())
            .flat_map(|fp| fp.to_bytes())
            .map(AbstractField::from_canonical_u8)
            .collect::<Vec<_>>();
        executor.execute(elf, vec![io])?;
        Ok(())
    }
}
