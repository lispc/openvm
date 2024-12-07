#![cfg_attr(not(feature = "std"), no_main)]
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

axvm::entry!(main);

pub fn main() {
    let mut v = alloc::vec::Vec::new();
    for i in 1..10 {
        let tmp = (0..i).collect::<alloc::vec::Vec<_>>();
        v.extend(tmp);
    }
    assert_eq!(v.len(), 45);
}
