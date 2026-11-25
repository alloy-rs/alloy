use alloc::boxed::Box;
use alloy_primitives::hex;
use core::fmt;
use derive_more::{AsMut, AsRef, Deref, DerefMut};

pub use c_kzg::{BYTES_PER_COMMITMENT as BYTES_PER_G1_POINT, FIELD_ELEMENTS_PER_BLOB};

/// Bytes per G2 point.
pub const BYTES_PER_G2_POINT: usize = 96;

/// Number of G1 Points.
pub const NUM_G1_POINTS: usize = 4096;

/// Number of G2 Points.
pub const NUM_G2_POINTS: usize = 65;

/// A newtype over list of G1 point from kzg trusted setup.
#[derive(Clone, Debug, PartialEq, Eq, AsRef, AsMut, Deref, DerefMut)]
#[repr(transparent)]
pub struct G1Points(pub [[u8; BYTES_PER_G1_POINT]; NUM_G1_POINTS]);

impl Default for G1Points {
    fn default() -> Self {
        Self([[0; BYTES_PER_G1_POINT]; NUM_G1_POINTS])
    }
}

/// A newtype over list of G2 point from kzg trusted setup.
#[derive(Clone, Debug, PartialEq, Eq, AsRef, AsMut, Deref, DerefMut)]
#[repr(transparent)]
pub struct G2Points(pub [[u8; BYTES_PER_G2_POINT]; NUM_G2_POINTS]);

impl Default for G2Points {
    fn default() -> Self {
        Self([[0; BYTES_PER_G2_POINT]; NUM_G2_POINTS])
    }
}

/// Parses the contents of a KZG trusted setup file into a list of G1 and G2 points.
///
/// These can then be used to create a KZG settings object with
/// [`KzgSettings::load_trusted_setup`](c_kzg::KzgSettings::load_trusted_setup).
pub fn parse_kzg_trusted_setup(
    trusted_setup: &str,
) -> Result<(Box<G1Points>, Box<G2Points>), KzgErrors> {
    let mut lines = trusted_setup.lines();

    // load number of points
    let n_g1 = lines
        .next()
        .ok_or(KzgErrors::FileFormatError)?
        .parse::<usize>()
        .map_err(|_| KzgErrors::ParseError)?;
    let n_g2 = lines
        .next()
        .ok_or(KzgErrors::FileFormatError)?
        .parse::<usize>()
        .map_err(|_| KzgErrors::ParseError)?;

    if n_g1 != NUM_G1_POINTS {
        return Err(KzgErrors::MismatchedNumberOfPoints);
    }

    if n_g2 != NUM_G2_POINTS {
        return Err(KzgErrors::MismatchedNumberOfPoints);
    }

    // load g1 points
    let mut g1_points = Box::<G1Points>::default();
    for bytes in &mut g1_points.0 {
        let line = lines.next().ok_or(KzgErrors::FileFormatError)?;
        hex::decode_to_slice(line, bytes).map_err(KzgErrors::InvalidPoint)?;
    }

    // load g2 points
    let mut g2_points = Box::<G2Points>::default();
    for bytes in &mut g2_points.0 {
        let line = lines.next().ok_or(KzgErrors::FileFormatError)?;
        hex::decode_to_slice(line, bytes).map_err(KzgErrors::InvalidPoint)?;
    }

    if lines.next().is_some() {
        return Err(KzgErrors::FileFormatError);
    }

    Ok((g1_points, g2_points))
}

/// KZG custom Error types
#[derive(Clone, Copy, Debug)]
pub enum KzgErrors {
    /// Failed to get current directory.
    FailedCurrentDirectory,
    /// The specified path does not exist.
    PathNotExists,
    /// Problems related to I/O.
    IOError,
    /// Not a valid file.
    NotValidFile,
    /// File is not properly formatted.
    FileFormatError,
    /// Not able to parse to usize.
    ParseError,
    /// Number of points does not match what is expected.
    MismatchedNumberOfPoints,
    /// A point could not be decoded from hexadecimal.
    InvalidPoint(hex::FromHexError),
}

impl fmt::Display for KzgErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::FailedCurrentDirectory => "failed to get current directory",
            Self::PathNotExists => "the specified path does not exist",
            Self::IOError => "IO error",
            Self::NotValidFile => "not a valid file",
            Self::FileFormatError => "file is not properly formatted",
            Self::ParseError => "could not parse as usize",
            Self::MismatchedNumberOfPoints => "number of points does not match what is expected",
            Self::InvalidPoint(err) => return write!(f, "could not decode point: {err}"),
        };
        f.write_str(s)
    }
}

impl core::error::Error for KzgErrors {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::InvalidPoint(err) => Some(err),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{format, string::ToString};
    use core::error::Error;

    #[test]
    fn retains_invalid_point_character() {
        let trusted_setup =
            format!("{NUM_G1_POINTS}\n{NUM_G2_POINTS}\n{}", "z".repeat(BYTES_PER_G1_POINT * 2));
        let err = parse_kzg_trusted_setup(&trusted_setup).unwrap_err();
        assert!(matches!(
            err,
            KzgErrors::InvalidPoint(hex::FromHexError::InvalidHexCharacter { c: 'z', index: 0 })
        ));
    }

    #[test]
    fn distinguishes_count_and_point_errors() {
        assert!(matches!(parse_kzg_trusted_setup("not a count\n65"), Err(KzgErrors::ParseError)));
        for valid_g1_points in [0, NUM_G1_POINTS] {
            let trusted_setup = format!(
                "{NUM_G1_POINTS}\n{NUM_G2_POINTS}\n{}zz",
                format!("{}\n", "00".repeat(BYTES_PER_G1_POINT)).repeat(valid_g1_points)
            );
            let err = parse_kzg_trusted_setup(&trusted_setup).unwrap_err();
            assert!(matches!(err, KzgErrors::InvalidPoint(hex::FromHexError::InvalidStringLength)));
            assert_eq!(err.to_string(), "could not decode point: invalid string length");
            assert_eq!(
                err.source().unwrap().downcast_ref::<hex::FromHexError>(),
                Some(&hex::FromHexError::InvalidStringLength)
            );
        }
    }
}
