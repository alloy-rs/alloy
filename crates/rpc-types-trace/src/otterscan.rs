//! Otterscan specific types for RPC responses.
//!
//! <https://www.quicknode.com/docs/ethereum/ots_getBlockTransactions>
//! <https://github.com/otterscan/otterscan/blob/v2.6.1/docs/custom-jsonrpc.md>

use crate::parity::TransactionTrace;
use alloy_primitives::{Address, Bloom, Bytes, TxHash, B256, U256};
use alloy_rpc_types_eth::{
    Block, BlockTransactions, Header, Log, Transaction, TransactionReceipt, Withdrawals,
};
use serde::{
    de::{self, Unexpected},
    ser::SerializeSeq,
    Deserialize, Deserializer, Serialize, Serializer,
};

/// Operation type enum for `InternalOperation` struct
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationType {
    /// Operation Transfer
    OpTransfer = 0,
    /// Operation Contract self destruct
    OpSelfDestruct = 1,
    /// Operation Create
    OpCreate = 2,
    /// Operation Create2
    OpCreate2 = 3,
    /// Operation EofCreate
    OpEofCreate = 4,
}

// Implement Serialize for OperationType
impl Serialize for OperationType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

// Implement Deserialize for OperationType
impl<'de> Deserialize<'de> for OperationType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize string, then parse it to u8
        let value = u8::deserialize(deserializer)?;
        match value {
            0 => Ok(Self::OpTransfer),
            1 => Ok(Self::OpSelfDestruct),
            2 => Ok(Self::OpCreate),
            3 => Ok(Self::OpCreate2),
            4 => Ok(Self::OpEofCreate),
            other => Err(de::Error::invalid_value(
                Unexpected::Unsigned(other as u64),
                &"a valid OperationType",
            )),
        }
    }
}

/// Custom struct for otterscan `getInternalOperations` RPC response
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalOperation {
    /// The type of the internal operation.
    pub r#type: OperationType,
    /// The address from which the operation originated.
    pub from: Address,
    /// The address to which the operation is directed.
    pub to: Address,
    /// The value transferred in the operation.
    pub value: U256,
}

/// Custom struct for otterscan `traceTransaction` RPC response
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEntry {
    /// The type of the trace entry.
    pub r#type: String,
    /// The depth of the trace entry.
    pub depth: u32,
    /// The address from which the trace originated.
    pub from: Address,
    /// The address to which the trace is directed.
    pub to: Address,
    /// The value transferred in the trace.
    pub value: Option<U256>,
    /// The input data for the trace.
    pub input: Bytes,
    /// The output data for the trace.
    pub output: Bytes,
}

impl TraceEntry {
    /// Create a new [`TraceEntry`] from a [`TransactionTrace`] if it is a call action.
    ///
    /// Returns `None` if the trace action is not a call.
    pub fn from_transaction_trace(trace: &TransactionTrace) -> Option<Self> {
        let call = trace.action.as_call()?;
        let output = trace.result.as_ref().map(|out| out.output().clone()).unwrap_or_default();
        Some(Self {
            r#type: call.call_type.to_string(),
            depth: trace.trace_address.len() as u32,
            from: call.from,
            to: call.to,
            value: Some(call.value),
            input: call.input.clone(),
            output,
        })
    }
}

/// Internal issuance struct for `BlockDetails` struct
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[expect(missing_copy_implementations)]
#[serde(rename_all = "camelCase")]
pub struct InternalIssuance {
    /// The block reward issued.
    pub block_reward: U256,
    /// The uncle reward issued.
    pub uncle_reward: U256,
    /// The total issuance amount.
    pub issuance: U256,
}

/// Custom `Block` struct that includes transaction count for Otterscan responses.
///
/// This is the same as a regular `Block`, but the input field returns only the 4 bytes method
/// selector instead of the entire calldata.
///
/// See also <https://docs.otterscan.io/api-docs/ots-api#ots_getblocktransactions>
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsBlock<T = Transaction, H = Header> {
    /// The block information.
    #[serde(flatten)]
    pub block: Block<T, H>,
    /// The number of transactions in the block.
    #[doc(alias = "tx_count")]
    pub transaction_count: usize,
}

impl<T, H> From<Block<T, H>> for OtsBlock<T, H> {
    fn from(block: Block<T, H>) -> Self {
        Self { transaction_count: block.transactions.len(), block }
    }
}

// we need to implement Serialize manually because we want to truncate the input field automatically
// for convenience
impl<T, H> Serialize for OtsBlock<T, H>
where
    T: Serialize,
    H: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct OtsBlockHelper<'a, T: Serialize, H> {
            /// Header of the block.
            #[serde(flatten)]
            header: &'a H,
            /// Uncles' hashes.
            uncles: &'a Vec<B256>,
            #[serde(
                serialize_with = "serialize_txs_with_truncated_input",
                skip_serializing_if = "BlockTransactions::is_uncle"
            )]
            transactions: &'a BlockTransactions<T>,
            /// Withdrawals in the block.
            #[serde(skip_serializing_if = "Option::is_none")]
            withdrawals: &'a Option<Withdrawals>,
            transaction_count: usize,
        }

        let Self {
            block: Block { header, uncles, transactions, withdrawals, .. },
            transaction_count,
        } = self;

        let helper = OtsBlockHelper {
            header,
            uncles,
            transactions,
            withdrawals,
            transaction_count: *transaction_count,
        };

        helper.serialize(serializer)
    }
}

fn serialize_txs_with_truncated_input<T, S>(
    txs: &BlockTransactions<T>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    use serde_json::Value;
    match txs {
        BlockTransactions::Hashes(hashes) => hashes.serialize(serializer),
        BlockTransactions::Uncle => serializer.serialize_seq(Some(0))?.end(),
        BlockTransactions::Full(txs) => {
            let mut value = serde_json::to_value(txs).map_err(serde::ser::Error::custom)?;
            if let Value::Array(txs) = &mut value {
                for tx in txs {
                    if let Value::Object(map) = tx {
                        if let Some(Value::String(input)) = map.get_mut("input") {
                            // Truncate the input to the first 4 bytes (8 hex characters) plus 0x
                            // prefix
                            *input = input.chars().take(2 + 4 + 4).collect::<String>();
                        }
                    }
                }
            }
            value.serialize(serializer)
        }
    }
}

/// Custom `Block` struct that without transactions for Otterscan responses
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsSlimBlock<H = Header> {
    /// Header of the block.
    #[serde(flatten)]
    pub header: H,
    /// Uncles' hashes.
    #[serde(default)]
    pub uncles: Vec<B256>,
    /// Withdrawals in the block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub withdrawals: Option<Withdrawals>,
    /// The number of transactions in the block.
    #[doc(alias = "tx_count")]
    pub transaction_count: usize,
}

impl<T, H> From<Block<T, H>> for OtsSlimBlock<H> {
    fn from(block: Block<T, H>) -> Self {
        Self {
            header: block.header,
            uncles: block.uncles,
            withdrawals: block.withdrawals,
            transaction_count: block.transactions.len(),
        }
    }
}

/// Custom struct for otterscan `getBlockDetails` RPC response
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockDetails<H = Header> {
    /// The block information with transaction count.
    pub block: OtsSlimBlock<H>,
    /// The issuance information for the block.
    pub issuance: InternalIssuance,
    /// The total fees for the block.
    pub total_fees: U256,
}

impl<T, H> From<Block<T, H>> for BlockDetails<H> {
    fn from(block: Block<T, H>) -> Self {
        Self { block: block.into(), issuance: Default::default(), total_fees: U256::default() }
    }
}

impl<H> BlockDetails<H> {
    /// Create a new `BlockDetails` struct.
    pub fn new<T>(block: Block<T, H>, issuance: InternalIssuance, total_fees: U256) -> Self {
        Self { block: block.into(), issuance, total_fees }
    }
}

/// Custom transaction receipt struct for otterscan `OtsBlockTransactions` struct
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsTransactionReceipt {
    /// The transaction receipt.
    ///
    /// Note: the otterscan API sets all log fields to null.
    #[serde(flatten)]
    pub receipt: TransactionReceipt<OtsReceipt>,
    /// The timestamp of the transaction.
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub timestamp: Option<u64>,
}

/// The receipt of a transaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsReceipt {
    /// If the transaction is executed successfully.
    ///
    /// This is the `statusCode`
    #[serde(with = "alloy_serde::quantity")]
    pub status: bool,
    /// The cumulative gas used.
    #[serde(with = "alloy_serde::quantity")]
    pub cumulative_gas_used: u64,
    /// The logs sent from contracts.
    ///
    /// Note: this is set to null.
    pub logs: Option<Vec<Log>>,
    /// The bloom filter.
    ///
    /// Note: this is set to null.
    pub logs_bloom: Option<Bloom>,
    /// The transaction type.
    #[serde(with = "alloy_serde::quantity")]
    pub r#type: u8,
}

/// Custom struct for otterscan `getBlockTransactions` RPC response
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtsBlockTransactions<T = Transaction, H = Header> {
    /// The full block information with transaction count.
    pub fullblock: OtsBlock<T, H>,
    /// The list of transaction receipts.
    pub receipts: Vec<OtsTransactionReceipt>,
}

/// Custom struct for otterscan `searchTransactionsAfter` and `searchTransactionsBefore` RPC
/// responses
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[doc(alias = "TxWithReceipts")]
pub struct TransactionsWithReceipts<T = Transaction> {
    /// The list of transactions.
    #[doc(alias = "transactions")]
    pub txs: Vec<T>,
    /// The list of transaction receipts.
    pub receipts: Vec<OtsTransactionReceipt>,
    /// Indicates if this is the first page of results.
    pub first_page: bool,
    /// Indicates if this is the last page of results.
    pub last_page: bool,
}

/// Custom struct for otterscan `getContractCreator` RPC responses
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractCreator {
    /// The transaction used to create the contract.
    pub hash: TxHash,
    /// The address of the contract creator.
    pub creator: Address,
}

#[cfg(test)]
mod tests {
    use super::*;
    use similar_asserts::assert_eq;

    #[test]
    fn test_otterscan_receipt() {
        let s = r#"{
            "blockHash": "0xf05aa8b73b005314684595adcff8e6149917b3239b6316247ce5e88eba9fd3f5",
            "blockNumber": "0x1106fe7",
            "contractAddress": null,
            "cumulativeGasUsed": "0x95fac3",
            "effectiveGasPrice": "0x2e9f0055d",
            "from": "0x793abeea78d94c14b884a56788f549836a35db65",
            "gasUsed": "0x14427",
            "logs": null,
            "logsBloom": null,
            "status": "0x1",
            "to": "0x06450dee7fd2fb8e39061434babcfc05599a6fb8",
            "transactionHash": "0xd3cead022cbb5d6d18091f8b375e3a3896ec139e986144b9448290d55837275a",
            "transactionIndex": "0x90",
            "type": "0x2"
        }"#;

        let _receipt: OtsTransactionReceipt = serde_json::from_str(s).unwrap();
    }

    #[test]
    fn test_otterscan_internal_operation() {
        let s = r#"{
          "type": 0,
          "from": "0xea593b730d745fb5fe01b6d20e6603915252c6bf",
          "to": "0xcc3d455481967dc97346ef1771a112d7a14c8f12",
          "value": "0xee846f9305c00"
        }"#;
        let _op: InternalOperation = serde_json::from_str(s).unwrap();
    }

    #[test]
    fn test_serialize_operation_type() {
        assert_eq!(serde_json::to_string(&OperationType::OpTransfer).unwrap(), "0");
        assert_eq!(serde_json::to_string(&OperationType::OpSelfDestruct).unwrap(), "1");
        assert_eq!(serde_json::to_string(&OperationType::OpCreate).unwrap(), "2");
        assert_eq!(serde_json::to_string(&OperationType::OpCreate2).unwrap(), "3");
    }

    #[test]
    fn test_deserialize_operation_type() {
        assert_eq!(serde_json::from_str::<OperationType>("0").unwrap(), OperationType::OpTransfer);
        assert_eq!(
            serde_json::from_str::<OperationType>("1").unwrap(),
            OperationType::OpSelfDestruct
        );
        assert_eq!(serde_json::from_str::<OperationType>("2").unwrap(), OperationType::OpCreate);
        assert_eq!(serde_json::from_str::<OperationType>("3").unwrap(), OperationType::OpCreate2);
    }

    #[test]
    fn serde_ots_block_transactions() {
        let s = r#"{
  "fullblock": {
    "hash": "0x5e98e8e4d80928867e03eb2224f66fc8c68f687de3a5550119c365fca7abb118",
    "parentHash": "0x84eba4ac122adba9bbe79b78ccc538ec5fd7b612cd6c2cd6d4ac3a23160f6151",
    "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
    "miner": "0x25941dc771bb64514fc8abbce970307fb9d477e9",
    "stateRoot": "0x7347d30e42da2799eb5b51d8e1a81756323afd47d68e9c7f7fe5c6cfd38572bd",
    "transactionsRoot": "0x7cbc552113ed936ee351981d5151a8913cc7cc2ac55d930d6a43ded6e721c21b",
    "receiptsRoot": "0x056b23fbba480696b65fe5a59b8f2148a1299103c4f57df839233af2cf4ca2d2",
    "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    "difficulty": "0x0",
    "number": "0x64733",
    "gasLimit": "0x2255100",
    "gasUsed": "0x5208",
    "timestamp": "0x68285874",
    "extraData": "0x4e65746865726d696e64",
    "mixHash": "0x5aa29a261f252912f12377c312d68a616af8efef7a9f8c8911b7482bcf4a3adc",
    "nonce": "0x0000000000000000",
    "baseFeePerGas": "0x4227fedf",
    "withdrawalsRoot": "0x9a0aedb6a7b38b44467d87dd8c08b64589fcf729a0f60e9361ecb160f074b08c",
    "blobGasUsed": "0x0",
    "excessBlobGas": "0x0",
    "parentBeaconBlockRoot": "0x065c517950023785bf51c075203764504b5fa9b65b8fe3943aa9fb8a86e0391d",
    "requestsHash": "0xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "size": "0x507",
    "uncles": [],
    "transactions": [
      {
        "type": "0x0",
        "chainId": "0x88bb0",
        "nonce": "0x0",
        "gasPrice": "0x45f23888",
        "gas": "0x5208",
        "to": "0x6dbd21b035b11eb3d921872b1930670904c82803",
        "value": "0xe8d4a51000",
        "input": "0x00000000",
        "r": "0xd22873dda61094957a2a2563fbb5cafde0a16839294fbecf2eb2d498c975d655",
        "s": "0x5e718d90b3ee82bf2792f6704f98a30ae9c6e90475298a5f84751e840f0e0f9e",
        "v": "0x111783",
        "hash": "0x8c22653c4736f3cedf2d36fb4a5565397cfae051b6a5cc53a043078f03aa6ceb",
        "blockHash": "0x5e98e8e4d80928867e03eb2224f66fc8c68f687de3a5550119c365fca7abb118",
        "blockNumber": "0x64733",
        "transactionIndex": "0x0",
        "from": "0x6dbd21b035b11eb3d921872b1930670904c82803"
      }
    ],
    "withdrawals": [ ],
    "transactionCount": 1
  },
  "receipts": [
    {
      "status": "0x1",
      "cumulativeGasUsed": "0x5208",
      "logs": null,
      "logsBloom": null,
      "type": "0x0",
      "transactionHash": "0x8c22653c4736f3cedf2d36fb4a5565397cfae051b6a5cc53a043078f03aa6ceb",
      "transactionIndex": "0x0",
      "blockHash": "0x5e98e8e4d80928867e03eb2224f66fc8c68f687de3a5550119c365fca7abb118",
      "blockNumber": "0x64733",
      "gasUsed": "0x5208",
      "effectiveGasPrice": "0x45f23888",
      "from": "0x6dbd21b035b11eb3d921872b1930670904c82803",
      "to": "0x6dbd21b035b11eb3d921872b1930670904c82803",
      "contractAddress": null,
      "timestamp": "0x68285874"
    }
  ]
}
"#;

        let block: OtsBlockTransactions = serde_json::from_str(s).unwrap();
        let expected: serde_json::Value = serde_json::from_str(s).unwrap();
        let value = serde_json::to_value(&block).unwrap();
        assert_eq!(value, expected, "Serialized value does not match expected value");
    }
}
