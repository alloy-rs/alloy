#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

use std::str::FromStr;

use alloy_consensus::TxType;
use alloy_dyn_abi::{DynSolType, DynSolValue};
use alloy_eips::BlockNumberOrTag;
use alloy_primitives::{Address, Bloom, Bytes, FixedBytes, Log, I256, U256};
use serde::{de::Error, Deserialize, Serialize};

/// Tenderly RPC estimate gas result.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenderlyEstimateGasResult {
    /// The estimated gas limit for the transaction.
    #[serde(with = "alloy_serde::quantity")]
    pub gas: u64,
    /// The actual gas used by the transaction.
    #[serde(with = "alloy_serde::quantity")]
    pub gas_used: u64,
}

/// Gas price tier information for Tenderly RPC.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenderlyGasPriceTier {
    /// The maximum priority fee per gas.
    #[serde(with = "alloy_serde::quantity")]
    pub max_priority_fee_per_gas: u128,
    /// The maximum fee per gas.
    #[serde(with = "alloy_serde::quantity")]
    pub max_fee_per_gas: u128,
    /// The estimated wait time in milliseconds.
    pub wait_time: u64,
}

/// Tenderly RPC gas price result.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenderlyGasPriceResult {
    /// The current block number.
    #[serde(with = "alloy_serde::quantity")]
    pub current_block_number: u64,
    /// The base fee per gas.
    #[serde(with = "alloy_serde::quantity")]
    pub base_fee_per_gas: u128,
    /// Gas price tiers for different urgency levels.
    pub price: TenderlyGasPriceTiers,
}

/// Gas price tiers for different urgency levels.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenderlyGasPriceTiers {
    /// Low urgency tier.
    pub low: TenderlyGasPriceTier,
    /// Medium urgency tier.
    pub medium: TenderlyGasPriceTier,
    /// High urgency tier.
    pub high: TenderlyGasPriceTier,
}

/// Decoded argument for Tenderly decode input.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenderlyDecodedArgument {
    /// Value of the argument.
    #[serde(rename = "value")]
    raw_value: serde_json::Value,
    /// Type of the argument.
    #[serde(rename = "type")]
    raw_typ: serde_json::Value,
    /// Name of the argument.
    pub name: String,
    /// True if the argument is indexed (for events).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed: Option<bool>,
}

impl TenderlyDecodedArgument {
    /// Returns the parsed type of the decoded argument.
    pub fn ty(&self) -> Option<DynSolType> {
        let raw = self.raw_typ.as_str()?;
        let Ok(ty) = raw.parse() else {
            return None;
        };
        Some(ty)
    }

    /// Returns the parsed value of the decoded argument.
    pub fn value(&self) -> Option<DynSolValue> {
        let Ok(val) = DecodedValue::parse_dyn_value(&self.raw_value, &self.ty()?) else {
            return None;
        };
        Some(val)
    }
}

/// Tenderly RPC decode input result.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenderlyDecodeInputResult {
    /// Name of the decoded function or event.
    pub name: String,
    /// Confidence level of the decoding (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Decoded arguments.
    pub decoded_arguments: Vec<TenderlyDecodedArgument>,
}

/// Function input type for Tenderly function signatures.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenderlyFunctionInput {
    /// Type of the input parameter.
    #[serde(rename = "type")]
    raw_typ: serde_json::Value,
}

impl TenderlyFunctionInput {
    /// Returns the parsed type of the input parameter.
    pub fn ty(&self) -> Option<DynSolType> {
        let raw = self.raw_typ.as_str()?;
        raw.parse().ok()
    }
}

/// Function signature for Tenderly function signatures.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenderlyFunctionSignature {
    /// Name of the function.
    pub name: String,
    /// Input parameters of the function.
    pub inputs: Vec<TenderlyFunctionInput>,
}

/// Parameters for Tenderly get transaction range request.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenderlyTransactionRangeParams {
    /// The address to check transactions from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<Address>,
    /// The address to check transactions to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Address>,
    /// The starting block number to search from.
    pub from_block: BlockNumberOrTag,
    /// The ending block number to search to.
    pub to_block: BlockNumberOrTag,
}

/// Parameters for Tenderly get storage changes request.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenderlyStorageQueryParams {
    /// The address of the contract to fetch storage changes for.
    pub address: Address,
    /// The storage slot offset to start querying from (hex string).
    pub offset: U256,
}

/// Storage change entry for Tenderly get storage changes response.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenderlyStorageChange {
    /// Block number where the change occurred.
    #[serde(with = "alloy_serde::quantity")]
    pub block_number: u64,
    /// New value of the storage slot.
    pub value: FixedBytes<32>,
}

/// Tenderly RPC simulation result.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenderlySimulationResult {
    /// The final status of the transaction, typically indicating success or failure.
    pub status: bool,
    /// The amount of gas used by the transaction.
    #[serde(with = "alloy_serde::quantity")]
    pub gas_used: u64,
    /// The total amount of gas used when this transaction was executed in the block.
    #[serde(with = "alloy_serde::quantity")]
    pub cumulative_gas_used: u64,
    /// The block the transaction was simulated in.
    pub block_number: BlockNumberOrTag,
    /// The type of the transaction.
    #[serde(rename = "type")]
    pub typ: TxType,
    /// The blocks bloom filter.
    pub logs_bloom: Bloom,
    /// Logs generated during the execution of the transaction.
    pub logs: Vec<TenderlyLog>,
    /// Tenderly trace of the transaction execution.
    pub trace: Vec<TenderlyTrace>,
    /// Asset changes caused by the transaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_changes: Option<Vec<AssetChange>>,
    /// Balance changes caused by the transaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance_changes: Option<Vec<BalanceChange>>,
    /// State changes caused by the transaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_changes: Option<Vec<StateChange>>,
}

/// Logs returned by Tenderly RPC, might be decoded.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TenderlyLog {
    /// Decoded name of the emitted log.
    pub name: String,
    /// True if log was emitted by an anonymous event.
    pub anonymous: bool,
    /// Decoded inputs of the event.
    /// This field is not skipped when inputs are `None`.
    pub inputs: Option<Vec<DecodedValue>>,
    /// Unencoded logs.
    pub raw: Log,
}

/// Log inputs decoded by the tenderly node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecodedValue {
    /// Value of the input.
    #[serde(rename = "value")]
    raw_value: serde_json::Value,
    /// Type of the input.
    #[serde(rename = "type")]
    raw_typ: serde_json::Value,
    /// Name of the input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// True if the input is indexed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed: Option<bool>,
}

impl DecodedValue {
    /// Returns the parsed type of the log input.
    pub fn ty(&self) -> Option<DynSolType> {
        let raw = self.raw_typ.as_str()?;
        let Ok(ty) = raw.parse() else {
            return None;
        };
        Some(ty)
    }

    /// Returns the parsed value of the log input.
    pub fn value(&self) -> Option<DynSolValue> {
        let Ok(val) = Self::parse_dyn_value(&self.raw_value, &self.ty()?) else {
            return None;
        };
        Some(val)
    }

    /// Parses a JSON value into a `DynSolValue` based on the given `DynSolType`.
    pub(crate) fn parse_dyn_value(
        val: &serde_json::Value,
        ty: &DynSolType,
    ) -> Result<DynSolValue, serde_json::error::Error> {
        use serde_json::Error;

        match ty {
            DynSolType::Bool => {
                val.as_bool().map(DynSolValue::Bool).ok_or_else(|| Error::custom("expected bool"))
            }
            DynSolType::Uint(bits) => val
                .as_str()
                .ok_or_else(|| Error::custom("expected string"))
                .and_then(|a| U256::from_str(a).map_err(Error::custom))
                .map(|u| DynSolValue::Uint(u, *bits)),
            DynSolType::Int(bits) => val
                .as_str()
                .ok_or_else(|| Error::custom("expected string"))
                .and_then(|a| I256::from_str(a).map_err(Error::custom))
                .map(|i| DynSolValue::Int(i, *bits)),
            DynSolType::Address => val
                .as_str()
                .ok_or_else(|| Error::custom("expected string"))
                .and_then(|a| Address::from_str(a).map_err(Error::custom))
                .map(DynSolValue::Address),
            DynSolType::FixedBytes(size) => val
                .as_str()
                .ok_or_else(|| Error::custom("expected string"))
                .and_then(|a| FixedBytes::from_str(a).map_err(Error::custom))
                .map(|b| DynSolValue::FixedBytes(b, *size)),
            DynSolType::Bytes => val
                .as_str()
                .ok_or_else(|| Error::custom("expected string"))
                .and_then(|b| Bytes::from_str(b).map_err(Error::custom))
                .map(|b| DynSolValue::Bytes(b.into())),
            DynSolType::String => val
                .as_str()
                .ok_or_else(|| Error::custom("expected string"))
                .map(|s| DynSolValue::String(s.to_owned())),
            DynSolType::Array(inner) => {
                let arr = val.as_array().ok_or_else(|| Error::custom("expected array"))?;
                let values: Vec<DynSolValue> = arr
                    .iter()
                    .map(|v| Self::parse_dyn_value(v, inner))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(DynSolValue::Array(values))
            }
            DynSolType::FixedArray(inner, size) => {
                let arr = val.as_array().ok_or_else(|| Error::custom("expected array"))?;
                if arr.len() != *size {
                    return Err(Error::custom("array size mismatch"));
                }
                let values: Vec<DynSolValue> = arr
                    .iter()
                    .map(|v| Self::parse_dyn_value(v, inner))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(DynSolValue::FixedArray(values))
            }
            DynSolType::Tuple(types) => {
                let arr = val.as_array().ok_or_else(|| Error::custom("expected tuple"))?;
                if arr.len() != types.len() {
                    return Err(Error::custom("tuple length mismatch"));
                }
                let values = arr
                    .iter()
                    .zip(types.iter())
                    .map(|(v, t)| Self::parse_dyn_value(v, t))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(DynSolValue::Tuple(values))
            }
            _ => Err(Error::custom("type is not supported")),
        }
    }
}

/// Call trace generated by tenderly.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TenderlyTrace {
    /// Call type.
    pub r#type: TenderlyCallType,
    /// Origin address of the call.
    pub from: Address,
    /// Target address of the call.
    pub to: Address,
    /// Gas used by the call.
    #[serde(with = "alloy_serde::quantity")]
    pub gas: u64,
    /// Gas used by the call.
    #[serde(with = "alloy_serde::quantity")]
    pub gas_used: u64,
    /// Value of the call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<U256>,
    /// Error caused by the call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Input of the call.
    pub input: Bytes,
    /// Decoded Trace Input
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoded_input: Option<Vec<DecodedValue>>,
    /// Name of the method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Output of the call.
    pub output: Bytes,
    /// Decoded output of the call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoded_output: Option<Vec<DecodedValue>>,
    /// How many subtraces this trace has.
    pub subtraces: usize,
    /// The identifier of this transaction trace in the set.
    ///
    /// This gives the exact location in the call trace
    /// [index in root CALL, index in first CALL, index in second CALL, …].
    pub trace_address: Vec<usize>,
}

/// Types of EVM calls.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum TenderlyCallType {
    /// Call type.
    Call,
    /// Deprecated CallCode type.
    CallCode,
    /// StaticCall type.
    StaticCall,
    /// DelegateCall type.
    DelegateCall,
    /// AuthorizedCall type.
    AuthCall,
}

/// Information about the assets affected by the transaction.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AssetChange {
    /// Information about the exchanged asset.
    pub asset_info: AssetInfo,
    /// Type of the asset change.
    pub r#type: ChangeType,
    /// Sender address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<Address>,
    /// Recipient address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Address>,
    /// Unformatted amount of the asset.
    pub raw_amount: U256,
    /// Amount formatted according to asset decimals.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    /// Dollar value of the change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dollar_value: Option<String>,
}

/// Information describing an onchain asset.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AssetInfo {
    /// Token standard of the asset.
    pub standard: AssetStandard,
    /// Fungibility of the asset, omitted if unknown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<AssetFungibility>,
    /// Address of the token contract.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_address: Option<Address>,
    /// Symbol of the asset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Name of the asset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// URL of the asset logo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    /// Decimals of the asset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
    /// Dollar value of the asset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dollar_value: Option<String>,
}

/// Token standard of an asset.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum AssetStandard {
    /// Native currency of the network.
    #[serde(rename = "NativeCurrency")]
    NativeCurrency,
    /// Fungible token.
    Erc20,
    /// Non-fungible token.
    Erc721,
    /// Multi-token.
    Erc1155,
}

/// Token standard of an asset.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub enum AssetFungibility {
    /// Native asset.
    Native,
    /// Fungible asset.
    Fungible,
    /// Non fungible asset.
    NonFungible,
}

/// Type of asset change.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub enum ChangeType {
    /// Asset mint.
    Mint,
    /// Asset burn.
    Burn,
    /// Asset transfer.
    Transfer,
}

/// Balance change of an address caused by a transaction.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BalanceChange {
    /// Address affected by the transaction.
    pub address: Address,
    /// Dollar value of the
    pub dollar_value: String,
    /// Identifiers of the traces affecting this balance change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfers: Option<Vec<usize>>,
}

/// State changes of an address caused by a transaction
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StateChange {
    /// Address affected by the transaction..
    pub address: Address,
    /// Nonce change caused by the transaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<ValueChange>,
    /// Balance change caused by the transaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<ValueChange>,
    /// Storage change caused by the transaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<Vec<StorageSlotChange>>,
}

/// Describes the change of a storage slot due to a transaction.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StorageSlotChange {
    /// Storage slot.
    pub slot: FixedBytes<32>,
    /// Value before the transaction.
    pub previous_value: FixedBytes<32>,
    /// Value after the transaction.
    pub new_value: FixedBytes<32>,
}

/// Describes the change of a value due to a transaction.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ValueChange {
    /// Value before the transaction.
    pub previous_value: U256,
    /// Value after the transaction.
    pub new_value: U256,
}

#[cfg(test)]
mod tests {
    use alloy_dyn_abi::{DynSolType, DynSolValue};

    use crate::{
        TenderlyDecodeInputResult, TenderlyEstimateGasResult, TenderlyGasPriceResult,
        TenderlySimulationResult,
    };

    #[test]
    fn test_success_response() {
        let input = include_str!("../test_data/success.json");
        let parsed: TenderlySimulationResult = serde_json::from_str(input).unwrap();

        // strip whitespace to force equal formatting
        assert_eq!(
            serde_json::to_string(&parsed).unwrap().split_whitespace().collect::<String>(),
            input.split_whitespace().collect::<String>()
        );
    }

    #[test]
    fn test_failure_response() {
        let input = include_str!("../test_data/failure.json");
        let parsed: TenderlySimulationResult = serde_json::from_str(input).unwrap();

        assert_eq!(
            serde_json::to_string(&parsed).unwrap().split_whitespace().collect::<String>(),
            input.split_whitespace().collect::<String>()
        );
    }

    #[test]
    fn test_bundle_success_response() {
        let input = include_str!("../test_data/bundle_success.json");
        let parsed: Vec<TenderlySimulationResult> = serde_json::from_str(input).unwrap();

        assert_eq!(
            serde_json::to_string(&parsed).unwrap().split_whitespace().collect::<String>(),
            input.split_whitespace().collect::<String>()
        );
    }

    #[test]
    fn test_trace_success_response() {
        let input = include_str!("../test_data/trace_success.json");
        let parsed: TenderlySimulationResult = serde_json::from_str(input).unwrap();

        assert_eq!(
            serde_json::to_string(&parsed).unwrap().split_whitespace().collect::<String>(),
            input.split_whitespace().collect::<String>()
        );
    }

    #[test]
    fn test_trace_complex_response() {
        let input = include_str!("../test_data/trace_complex.json");
        let parsed: TenderlySimulationResult = serde_json::from_str(input).unwrap();

        assert_eq!(
            serde_json::to_string(&parsed).unwrap().split_whitespace().collect::<String>(),
            input.split_whitespace().collect::<String>()
        );
    }

    #[test]
    fn test_trace_swap_response() {
        let input = include_str!("../test_data/trace_swap.json");
        let parsed: TenderlySimulationResult = serde_json::from_str(input).unwrap();

        assert_eq!(
            serde_json::to_string(&parsed).unwrap().split_whitespace().collect::<String>(),
            input.split_whitespace().collect::<String>()
        );
    }

    #[test]
    fn test_estimate_gas_response() {
        let input = include_str!("../test_data/estimate_gas.json");
        let parsed: TenderlyEstimateGasResult = serde_json::from_str(input).unwrap();

        assert_eq!(parsed.gas, 0x12579);
        assert_eq!(parsed.gas_used, 0xff06);

        assert_eq!(
            serde_json::to_string(&parsed).unwrap().split_whitespace().collect::<String>(),
            input.split_whitespace().collect::<String>()
        );
    }

    #[test]
    fn test_gas_price_response() {
        let input = include_str!("../test_data/gas_price.json");
        let parsed: TenderlyGasPriceResult = serde_json::from_str(input).unwrap();

        assert_eq!(parsed.current_block_number, 0x14a0bdb);
        assert_eq!(parsed.base_fee_per_gas, 0xbcdd3e0f);
        assert_eq!(parsed.price.low.max_priority_fee_per_gas, 0x27f840a);
        assert_eq!(parsed.price.low.max_fee_per_gas, 0x1097c5d8b);
        assert_eq!(parsed.price.low.wait_time, 36000);
        assert_eq!(parsed.price.medium.max_priority_fee_per_gas, 0x9b4c5bb);
        assert_eq!(parsed.price.medium.max_fee_per_gas, 0x1137c6003);
        assert_eq!(parsed.price.medium.wait_time, 24000);
        assert_eq!(parsed.price.high.max_priority_fee_per_gas, 0x10128f8e);
        assert_eq!(parsed.price.high.max_fee_per_gas, 0x11c5174a8);
        assert_eq!(parsed.price.high.wait_time, 12000);

        assert_eq!(
            serde_json::to_string(&parsed).unwrap().split_whitespace().collect::<String>(),
            input.split_whitespace().collect::<String>()
        );
    }

    #[test]
    fn test_estimate_gas_bundle_response() {
        let input = include_str!("../test_data/estimate_gas_bundle.json");
        let parsed: Vec<TenderlyEstimateGasResult> = serde_json::from_str(input).unwrap();

        assert_eq!(parsed.len(), 4);
        assert_eq!(parsed[0].gas, 0x12579);
        assert_eq!(parsed[0].gas_used, 0xff06);
        assert_eq!(parsed[1].gas, 0x10918);
        assert_eq!(parsed[1].gas_used, 0xb551);
        assert_eq!(parsed[2].gas, 0xa621);
        assert_eq!(parsed[2].gas_used, 0x6625);
        assert_eq!(parsed[3].gas, 0x10649);
        assert_eq!(parsed[3].gas_used, 0xb249);

        assert_eq!(
            serde_json::to_string(&parsed).unwrap().split_whitespace().collect::<String>(),
            input.split_whitespace().collect::<String>()
        );
    }

    #[test]
    fn test_decode_input_response() {
        let input = include_str!("../test_data/decode_input.json");
        let parsed: TenderlyDecodeInputResult = serde_json::from_str(input).unwrap();

        assert_eq!(parsed.name, "transfer");
        assert_eq!(parsed.decoded_arguments.len(), 2);

        // Test first argument (address)
        let arg0 = &parsed.decoded_arguments[0];
        assert_eq!(arg0.name, "arg0");
        let ty0 = arg0.ty().expect("should parse address type");
        assert!(matches!(ty0, DynSolType::Address));
        let value0 = arg0.value().expect("should parse address value");
        assert!(matches!(value0, DynSolValue::Address(_)));

        // Test second argument (uint256)
        let arg1 = &parsed.decoded_arguments[1];
        assert_eq!(arg1.name, "arg1");
        let ty1 = arg1.ty().expect("should parse uint256 type");
        assert!(matches!(ty1, DynSolType::Uint(_)));
        let value1 = arg1.value().expect("should parse uint256 value");
        assert!(matches!(value1, DynSolValue::Uint(_, _)));

        // Round-trip test: deserialize and serialize back to verify structure
        let serialized = serde_json::to_string(&parsed).unwrap();
        let reparsed: TenderlyDecodeInputResult = serde_json::from_str(&serialized).unwrap();
        assert_eq!(reparsed.name, parsed.name);
        assert_eq!(reparsed.decoded_arguments.len(), parsed.decoded_arguments.len());
    }

    #[test]
    fn test_decode_error_response() {
        let input = include_str!("../test_data/decode_error.json");
        let parsed: TenderlyDecodeInputResult = serde_json::from_str(input).unwrap();

        assert_eq!(parsed.name, "ERC20InsufficientBalance");
        assert_eq!(parsed.decoded_arguments.len(), 3);

        // Test first argument (address)
        let arg0 = &parsed.decoded_arguments[0];
        assert_eq!(arg0.name, "arg0");
        let ty0 = arg0.ty().expect("should parse address type");
        assert!(matches!(ty0, DynSolType::Address));
        let value0 = arg0.value().expect("should parse address value");
        assert!(matches!(value0, DynSolValue::Address(_)));

        // Test second argument (uint256)
        let arg1 = &parsed.decoded_arguments[1];
        assert_eq!(arg1.name, "arg1");
        let ty1 = arg1.ty().expect("should parse uint256 type");
        assert!(matches!(ty1, DynSolType::Uint(_)));
        let value1 = arg1.value().expect("should parse uint256 value");
        assert!(matches!(value1, DynSolValue::Uint(_, _)));

        // Test third argument (uint256)
        let arg2 = &parsed.decoded_arguments[2];
        assert_eq!(arg2.name, "arg2");
        let ty2 = arg2.ty().expect("should parse uint256 type");
        assert!(matches!(ty2, DynSolType::Uint(_)));
        let value2 = arg2.value().expect("should parse uint256 value");
        assert!(matches!(value2, DynSolValue::Uint(_, _)));

        // Round-trip test: deserialize and serialize back to verify structure
        let serialized = serde_json::to_string(&parsed).unwrap();
        let reparsed: TenderlyDecodeInputResult = serde_json::from_str(&serialized).unwrap();
        assert_eq!(reparsed.name, parsed.name);
        assert_eq!(reparsed.decoded_arguments.len(), parsed.decoded_arguments.len());
    }
}
