// This is a program to test float number precision.

use rust_decimal::Decimal;

fn main() {
    let a = Some(Decimal::from_str_exact("48.4").unwrap());
    let c = Some(Decimal::from_str_exact("0.1").unwrap());

    a.unwrap();

    let b = a;

    println!("{}", a.unwrap());
} // end fn main
