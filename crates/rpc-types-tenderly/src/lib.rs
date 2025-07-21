#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use std::str::FromStr;

use alloy_consensus::TxType;
use alloy_dyn_abi::{DynSolType, DynSolValue};
use alloy_eips::BlockNumberOrTag;
use alloy_primitives::{Address, Bloom, Bytes, FixedBytes, Log, I256, U256};
use serde::{de::Error, Deserialize, Serialize};

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
    pub asset_changes: Option<Vec<AssetChange>>,
    /// Balance changes caused by the transaction.
    pub balance_changes: Option<Vec<BalanceChange>>,
    /// State changes caused by the transaction.
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
    pub inputs: Option<Vec<TenderlyLogInput>>,
    /// Unencoded logs.
    pub raw: Log,
}

/// Log inputs decoded by the tenderly node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenderlyLogInput {
    /// Value of the input.
    #[serde(rename = "value")]
    raw_value: serde_json::Value,
    /// Type of the input.
    #[serde(rename = "type")]
    raw_typ: serde_json::Value,
    /// Name of the input.
    pub name: String,
    /// True if the input is indexed.
    pub indexed: bool,
}

impl TenderlyLogInput {
    /// Returns the parsed type of the log input.
    pub fn typ(&self) -> Option<DynSolType> {
        let raw = self.raw_typ.as_str()?;
        let Ok(typ) = raw.parse() else {
            return None;
        };
        Some(typ)
    }

    /// Returns the parsed value of the log input.
    pub fn value(&self) -> Option<DynSolValue> {
        let Ok(val) = Self::parse_dyn_value(&self.raw_value, &self.typ()?) else {
            return None;
        };
        Some(val)
    }

    fn parse_dyn_value(
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
    /// Value of the call. Omitted if zero.
    pub value: Option<U256>,
    /// Input of the call.
    pub input: Bytes,
    /// Name of the method. Omitted if unknown.
    pub method: Option<String>,
    /// Output of the call.
    pub output: Bytes,
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
    pub from: Option<Address>,
    /// Recipient address.
    pub to: Option<Address>,
    /// Unformatted amount of the asset.
    pub raw_amount: U256,
    /// Amount formatted according to asset decimals.
    pub amount: Option<String>,
    /// Dollar value of the change.
    pub dollar_value: Option<String>,
}

/// Information describing an onchain asset.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AssetInfo {
    /// Token standard of the asset.
    pub standard: AssetStandard,
    /// Fungibility of the asset, omitted if unknown.
    pub r#type: Option<AssetFungibility>,
    /// Address of the token contract.
    pub contract_address: Option<Address>,
    /// Symbol of the asset.
    pub symbol: Option<String>,
    /// Name of the asset.
    pub name: Option<String>,
    /// URL of the asset logo.
    // TODO: use url crate here?
    pub logo: Option<String>,
    /// Decimals of the asset.
    pub decimals: Option<u8>,
    /// Dollar value of the asset.
    // TODO: this does not fit in a f64 so I left it as string for now
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

/// Token standard of an asset.
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
    pub transfers: Option<Vec<usize>>,
}

/// State changes of an address caused by a transaction
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StateChange {
    /// Address affected by the transaction..
    pub address: Address,
    /// Storage change caused by the transaction.
    pub storage: Option<Vec<StorageSlotChange>>,
    /// Nonce change caused by the transaction.
    pub nonce: Option<ValueChange>,
    /// Balance change caused by the transaction.
    pub balance: Option<ValueChange>,
}

/// Describes the change of a storage slot due to a trasnaction.
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

/// Describes the change of a value due to a trasnaction.
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
    use crate::TenderlySimulationResult;

    #[test]
    fn test_success_response() {
        let input = include_str!("../test_data/success.json");
        let _parsed: TenderlySimulationResult = serde_json::from_str(input).unwrap();
    }

    #[test]
    fn test_failure_response() {
        let input = include_str!("../test_data/failure.json");
        let _parsed: TenderlySimulationResult = serde_json::from_str(input).unwrap();
    }

    #[test]
    fn test_bundle_success_response() {
        let input = include_str!("../test_data/bundle_success.json");
        let _parsed: Vec<TenderlySimulationResult> = serde_json::from_str(input).unwrap();
    }

    #[test]
    fn test_trace_success_response() {
        let input = include_str!("../test_data/trace_success.json");
        let _parsed: TenderlySimulationResult = serde_json::from_str(input).unwrap();
    }

    #[test]
    fn test_trace_complex_response() {
        let input = include_str!("../test_data/trace_complex.json");
        let _parsed: TenderlySimulationResult = serde_json::from_str(input).unwrap();
    }

    #[test]
    fn test_trace_swap_response() {
        let input = include_str!("../test_data/trace_swap.json");
        let _parsed: TenderlySimulationResult = serde_json::from_str(input).unwrap();
    }
}
