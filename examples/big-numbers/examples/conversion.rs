//! Example of converting `U256` to native Rust types.

use alloy_primitives::{utils::format_units, U256};

/// `U256` provides useful conversion functions to enable transformation into native Rust types.
///
/// It is important to note that converting a big-number to a floating point type (such as a `f32`
/// or `f64`) can result in a loss of precision, since you cannot fit 256 bits of information into
/// 64 bits.
///
/// However, there may be cases where you want to perform conversions for presentation purposes.
/// For example, you may want to display a large number to the user in a more readable format.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let num = U256::from(42_u8);

    let a: u128 = num.to::<u128>();
    assert_eq!(a, 42);

    let b: u64 = num.to::<u64>();
    assert_eq!(b, 42);

    let c: u32 = num.to::<u32>();
    assert_eq!(c, 42);

    let d: usize = num.to::<usize>();
    assert_eq!(d, 42);

    let e: String = num.to_string();
    assert_eq!(e, "42");

    let f: String = format_units(num, 4)?;
    assert_eq!(f, "0.0042");

    Ok(())
}
