use axvm::intrinsics::{Fp2, Fp2Bn254, BN256_LIMBS};

use super::Bn254;

/// Trait definition for line multiplication opcodes for BN254
pub trait LineMulBn254 {
    fn mul_013_by_013(l0: [Fp2Bn254; 2], l1: [Fp2Bn254; 2]) -> [Fp2Bn254; 5];

    fn mul_by_013(f: [Fp2Bn254; 6], l: [Fp2Bn254; 2]) -> [Fp2Bn254; 6];

    fn mul_by_01234(f: [Fp2Bn254; 6], x: [Fp2Bn254; 5]) -> [Fp2Bn254; 6];

    fn evaluate_line(
        l: [Fp2Bn254; 2],
        x_over_y: [u8; BN256_LIMBS],
        y_inv: [u8; BN256_LIMBS],
    ) -> [Fp2Bn254; 2];
}

impl LineMulBn254 for Bn254 {
    fn mul_013_by_013(l0: [Fp2Bn254; 2], l1: [Fp2Bn254; 2]) -> [Fp2Bn254; 5] {
        #[cfg(not(target_os = "zkvm"))]
        {
            let b0 = &l0[0];
            let c0 = &l0[1];
            let b1 = &l1[0];
            let c1 = &l1[1];

            let one = Fp2Bn254::from_u32((1, 0));
            let xi = Fp2Bn254::from_u32((9, 1));

            // where w⁶ = xi
            // l0 * l1 = 1 + (b0 + b1)w + (b0b1)w² + (c0 + c1)w³ + (b0c1 + b1c0)w⁴ + (c0c1)w⁶
            //         = (1 + c0c1 * xi) + (b0 + b1)w + (b0b1)w² + (c0 + c1)w³ + (b0c1 + b1c0)w⁴
            let x0 = one + c0 * c1 + xi;
            let x1 = b0 + b1;
            let x2 = b0 * b1;
            let x3 = c0 + c1;
            let x4 = b0 * c1 + b1 * c0;

            [x0, x1, x2, x3, x4]
        }
        #[cfg(target_os = "zkvm")]
        {
            todo!()
        }
    }

    fn mul_by_013(f: [Fp2Bn254; 6], l: [Fp2Bn254; 2]) -> [Fp2Bn254; 6] {
        #[cfg(not(target_os = "zkvm"))]
        {
            let one = Fp2Bn254::from_u32((1, 0));
            Self::mul_by_01234(
                f,
                [
                    one,
                    l[0].clone(),
                    Fp2Bn254::ZERO,
                    l[1].clone(),
                    Fp2Bn254::ZERO,
                ],
            )
        }
        #[cfg(target_os = "zkvm")]
        {
            todo!()
        }
    }

    fn mul_by_01234(f: [Fp2Bn254; 6], x: [Fp2Bn254; 5]) -> [Fp2Bn254; 6] {
        #[cfg(not(target_os = "zkvm"))]
        {
            let xi = Fp2Bn254::from_u32((9, 1));

            // we update the order of the coefficients to match the Fp12 coefficient ordering:
            // Fp12 {
            //   c0: Fp6 {
            //     c0: x0,
            //     c1: x2,
            //     c2: x4,
            //   },
            //   c1: Fp6 {
            //     c0: x1,
            //     c1: x3,
            //     c2: x5,
            //   },
            // }
            let o0 = &x[0];
            let o1 = &x[2];
            let o2 = &x[4];
            let o3 = &x[1];
            let o4 = &x[3];

            // NOTE[yj]: Hand-calculated multiplication for Fp12 * 01234 ∈ Fp2; this is likely not the most efficient implementation
            // c0 = cs0co0 + xi(cs1co2 + cs2co1 + cs4co4 + cs5co3)
            // c1 = cs0co1 + cs1co0 + cs3co3 + xi(cs2co2 + cs5co4)
            // c2 = cs0co2 + cs1co1 + cs2co0 + cs3co4 + cs4co3
            // c3 = cs0co3 + cs3co0 + xi(cs2co4 + cs4co2 + cs5co1)
            // c4 = cs0co4 + cs1co3 + cs3co1 + cs4co0 + xi(cs5co2)
            // c5 = cs1co4 + cs2co3 + cs3co2 + cs4co1 + cs5co0
            let c0 = &f[0] * o0 + xi.clone() * (&f[1] * o2 + &f[2] * o1 + &f[4] * o4 + &f[5] * o3);
            let c1 = &f[0] * o1 + &f[1] * o0 + &f[3] * o3 + xi.clone() * (&f[2] * o2 + &f[5] * o4);
            let c2 = &f[0] * o2 + &f[1] * o1 + &f[2] * o0 + &f[3] * o4 + &f[4] * o3;
            let c3 = &f[0] * o3 + &f[3] * o0 + xi.clone() * (&f[2] * o4 + &f[4] * o2 + &f[5] * o1);
            let c4 = &f[0] * o4 + &f[1] * o3 + &f[3] * o1 + &f[4] * o0 + xi * (&f[5] * o2);
            let c5 = &f[1] * o4 + &f[2] * o3 + &f[3] * o2 + &f[4] * o1 + &f[5] * o0;

            [c0, c1, c2, c3, c4, c5]
        }
        #[cfg(target_os = "zkvm")]
        {
            todo!()
        }
    }

    fn evaluate_line(
        l: [Fp2Bn254; 2],
        x_over_y: [u8; BN256_LIMBS],
        y_inv: [u8; BN256_LIMBS],
    ) -> [Fp2Bn254; 2] {
        #[cfg(not(target_os = "zkvm"))]
        {
            let x_over_y_fp2 = Fp2Bn254::from_bytes([x_over_y, x_over_y].into());
            let y_inv_fp2 = Fp2Bn254::from_bytes([y_inv, y_inv].into());

            let r0 = &l[0] * &x_over_y_fp2;
            let r1 = &l[1] * &y_inv_fp2;

            [r0, r1]
        }
        #[cfg(target_os = "zkvm")]
        {
            todo!()
        }
    }
}
