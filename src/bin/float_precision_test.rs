// This is a program to test float number precision.

use rust_decimal::Decimal;

fn main() {
    let a = Decimal::from_str_exact("48.4").unwrap();
    let c = Decimal::from_str_exact("0.1").unwrap();

    let b = Decimal::floor(&(a / c)) * c;

    println!("{}", b);
} // end fn main
