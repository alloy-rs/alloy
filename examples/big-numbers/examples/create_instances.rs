//! Example of creating instances of `U256` from strings and numbers.

use alloy_primitives::{
    utils::{parse_units, ParseUnits},
    U256,
};
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // From strings
    let a = U256::from_str("42")?;
    assert_eq!(a.to_string(), "42");

    let amount = "42";
    let units = 4;
    let b: ParseUnits = parse_units(amount, units)?;
    assert_eq!(b.to_string(), "420000");

    // From numbers
    let c = U256::from(42_u8);
    assert_eq!(c.to_string(), "42");

    let d = U256::from(42_u16);
    assert_eq!(d.to_string(), "42");

    let e = U256::from(42_u32);
    assert_eq!(e.to_string(), "42");

    let f = U256::from(42_u64);
    assert_eq!(f.to_string(), "42");

    let g = U256::from(42_u128);
    assert_eq!(g.to_string(), "42");

    let h = U256::from(0x2a);
    assert_eq!(h.to_string(), "42");

    let i = U256::from(42);
    assert_eq!(i.to_string(), "42");

    Ok(())
}
