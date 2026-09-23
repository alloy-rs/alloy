//! Contains the builder deposit and exit system contracts and request types, first introduced in
//! the Amsterdam hardfork.
//!
//! See also [EIP-8282](https://eips.ethereum.org/EIPS/eip-8282): Builder Execution Requests

use alloy_primitives::{address, bytes, Address, Bytes, FixedBytes, B256};

/// The address for the EIP-8282 builder deposit contract.
pub const BUILDER_DEPOSIT_CONTRACT_ADDRESS: Address =
    address!("0x0000BFF46984E3725691FA540A8C7589300D8282");

/// The code for the EIP-8282 builder deposit contract.
pub static BUILDER_DEPOSIT_CONTRACT_CODE: Bytes = bytes!("0x3373fffffffffffffffffffffffffffffffffffffffe1461011c575f54807fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff146102705760015460088111605257506058565b60089003015b601190600182026001905f5b5f821115607f57810190830284830290049160010191906064565b90939004925050503660b814609f57366102705734610270575f5260205ff35b8034106102705760383567ffffffffffffffff1680633b9aca001161027057633b9aca00029034031061027057600154600101600155600354806006026004015f358155600101602035815560010160403581556001016060358155600101608035815560010160a035905560b85f5f3760b85fa0600101600355005b60035460025480820380604011610131575060405b5f5b8181146101d7578281016006026004018160b8028154815260200181600101548152602001816002015480825260401c67ffffffffffffffff16816010018160381c81600701538160301c81600601538160281c81600501538160201c81600401538160181c81600301538160101c81600201538160081c816001015353602001816003015481526020018160040154815260200190600501549052600101610133565b91018092146101e957906002556101f4565b90505f6002555f6003555b36610242575f54600154817fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff1461023057600882820111610238575b50505f610264565b0160089003610264565b7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff5b5f555f60015560b8025ff35b5f5ffd");

/// The address for the EIP-8282 builder exit contract.
pub const BUILDER_EXIT_CONTRACT_ADDRESS: Address =
    address!("0x000064D678505AD48F8CCB093BC65613800E8282");

/// The code for the EIP-8282 builder exit contract.
pub static BUILDER_EXIT_CONTRACT_CODE: Bytes = bytes!("0x3373fffffffffffffffffffffffffffffffffffffffe1460e1575f54807fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff146101c65760015460028111605157506057565b60029003015b601190600182026001905f5b5f821115607e57810190830284830290049160010191906063565b909390049250505036603014609e57366101c657346101c6575f5260205ff35b34106101c657600154600101600155600354806003026004013381556001015f35815560010160203590553360601b5f5260305f60143760445fa0600101600355005b6003546002548082038060101160f5575060105b5f5b81811461012d5782810160030260040181604402815460601b8152601401816001015481526020019060020154905260010160f7565b910180921461013f579060025561014a565b90505f6002555f6003555b36610198575f54600154817fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff146101865760028282011161018e575b50505f6101ba565b01600290036101ba565b7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff5b5f555f6001556044025ff35b5f5ffd");

/// The [EIP-7685](https://eips.ethereum.org/EIPS/eip-7685) request type for builder deposit
/// requests.
pub const BUILDER_DEPOSIT_REQUEST_TYPE: u8 = 0x03;

/// The [EIP-7685](https://eips.ethereum.org/EIPS/eip-7685) request type for builder exit requests.
pub const BUILDER_EXIT_REQUEST_TYPE: u8 = 0x04;

/// The [EIP-8282](https://eips.ethereum.org/EIPS/eip-8282) defined maximum builder deposit requests per block.
pub const MAX_BUILDER_DEPOSIT_REQUESTS_PER_BLOCK: usize = 64;

/// The [EIP-8282](https://eips.ethereum.org/EIPS/eip-8282) defined maximum builder exit requests per block.
pub const MAX_BUILDER_EXIT_REQUESTS_PER_BLOCK: usize = 16;

/// This structure maps onto the builder deposit request object from
/// [EIP-8282](https://eips.ethereum.org/EIPS/eip-8282).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct BuilderDepositRequest {
    /// Builder public key
    pub pubkey: FixedBytes<48>,
    /// Withdrawal credentials
    pub withdrawal_credentials: B256,
    /// Amount of ether deposited in gwei
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::displayfromstr"))]
    pub amount: u64,
    /// Deposit signature
    pub signature: FixedBytes<96>,
}

/// This structure maps onto the builder exit request object from
/// [EIP-8282](https://eips.ethereum.org/EIPS/eip-8282).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub struct BuilderExitRequest {
    /// Address of the source of the exit
    pub source_address: Address,
    /// Builder public key
    pub pubkey: FixedBytes<48>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{b256, hex};

    #[test]
    #[cfg(feature = "serde")]
    fn test_serde_builder_deposit_request() {
        // Sample JSON input representing a builder deposit request
        let json_data = r#"{
            "pubkey":"0x8e8d8749f6bc79b78be7cc6e49ff640e608454840c360b344c3a4d9b7428e280e7f40d2271bad65d8cbbfdd43cb8793b",
            "withdrawal_credentials":"0x0100000000000000000000000ae0e8770147aaa6828a0d6f642504663f10f7d1",
            "amount":"32000000000",
            "signature":"0xb3c0d1a2f4e5968778190a2b3c4d5e6f708192a3b4c5d6e7f80910a2b3c4d5e6b3c0d1a2f4e5968778190a2b3c4d5e6f708192a3b4c5d6e7f80910a2b3c4d5e6b3c0d1a2f4e5968778190a2b3c4d5e6f708192a3b4c5d6e7f80910a2b3c4d5e6"
        }"#;

        // Deserialize the JSON into a BuilderDepositRequest struct
        let deposit_request: BuilderDepositRequest =
            serde_json::from_str(json_data).expect("Failed to deserialize");

        // Verify the deserialized content
        assert_eq!(
            deposit_request.pubkey,
            FixedBytes::<48>::from(hex!("8e8d8749f6bc79b78be7cc6e49ff640e608454840c360b344c3a4d9b7428e280e7f40d2271bad65d8cbbfdd43cb8793b"))
        );
        assert_eq!(
            deposit_request.withdrawal_credentials,
            b256!("0100000000000000000000000ae0e8770147aaa6828a0d6f642504663f10f7d1")
        );
        assert_eq!(deposit_request.amount, 32_000_000_000);

        // Serialize the struct back into JSON
        let serialized_json = serde_json::to_string(&deposit_request).expect("Failed to serialize");

        // Check if the serialized JSON matches the expected JSON structure
        let expected_json = r#"{"pubkey":"0x8e8d8749f6bc79b78be7cc6e49ff640e608454840c360b344c3a4d9b7428e280e7f40d2271bad65d8cbbfdd43cb8793b","withdrawal_credentials":"0x0100000000000000000000000ae0e8770147aaa6828a0d6f642504663f10f7d1","amount":"32000000000","signature":"0xb3c0d1a2f4e5968778190a2b3c4d5e6f708192a3b4c5d6e7f80910a2b3c4d5e6b3c0d1a2f4e5968778190a2b3c4d5e6f708192a3b4c5d6e7f80910a2b3c4d5e6b3c0d1a2f4e5968778190a2b3c4d5e6f708192a3b4c5d6e7f80910a2b3c4d5e6"}"#;
        assert_eq!(serialized_json, expected_json);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_serde_builder_exit_request() {
        // Sample JSON input representing a builder exit request
        let json_data = r#"{
            "source_address":"0xAE0E8770147AaA6828a0D6f642504663F10F7d1E",
            "pubkey":"0x8e8d8749f6bc79b78be7cc6e49ff640e608454840c360b344c3a4d9b7428e280e7f40d2271bad65d8cbbfdd43cb8793b"
        }"#;

        // Deserialize the JSON into a BuilderExitRequest struct
        let exit_request: BuilderExitRequest =
            serde_json::from_str(json_data).expect("Failed to deserialize");

        // Verify the deserialized content
        assert_eq!(
            exit_request.source_address,
            address!("AE0E8770147AaA6828a0D6f642504663F10F7d1E")
        );
        assert_eq!(
            exit_request.pubkey,
            FixedBytes::<48>::from(hex!("8e8d8749f6bc79b78be7cc6e49ff640e608454840c360b344c3a4d9b7428e280e7f40d2271bad65d8cbbfdd43cb8793b"))
        );

        // Serialize the struct back into JSON
        let serialized_json = serde_json::to_string(&exit_request).expect("Failed to serialize");

        // Check if the serialized JSON matches the expected JSON structure
        let expected_json = r#"{"source_address":"0xae0e8770147aaa6828a0d6f642504663f10f7d1e","pubkey":"0x8e8d8749f6bc79b78be7cc6e49ff640e608454840c360b344c3a4d9b7428e280e7f40d2271bad65d8cbbfdd43cb8793b"}"#;
        assert_eq!(serialized_json, expected_json);
    }
}
