//! Host-side encoding for the Trezor `EthereumSignTypedData` flow.
//!
//! Firmware major version 2 (Trezor Model T and Safe devices) signs EIP-712 typed data by
//! requesting the struct definitions and every field value from the host while it hashes and
//! displays the data. This mirrors the reference implementation in trezorlib's
//! [`sign_typed_data`](https://github.com/trezor/trezor-firmware/blob/main/python/src/trezorlib/ethereum.py).

use crate::TrezorError;
use alloy_dyn_abi::{eip712::Resolver, DynSolType, DynSolValue, TypedData};
use alloy_sol_types::{Eip712Domain, SolStruct, SolType};
use trezor_client::protos::ethereum_typed_data_struct_ack::{
    EthereumDataType, EthereumFieldType, EthereumStructMember,
};

/// EIP-712 typed data prepared for the Trezor typed-data signing flow.
///
/// The device asks for struct definitions by name and for values by member path, where the root
/// index `0` addresses the domain and `1` the message.
#[derive(Debug)]
pub(crate) struct TrezorTypedData {
    resolver: Resolver,
    primary_type: String,
    domain: DynSolValue,
    message: DynSolValue,
}

impl TrezorTypedData {
    /// Prepares a [`SolStruct`] payload by resolving its EIP-712 type graph and decoding the
    /// ABI-encoded value into a dynamic value tree.
    pub(crate) fn from_struct<T: SolStruct>(
        payload: &T,
        domain: &Eip712Domain,
    ) -> Result<Self, TrezorError> {
        let resolver = Resolver::from_struct::<T>();
        let message = resolver.resolve(T::NAME)?.abi_decode(&abi_encode::<T>(payload))?;
        Self::new(resolver, T::NAME.to_string(), domain, message)
    }

    /// Prepares dynamic [`TypedData`].
    pub(crate) fn from_typed_data(typed_data: &TypedData) -> Result<Self, TrezorError> {
        let message = typed_data.coerce()?;
        Self::new(
            typed_data.resolver.clone(),
            typed_data.primary_type.clone(),
            &typed_data.domain,
            message,
        )
    }

    fn new(
        mut resolver: Resolver,
        primary_type: String,
        domain: &Eip712Domain,
        message: DynSolValue,
    ) -> Result<Self, TrezorError> {
        // The domain type is derived from the populated fields, matching how alloy computes the
        // domain separator, and takes precedence over an `EIP712Domain` definition in the types.
        // A message may only reference `EIP712Domain` as a member type if that definition is the
        // same, since the device hashes and displays the values under the definition it is sent.
        let domain_type = domain.encode_type();
        if primary_type != Eip712Domain::NAME
            && resolver.linearize(&primary_type)?.iter().any(|def| {
                def.type_name() == Eip712Domain::NAME && def.eip712_encode_type() != domain_type
            })
        {
            return Err(TrezorError::UnsupportedTypedData(
                "the message references an `EIP712Domain` type that differs from the signing domain"
                    .to_string(),
            ));
        }
        resolver.ingest_string(domain_type)?;
        Ok(Self { resolver, primary_type, domain: domain_value(domain), message })
    }

    pub(crate) fn primary_type(&self) -> &str {
        &self.primary_type
    }

    /// Answers an `EthereumTypedDataStructRequest` with the members of the struct `name`.
    pub(crate) fn struct_members(
        &self,
        name: &str,
    ) -> Result<Vec<EthereumStructMember>, TrezorError> {
        let DynSolType::CustomStruct { prop_names, tuple, .. } = self.resolver.resolve(name)?
        else {
            return Err(TrezorError::UnsupportedTypedData(format!("`{name}` is not a struct")));
        };
        prop_names
            .iter()
            .zip(&tuple)
            .map(|(prop_name, ty)| {
                let mut member = EthereumStructMember::new();
                member.set_name(prop_name.clone());
                member.type_ = Some(field_type(ty)?).into();
                Ok(member)
            })
            .collect()
    }

    /// Answers an `EthereumTypedDataValueRequest` for `member_path`.
    ///
    /// Atomic values are encoded as the device expects them, while arrays are answered with their
    /// length as a big-endian `u16` before the device requests each element separately.
    pub(crate) fn encode_value(&self, member_path: &[u32]) -> Result<Vec<u8>, TrezorError> {
        let invalid_path = || {
            TrezorError::UnsupportedTypedData(format!(
                "device requested invalid member path {member_path:?}"
            ))
        };
        let (root, path) = member_path.split_first().ok_or_else(invalid_path)?;
        let mut value = match root {
            0 => &self.domain,
            1 => &self.message,
            _ => return Err(invalid_path()),
        };
        for &index in path {
            let children = match value {
                DynSolValue::CustomStruct { tuple, .. } => tuple,
                DynSolValue::Array(items) | DynSolValue::FixedArray(items) => items,
                _ => return Err(invalid_path()),
            };
            value = children.get(index as usize).ok_or_else(invalid_path)?;
        }
        encode_value(value)
    }
}

/// ABI-encodes a value through its [`SolType`].
///
/// Going through the `RustType` projection lets the compiler use the `SolTypeValue` bound declared
/// on the associated type, which is not available once `RustType` is normalized to the
/// [`SolStruct`] itself.
fn abi_encode<S: SolType>(value: &S::RustType) -> Vec<u8> {
    S::abi_encode(value)
}

/// Builds the domain value in the field order of [`Eip712Domain::encode_type`].
fn domain_value(domain: &Eip712Domain) -> DynSolValue {
    let mut prop_names = Vec::new();
    let mut tuple = Vec::new();
    let mut push = |name: &str, value: DynSolValue| {
        prop_names.push(name.to_string());
        tuple.push(value);
    };
    if let Some(name) = &domain.name {
        push("name", DynSolValue::String(name.to_string()));
    }
    if let Some(version) = &domain.version {
        push("version", DynSolValue::String(version.to_string()));
    }
    if let Some(chain_id) = domain.chain_id {
        push("chainId", DynSolValue::Uint(chain_id, 256));
    }
    if let Some(verifying_contract) = domain.verifying_contract {
        push("verifyingContract", DynSolValue::Address(verifying_contract));
    }
    if let Some(salt) = domain.salt {
        push("salt", DynSolValue::FixedBytes(salt, 32));
    }
    DynSolValue::CustomStruct { name: Eip712Domain::NAME.to_string(), prop_names, tuple }
}

/// Converts a resolved type into the device's field type descriptor.
fn field_type(ty: &DynSolType) -> Result<EthereumFieldType, TrezorError> {
    let mut field = EthereumFieldType::new();
    match ty {
        DynSolType::Uint(bits) => {
            field.set_data_type(EthereumDataType::UINT);
            field.set_size((bits / 8) as u32);
        }
        DynSolType::Int(bits) => {
            field.set_data_type(EthereumDataType::INT);
            field.set_size((bits / 8) as u32);
        }
        DynSolType::FixedBytes(size) => {
            field.set_data_type(EthereumDataType::BYTES);
            field.set_size(*size as u32);
        }
        DynSolType::Bytes => field.set_data_type(EthereumDataType::BYTES),
        DynSolType::String => field.set_data_type(EthereumDataType::STRING),
        DynSolType::Bool => field.set_data_type(EthereumDataType::BOOL),
        DynSolType::Address => field.set_data_type(EthereumDataType::ADDRESS),
        DynSolType::Array(inner) => {
            field.set_data_type(EthereumDataType::ARRAY);
            field.entry_type = Some(array_entry_type(inner)?).into();
        }
        DynSolType::FixedArray(inner, size) => {
            field.set_data_type(EthereumDataType::ARRAY);
            field.set_size(*size as u32);
            field.entry_type = Some(array_entry_type(inner)?).into();
        }
        DynSolType::CustomStruct { name, tuple, .. } => {
            field.set_data_type(EthereumDataType::STRUCT);
            field.set_size(tuple.len() as u32);
            field.set_struct_name(name.clone());
        }
        DynSolType::Tuple(_) | DynSolType::Function => {
            return Err(TrezorError::UnsupportedTypedData(format!(
                "type `{ty}` is not supported by Trezor"
            )))
        }
    }
    Ok(field)
}

/// The firmware does not support arrays of arrays.
fn array_entry_type(inner: &DynSolType) -> Result<EthereumFieldType, TrezorError> {
    if matches!(inner, DynSolType::Array(_) | DynSolType::FixedArray(..)) {
        return Err(TrezorError::UnsupportedTypedData(
            "nested arrays are not supported by Trezor".to_string(),
        ));
    }
    field_type(inner)
}

/// Encodes an atomic value, or the length of an array, as an `EthereumTypedDataValueAck` payload.
fn encode_value(value: &DynSolValue) -> Result<Vec<u8>, TrezorError> {
    let encoded = match value {
        DynSolValue::Uint(x, bits) => x.to_be_bytes::<32>()[32 - bits / 8..].to_vec(),
        DynSolValue::Int(x, bits) => x.to_be_bytes::<32>()[32 - bits / 8..].to_vec(),
        DynSolValue::FixedBytes(word, size) => word.as_slice()[..*size].to_vec(),
        DynSolValue::Bytes(bytes) => bytes.clone(),
        DynSolValue::String(s) => s.as_bytes().to_vec(),
        DynSolValue::Bool(b) => vec![*b as u8],
        DynSolValue::Address(address) => address.to_vec(),
        DynSolValue::Array(items) | DynSolValue::FixedArray(items) => {
            let len = u16::try_from(items.len()).map_err(|_| {
                TrezorError::UnsupportedTypedData(format!(
                    "array length {} exceeds the device limit of {}",
                    items.len(),
                    u16::MAX
                ))
            })?;
            len.to_be_bytes().to_vec()
        }
        DynSolValue::CustomStruct { .. } | DynSolValue::Tuple(_) | DynSolValue::Function(_) => {
            return Err(TrezorError::UnsupportedTypedData(
                "device requested a value that is not atomic".to_string(),
            ))
        }
    };
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256, hex, U256};
    use alloy_sol_types::{eip712_domain, sol};

    /// The example typed data from the EIP-712 specification.
    const MAIL: &str = r#"{
        "types": {
            "EIP712Domain": [
                {"name": "name", "type": "string"},
                {"name": "version", "type": "string"},
                {"name": "chainId", "type": "uint256"},
                {"name": "verifyingContract", "type": "address"}
            ],
            "Person": [
                {"name": "name", "type": "string"},
                {"name": "wallet", "type": "address"}
            ],
            "Mail": [
                {"name": "from", "type": "Person"},
                {"name": "to", "type": "Person"},
                {"name": "contents", "type": "string"}
            ]
        },
        "primaryType": "Mail",
        "domain": {
            "name": "Ether Mail",
            "version": "1",
            "chainId": 1,
            "verifyingContract": "0xCcCCccccCCCCcCCCCCCcCcCccCcCCCcCcccccccC"
        },
        "message": {
            "from": {"name": "Cow", "wallet": "0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826"},
            "to": {"name": "Bob", "wallet": "0xbBbBBBBbbBBBbbbBbbBbbbbBBbBbbbbBbBbbBBbB"},
            "contents": "Hello, Bob!"
        }
    }"#;

    fn describe(
        member: &EthereumStructMember,
    ) -> (&str, EthereumDataType, Option<u32>, Option<&str>) {
        let ty = member.type_.as_ref().unwrap();
        (member.name(), ty.data_type(), ty.size, ty.struct_name.as_deref())
    }

    #[test]
    fn answers_struct_and_value_requests() {
        let typed_data: TypedData = serde_json::from_str(MAIL).unwrap();
        let data = TrezorTypedData::from_typed_data(&typed_data).unwrap();
        assert_eq!(data.primary_type(), "Mail");

        let domain = data.struct_members("EIP712Domain").unwrap();
        assert_eq!(
            domain.iter().map(describe).collect::<Vec<_>>(),
            [
                ("name", EthereumDataType::STRING, None, None),
                ("version", EthereumDataType::STRING, None, None),
                ("chainId", EthereumDataType::UINT, Some(32), None),
                ("verifyingContract", EthereumDataType::ADDRESS, None, None),
            ]
        );
        let mail = data.struct_members("Mail").unwrap();
        assert_eq!(
            mail.iter().map(describe).collect::<Vec<_>>(),
            [
                ("from", EthereumDataType::STRUCT, Some(2), Some("Person")),
                ("to", EthereumDataType::STRUCT, Some(2), Some("Person")),
                ("contents", EthereumDataType::STRING, None, None),
            ]
        );

        assert_eq!(data.encode_value(&[0, 0]).unwrap(), b"Ether Mail");
        assert_eq!(data.encode_value(&[0, 2]).unwrap(), U256::from(1).to_be_bytes::<32>());
        assert_eq!(
            data.encode_value(&[0, 3]).unwrap(),
            address!("CcCCccccCCCCcCCCCCCcCcCccCcCCCcCcccccccC").to_vec()
        );
        assert_eq!(data.encode_value(&[1, 0, 0]).unwrap(), b"Cow");
        assert_eq!(
            data.encode_value(&[1, 1, 1]).unwrap(),
            address!("bBbBBBBbbBBBbbbBbbBbbbbBBbBbbbbBbBbbBBbB").to_vec()
        );
        assert_eq!(data.encode_value(&[1, 2]).unwrap(), b"Hello, Bob!");

        // Structs are never requested as a whole, and unknown roots or members are rejected.
        assert!(data.encode_value(&[1]).is_err());
        assert!(data.encode_value(&[2, 0]).is_err());
        assert!(data.encode_value(&[1, 3]).is_err());
    }

    #[test]
    fn sol_struct_matches_typed_data() {
        sol! {
            #[derive(serde::Serialize)]
            struct Item {
                uint8 id;
                bytes32[2] hashes;
            }

            #[derive(serde::Serialize)]
            struct Order {
                int16 delta;
                bool active;
                bytes payload;
                Item[] items;
                address owner;
            }
        }

        let salt = b256!("0101010101010101010101010101010101010101010101010101010101010101");
        let domain = eip712_domain! {
            name: "Trezor",
            version: "2",
            chain_id: 5,
            verifying_contract: address!("0000000000000000000000000000000000000001"),
            salt: salt,
        };
        let order = Order {
            delta: -2,
            active: true,
            payload: hex!("c0ffee").into(),
            items: vec![Item {
                id: 7,
                hashes: [
                    b256!("0202020202020202020202020202020202020202020202020202020202020202"),
                    b256!("0303030303030303030303030303030303030303030303030303030303030303"),
                ],
            }],
            owner: address!("0000000000000000000000000000000000000002"),
        };

        let from_struct = TrezorTypedData::from_struct(&order, &domain).unwrap();
        let typed_data = TypedData::from_struct(&order, Some(domain));
        let from_typed_data = TrezorTypedData::from_typed_data(&typed_data).unwrap();

        for name in ["EIP712Domain", "Order", "Item"] {
            assert_eq!(
                from_struct.struct_members(name).unwrap(),
                from_typed_data.struct_members(name).unwrap(),
                "{name}"
            );
        }
        let order_members = from_struct.struct_members("Order").unwrap();
        let items = order_members[3].type_.as_ref().unwrap();
        assert_eq!((items.data_type(), items.size), (EthereumDataType::ARRAY, None));
        let entry = items.entry_type.as_ref().unwrap();
        assert_eq!(
            (entry.data_type(), entry.size, entry.struct_name.as_deref()),
            (EthereumDataType::STRUCT, Some(2), Some("Item"))
        );
        let item_members = from_struct.struct_members("Item").unwrap();
        let hashes = item_members[1].type_.as_ref().unwrap();
        assert_eq!((hashes.data_type(), hashes.size), (EthereumDataType::ARRAY, Some(2)));
        let entry = hashes.entry_type.as_ref().unwrap();
        assert_eq!((entry.data_type(), entry.size), (EthereumDataType::BYTES, Some(32)));

        let paths: [&[u32]; 11] = [
            &[0, 0],
            &[0, 2],
            &[0, 4],
            &[1, 0],
            &[1, 1],
            &[1, 2],
            &[1, 3],
            &[1, 3, 0, 0],
            &[1, 3, 0, 1],
            &[1, 3, 0, 1, 1],
            &[1, 4],
        ];
        for path in paths {
            assert_eq!(
                from_struct.encode_value(path).unwrap(),
                from_typed_data.encode_value(path).unwrap(),
                "{path:?}"
            );
        }
        assert_eq!(from_struct.encode_value(&[0, 4]).unwrap(), salt.to_vec());
        assert_eq!(from_struct.encode_value(&[1, 0]).unwrap(), (-2i16).to_be_bytes());
        assert_eq!(from_struct.encode_value(&[1, 1]).unwrap(), [1]);
        assert_eq!(from_struct.encode_value(&[1, 2]).unwrap(), hex!("c0ffee"));
        assert_eq!(from_struct.encode_value(&[1, 3]).unwrap(), 1u16.to_be_bytes());
        assert_eq!(from_struct.encode_value(&[1, 3, 0, 0]).unwrap(), [7]);
        assert_eq!(from_struct.encode_value(&[1, 3, 0, 1]).unwrap(), 2u16.to_be_bytes());
        // Array elements that are structs are expanded by the device, never requested as a whole.
        assert!(from_struct.encode_value(&[1, 3, 0]).is_err());
    }

    #[test]
    fn rejects_conflicting_domain_type_in_message() {
        let wrapper = |domain: &str| -> TypedData {
            serde_json::from_str(&format!(
                r#"{{
                    "types": {{
                        "EIP712Domain": [{{"name": "name", "type": "string"}}],
                        "Wrapper": [{{"name": "inner", "type": "EIP712Domain"}}]
                    }},
                    "primaryType": "Wrapper",
                    "domain": {domain},
                    "message": {{"inner": {{"name": "spoofed"}}}}
                }}"#
            ))
            .unwrap()
        };

        let data = TrezorTypedData::from_typed_data(&wrapper(r#"{"name": "Ether Mail"}"#)).unwrap();
        assert_eq!(data.encode_value(&[1, 0, 0]).unwrap(), b"spoofed");

        assert!(matches!(
            TrezorTypedData::from_typed_data(&wrapper(r#"{"name": "Ether Mail", "chainId": 1}"#)),
            Err(TrezorError::UnsupportedTypedData(_))
        ));
    }

    #[test]
    fn rejects_nested_arrays() {
        let typed_data: TypedData = serde_json::from_str(
            r#"{
                "types": {"Matrix": [{"name": "rows", "type": "uint256[][]"}]},
                "primaryType": "Matrix",
                "domain": {},
                "message": {"rows": [[1]]}
            }"#,
        )
        .unwrap();
        let data = TrezorTypedData::from_typed_data(&typed_data).unwrap();
        assert!(matches!(data.struct_members("Matrix"), Err(TrezorError::UnsupportedTypedData(_))));
    }
}
