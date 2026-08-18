use alloy_primitives::{Address, B256, Bytes};
use alloy_rlp::{RlpDecodable, RlpEncodable};

#[derive(Clone, Debug, PartialEq, Eq, RlpEncodable, RlpDecodable)]
pub struct BasicTypes {
    pub boolean: bool,
    pub u8_value: u8,
    pub u16_value: u16,
    pub u32_value: u32,
    pub u64_value: u64,
    pub u128_value: u128,
    pub bytes: Bytes,
    pub string: String,
    pub address: Address,
    pub hash: B256,
}

#[derive(Clone, Debug, PartialEq, Eq, RlpEncodable, RlpDecodable)]
pub struct VectorTypes {
    pub integers: Vec<u64>,
    pub byte_strings: Vec<Bytes>,
    pub strings: Vec<String>,
    pub basics: Vec<BasicTypes>,
}

#[derive(Clone, Debug, PartialEq, Eq, RlpEncodable, RlpDecodable)]
pub struct ArrayTypes {
    pub bytes: [u8; 32],
    pub integers: Vec<u32>,
    pub hashes: Vec<B256>,
}

#[derive(Clone, Debug, PartialEq, Eq, RlpEncodable, RlpDecodable)]
pub struct NestedTypes {
    pub basic: BasicTypes,
    pub vectors: VectorTypes,
    pub arrays: ArrayTypes,
}

#[derive(Clone, Debug, PartialEq, Eq, RlpEncodable, RlpDecodable)]
#[rlp(trailing(no_gaps))]
pub struct OptionalTypes {
    pub required: u64,
    pub bytes: Bytes,
    pub optional_hash: Option<B256>,
    pub optional_values: Option<Vec<u64>>,
}
