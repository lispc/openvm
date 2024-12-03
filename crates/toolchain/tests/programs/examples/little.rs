#![cfg_attr(not(feature = "std"), no_main)]
#![cfg_attr(not(feature = "std"), no_std)]

use axvm_algebra_guest::{DivUnsafe, IntMod};

axvm::entry!(main);

axvm_algebra_moduli_setup::moduli_declare! {
    Secp256k1Coord { modulus = "0xFFFFFFFF FFFFFFFF FFFFFFFF FFFFFFFF FFFFFFFF FFFFFFFF FFFFFFFE FFFFFC2F" }
}

axvm_algebra_moduli_setup::moduli_init!(
    "0xFFFFFFFF FFFFFFFF FFFFFFFF FFFFFFFF FFFFFFFF FFFFFFFF FFFFFFFE FFFFFC2F"
);
extern crate alloc;
use alloc::string::ToString;
use core::ops::MulAssign;
pub fn main() {
    setup_all_moduli();
    let mut pow = Secp256k1Coord::MODULUS;
    pow[0] -= 2;

    let mut a = Secp256k1Coord::from_u32(1234);
    let mut res = Secp256k1Coord::from_u32(1);
    let inv = res.clone().div_unsafe(&a);

    // This can fix, but don't know why
    // assert_ne!(res, Secp256k1Coord::from_u32(0));

    for i in 0..32 {
        for j in 0..8 {
            if pow[i] & (1 << j) != 0 {
                res.mul_assign(&a); // ok

                res = res * &a; // not ok, unless print the uninit
            }
            a.square_assign(); // ok

            // a *= a.clone(); // not ok

            // let aa = a.clone(); // needs print
            // axvm::io::print((&aa as *const Secp256k1Coord as usize).to_string());
            // a *= aa;
        }
    }

    // https://en.wikipedia.org/wiki/Fermat%27s_little_theorem
    assert_eq!(res, inv);

    let two = Secp256k1Coord::from_u32(2);
    let minus_two = Secp256k1Coord::from_le_bytes(&pow);

    assert_eq!(res - &minus_two, inv + &two);

    if two == minus_two {
        axvm::process::panic();
    }
}
