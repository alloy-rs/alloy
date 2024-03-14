//! Example of using math utilities to handle big numbers in 'wei' units.

use alloy_primitives::{
    utils::{format_units, parse_units},
    U256,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    parse_units_example()?;
    format_units_example()?;

    Ok(())
}

/// dApps business logics handle big numbers in 'wei' units (i.e. sending transactions, on-chain
/// math, etc.). We provide convenient methods to map user inputs (usually in 'ether' or 'gwei')
/// into 'wei' format.
fn parse_units_example() -> Result<(), Box<dyn std::error::Error>> {
    let pu = parse_units("1.0", "wei")?;
    let num: U256 = pu.into();
    assert_eq!(num, U256::from(1));

    let pu = parse_units("1.0", "kwei")?;
    let num: U256 = pu.into();
    assert_eq!(num, U256::from(1000));

    let pu = parse_units("1.0", "mwei")?;
    let num: U256 = pu.into();
    assert_eq!(num, U256::from(1000000));

    let pu = parse_units("1.0", "gwei")?;
    let num: U256 = pu.into();
    assert_eq!(num, U256::from(1000000000));

    let pu = parse_units("1.0", "szabo")?;
    let num: U256 = pu.into();
    assert_eq!(num, U256::from(1000000000000_u128));

    let pu = parse_units("1.0", "finney")?;
    let num: U256 = pu.into();
    assert_eq!(num, U256::from(1000000000000000_u128));

    let pu = parse_units("1.0", "ether")?;
    let num: U256 = pu.into();
    assert_eq!(num, U256::from(1000000000000000000_u128));

    let pu = parse_units("1.0", 18)?;
    let num: U256 = pu.into();
    assert_eq!(num, U256::from(1000000000000000000_u128));

    Ok(())
}

/// dApps business logics handle big numbers in 'wei' units (i.e. sending transactions, on-chain
/// math, etc.). On the other hand it is useful to convert big numbers into user readable formats
/// when displaying on a UI. Generally dApps display numbers in 'ether' and 'gwei' units,
/// respectively for displaying amounts and gas. The `format_units` function will format a big
/// number into a user readable string.
fn format_units_example() -> Result<(), Box<dyn std::error::Error>> {
    // 1 ETHER = 10^18 WEI
    let one_ether = U256::from(1000000000000000000_u128);

    let num: String = format_units(one_ether, "wei")?;
    assert_eq!(num, "1000000000000000000.0");

    let num: String = format_units(one_ether, "gwei")?;
    assert_eq!(num, "1000000000.000000000");

    let num: String = format_units(one_ether, "ether")?;
    assert_eq!(num, "1.000000000000000000");

    // 1 GWEI = 10^9 WEI
    let one_gwei = U256::from(1000000000_u128);

    let num: String = format_units(one_gwei, 0)?;
    assert_eq!(num, "1000000000.0");

    let num: String = format_units(one_gwei, "wei")?;
    assert_eq!(num, "1000000000.0");

    let num: String = format_units(one_gwei, "kwei")?;
    assert_eq!(num, "1000000.000");

    let num: String = format_units(one_gwei, "mwei")?;
    assert_eq!(num, "1000.000000");

    let num: String = format_units(one_gwei, "gwei")?;
    assert_eq!(num, "1.000000000");

    let num: String = format_units(one_gwei, "szabo")?;
    assert_eq!(num, "0.001000000000");

    let num: String = format_units(one_gwei, "finney")?;
    assert_eq!(num, "0.000001000000000");

    let num: String = format_units(one_gwei, "ether")?;
    assert_eq!(num, "0.000000001000000000");

    Ok(())
}
