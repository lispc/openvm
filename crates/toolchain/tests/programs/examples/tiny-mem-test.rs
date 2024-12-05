#![cfg_attr(not(feature = "std"), no_main)]
#![cfg_attr(not(feature = "std"), no_std)]

use axvm::io::print;
axvm::entry!(main);

// fn test() -> std::result::Result<(), ()> {
//     Ok(())
// }

pub fn main() {
    panic!();
    // match test() {
    //     Ok(_) => {
    //         println!("ok");
    //     }
    //     Err(_) => {
    //         println!("err");
    //     }
    // }
    // panic!();
}
