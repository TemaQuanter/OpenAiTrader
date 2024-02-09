// This is a program to test float number precision.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn main() {
    let a = dec!(0.2);
    let b = dec!(3.0);

    let c = a / b;

    println!("{}", c);
} // end fn main
