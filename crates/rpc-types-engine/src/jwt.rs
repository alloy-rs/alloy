use alloy_primitives::hex;
use jsonwebtoken::{decode, errors::ErrorKind, Algorithm, DecodingKey, Validation};
use rand::Rng;
use std::time::Duration;
use thiserror::Error;

/// Errors returned by the [`JwtSecret`]
#[derive(Error, Debug)]
pub enum JwtError {
    /// An error encountered while decoding the hexadecimal string for the JWT secret.
    #[error(transparent)]
    JwtSecretHexDecodeError(#[from] hex::FromHexError),

    /// The JWT key length provided is invalid, expecting a specific length.
    #[error("JWT key is expected to have a length of {0} digits. {1} digits key provided")]
    InvalidLength(usize, usize),

    /// The signature algorithm used in the JWT is not supported. Only HS256 is supported.
    #[error("unsupported signature algorithm. Only HS256 is supported")]
    UnsupportedSignatureAlgorithm,

    /// The provided signature in the JWT is invalid.
    #[error("provided signature is invalid")]
    InvalidSignature,

    /// The "iat" (issued-at) claim in the JWT is not within the allowed ±60 seconds from the
    /// current time.
    #[error("IAT (issued-at) claim is not within ±60 seconds from the current time")]
    InvalidIssuanceTimestamp,

    /// The Authorization header is missing or invalid in the context of JWT validation.
    #[error("Authorization header is missing or invalid")]
    MissingOrInvalidAuthorizationHeader,

    /// An error occurred during JWT decoding.
    #[error("JWT decoding error: {0}")]
    JwtDecodingError(String),
}

/// Length of the hex-encoded 256 bit secret key.
/// A 256-bit encoded string in Rust has a length of 64 digits because each digit represents 4 bits
/// of data. In hexadecimal representation, each digit can have 16 possible values (0-9 and A-F), so
/// 4 bits can be represented using a single hex digit. Therefore, to represent a 256-bit string,
/// we need 64 hexadecimal digits (256 bits ÷ 4 bits per digit = 64 digits).
const JWT_SECRET_LEN: usize = 64;

/// The JWT `iat` (issued-at) claim cannot exceed +-60 seconds from the current time.
const JWT_MAX_IAT_DIFF: Duration = Duration::from_secs(60);

/// The execution layer client MUST support at least the following alg HMAC + SHA256 (HS256)
const JWT_SIGNATURE_ALGO: Algorithm = Algorithm::HS256;

/// Value-object holding a reference to a hex-encoded 256-bit secret key.
/// A JWT secret key is used to secure JWT-based authentication. The secret key is
/// a shared secret between the server and the client and is used to calculate a digital signature
/// for the JWT, which is included in the JWT along with its payload.
///
/// See also: [Secret key - Engine API specs](https://github.com/ethereum/execution-apis/blob/main/src/engine/authentication.md#key-distribution)
#[derive(Clone, PartialEq, Eq)]
pub struct JwtSecret([u8; 32]);

impl JwtSecret {
    /// Creates an instance of [`JwtSecret`].
    ///
    /// Returns an error if one of the following applies:
    /// - `hex` is not a valid hexadecimal string
    /// - `hex` argument length is less than `JWT_SECRET_LEN`
    ///
    /// This strips the leading `0x`, if any.
    pub fn from_hex<S: AsRef<str>>(hex: S) -> Result<Self, JwtError> {
        let hex: &str = hex.as_ref().trim().trim_start_matches("0x");
        if hex.len() != JWT_SECRET_LEN {
            Err(JwtError::InvalidLength(JWT_SECRET_LEN, hex.len()))
        } else {
            let hex_bytes = hex::decode(hex)?;
            // is 32bytes, see length check
            let bytes = hex_bytes.try_into().expect("is expected len");
            Ok(JwtSecret(bytes))
        }
    }
}
