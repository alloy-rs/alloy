//! Geth ERC-7562 tracer types.

use alloy_primitives::{map::HashMap, Address, Bytes, B256, U256};
use serde::{Deserialize, Serialize};

use crate::geth::CallLogFrame;

/// The response object for `debug_traceTransaction` with `"tracer": "erc7562Tracer"`.
///
/// <https://github.com/ethereum/go-ethereum/pull/31006>
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Erc7562Frame {
    /// The type of the call frame.
    #[serde(rename = "type")]
    pub call_frame_type: CallFrameType,
    /// The address of the caller.
    pub from: Address,
    /// The amount of gas provided to the call.
    #[serde(with = "alloy_serde::quantity")]
    pub gas: u64,
    /// The amount of gas used by the call.
    #[serde(with = "alloy_serde::quantity")]
    pub gas_used: u64,
    /// The address of the callee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Address>,
    /// The input of the call.
    pub input: Bytes,
    /// The output of the call, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Bytes>,
    /// The error message, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The revert reason, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revert_reason: Option<String>,
    /// The call log frames.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub logs: Vec<CallLogFrame>,
    /// The value of the call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<U256>,
    /// The accessed slots.
    pub accessed_slots: AccessedSlots,
    /// The ext code access info.
    pub ext_code_access_info: Vec<String>,
    /// The used opcodes and their counts.
    #[serde(with = "alloy_serde::quantity::hashmap")]
    pub used_opcodes: HashMap<u8, u64>,
    /// The contract sizes.
    pub contract_size: HashMap<Address, ContractSize>,
    /// Whether the call ran out of gas.
    pub out_of_gas: bool,
    /// Keccak preimages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keccak: Vec<Bytes>,
    /// The call frames.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calls: Vec<Self>,
}

impl Erc7562Frame {
    /// Returns true if this call reverted.
    pub fn is_revert(&self) -> bool {
        if self.revert_reason.is_some() {
            return true;
        }
        matches!(self.error.as_deref(), Some("execution reverted"))
    }
}

/// The accessed slots.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessedSlots {
    /// The reads.
    pub reads: HashMap<B256, Vec<B256>>,
    /// The writes.
    pub writes: HashMap<B256, u64>,
    /// The transient reads.
    pub transient_reads: HashMap<B256, u64>,
    /// The transient writes.
    pub transient_writes: HashMap<B256, u64>,
}

/// The contract sizes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractSize {
    /// The contract size.
    pub contract_size: u64,
    /// The opcode.
    pub opcode: u8,
}

/// The type of the call frame.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CallFrameType {
    /// The call frame type.
    #[default]
    Call,
    /// The delegate call frame type.
    DelegateCall,
    /// The call code frame type.
    CallCode,
    /// The static call frame type.
    StaticCall,
    /// The create frame type.
    Create,
    /// The create2 frame type.
    Create2,
}

/// The configuration for the ERC-7562 tracer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Erc7562Config {
    /// The size of the stack top items.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack_top_items_size: Option<u64>,
    /// Opcodes to ignore during OnOpcode hook execution.
    #[serde(default, skip_serializing_if = "Vec::is_empty", with = "alloy_serde::quantity::vec")]
    pub ignored_opcodes: Vec<u8>,
    /// If true, ERC-7562 tracer will collect event logs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_log: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geth::*;
    use similar_asserts::assert_eq;

    #[test]
    fn test_serialize_erc7562_trace() {
        let mut opts = GethDebugTracingCallOptions::default();
        opts.tracing_options.tracer =
            Some(GethDebugTracerType::BuiltInTracer(GethDebugBuiltInTracerType::Erc7562Tracer));

        assert_eq!(serde_json::to_string(&opts).unwrap(), r#"{"tracer":"erc7562Tracer"}"#);
    }

    #[test]
    fn test_deserialize_erc7562_trace() {
        let trace = r#"
        {
            "accessedSlots": {
                "reads": {
                    "0xa3b2ff63dddb6717733673f0c1cf67be4e4eecc50d4e5fd777cf82e814f7242f": [
                        "0x0000000000000000000000000000000000000000000000000000000000000000"
                    ]
                },
                "transientReads": {},
                "transientWrites": {},
                "writes": {
                    "0xa3b2ff63dddb6717733673f0c1cf67be4e4eecc50d4e5fd777cf82e814f7242f": 1
                }
            },
            "contractSize": {},
            "extCodeAccessInfo": [],
            "from": "0x7ab5742e5b448c142a35c92699fd2dd6b8930cbd",
            "gas": "0x23a3c",
            "gasUsed": "0xb21f",
            "input": "0xb760faf90000000000000000000000005604b855b3708057705f8dfc0e6470917082c43a",
            "keccak": [
                "0x0000000000000000000000005604b855b3708057705f8dfc0e6470917082c43a0000000000000000000000000000000000000000000000000000000000000000"
            ],
            "outOfGas": false,
            "to": "0x0000000071727de22e5e9d8baf0edac6f37da032",
            "type": "CALL",
            "usedOpcodes": {"0x0":1, "0x20":1, "0x34":1, "0x35":2, "0x36":2, "0x51":1, "0x52":4, "0x54":1, "0x55":1, "0x56":8, "0x57":18, "0x5b":10, "0xa2":1},
            "value": "0xde0b6b3a7640000"
        }
        "#;

        let trace: Erc7562Frame = serde_json::from_str(trace).unwrap();

        assert_eq!(trace.accessed_slots.reads.len(), 1);
        assert_eq!(trace.accessed_slots.writes.len(), 1);
        assert_eq!(trace.accessed_slots.transient_reads.len(), 0);
        assert_eq!(trace.accessed_slots.transient_writes.len(), 0);
        assert_eq!(trace.contract_size.len(), 0);
    }

    #[test]
    fn test_deserialize_erc7562_trace_nested() {
        let trace = r#"
        {
            "accessedSlots": {
              "reads": {
                "0x0000000000000000000000000000000000000000000000000000000000000002": [
                  "0x0000000000000000000000000000000000000000000000000000000000000001"
                ],
                "0x83d2064309e31181791f895d99cc244865c480f99200cd4d4f2412a7a3a265b6": [
                  "0x0000000000000000000000000000000000000000000000000000000000000000"
                ],
                "0xe18cacf0f1fc038f916461d29104232bd93d822359dfc137927858cc71f1e4ed": [
                  "0x0000000000000000000000000000000000000000000000001bc16d674ec80000"
                ]
              },
              "transientReads": {},
              "transientWrites": {},
              "writes": {
                    "0x0000000000000000000000000000000000000000000000000000000000000002": 2,
                    "0x83d2064309e31181791f895d99cc244865c480f99200cd4d4f2412a7a3a265b6": 1,
                    "0xe18cacf0f1fc038f916461d29104232bd93d822359dfc137927858cc71f1e4ed": 1
                  }
            },
            "calls": [
              {
                "accessedSlots": {
                  "reads": {},
                  "transientReads": {},
                  "transientWrites": {},
                  "writes": {}
                },
                "contractSize": {},
                "extCodeAccessInfo": [],
                "from": "0x0000000071727de22e5e9d8baf0edac6f37da032",
                "gas": "0x54dda",
                "gasUsed": "0x8ec",
                "input": "0x19822f7c000000000000000000000000000000000000000000000000000000000000006088a9b2626e43da02f978ae6cc89feffb68afcd5860cb9239337352db4b694fe100000000000000000000000000000000000000000000000000000000000000000000000000000000000000008c9d927336adc963536122f8e0d269319e79ed7a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001200000000000000000000000000000000000000000000000000000000000000140000000000000000000000000000f4240000000000000000000000000000493e000000000000000000000000000000000000000000000000000000000000493e0000000000000000000000000b2d05e00000000000000000000000000ee6b280000000000000000000000000000000000000000000000000000000000000001a000000000000000000000000000000000000000000000000000000000000001c000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000024a9e966b7000000000000000000000000000000000000000000000000000000000010f4470000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002face000000000000000000000000000000000000000000000000000000000000",
                "outOfGas": false,
                "output": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "to": "0x8c9d927336adc963536122f8e0d269319e79ed7a",
                "type": "CALL",
                "usedOpcodes": {"0x34":1, "0x35":9, "0x36":6, "0x51":1, "0x52":2, "0x56":102, "0x57":19, "0x5b":108, "0xf3":1},
                "value": "0x0"
              },
              {
                "accessedSlots": {
                  "reads": {
                    "0xe18cacf0f1fc038f916461d29104232bd93d822359dfc137927858cc71f1e4ed": [
                      "0x0000000000000000000000000000000000000000000000001baab0a330380000"
                    ]
                  },
                  "transientReads": {},
                  "transientWrites": {},
                  "writes": {
                        "0xe18cacf0f1fc038f916461d29104232bd93d822359dfc137927858cc71f1e4ed": 1
                      }
                },
                "calls": [
                  {
                    "accessedSlots": {
                      "reads": {
                        "0x0000000000000000000000000000000000000000000000000000000000000001": [
                          "0x0000000000000000000000000000000000000000000000000000000000000000"
                        ]
                      },
                      "transientReads": {},
                      "transientWrites": {},
                      "writes": {
                            "0x0000000000000000000000000000000000000000000000000000000000000001": 1
                          }
                    },
                    "contractSize": {},
                    "extCodeAccessInfo": [],
                    "from": "0x0000000071727de22e5e9d8baf0edac6f37da032",
                    "gas": "0x493e0",
                    "gasUsed": "0x5956",
                    "input": "0xa9e966b7000000000000000000000000000000000000000000000000000000000010f447",
                    "outOfGas": false,
                    "to": "0x8c9d927336adc963536122f8e0d269319e79ed7a",
                    "type": "CALL",
                    "usedOpcodes": {"0x34":1, "0x35":2, "0x36":2, "0x51":1, "0x52":1, "0x54":1, "0x55":1, "0x56":33, "0x57":9, "0x5b":35, "0xf3":1},
                    "value": "0x0"
                  }
                ],
                "contractSize": {
                  "0x8c9d927336adc963536122f8e0d269319e79ed7a": {
                    "contractSize": 2946,
                    "opcode": 241
                  }
                },
                "extCodeAccessInfo": [],
                "from": "0x0000000071727de22e5e9d8baf0edac6f37da032",
                "gas": "0x4d7ac",
                "gasUsed": "0x6ff7",
                "input": "0x0042dc5300000000000000000000000000000000000000000000000000000000000002000000000000000000000000008c9d927336adc963536122f8e0d269319e79ed7a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000f424000000000000000000000000000000000000000000000000000000000000493e00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000493e0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ee6b280000000000000000000000000000000000000000000000000000000000b2d05e0088a9b2626e43da02f978ae6cc89feffb68afcd5860cb9239337352db4b694fe10000000000000000000000000000000000000000000000000016bcc41e900000000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000522d600000000000000000000000000000000000000000000000000000000000002600000000000000000000000000000000000000000000000000000000000000024a9e966b7000000000000000000000000000000000000000000000000000000000010f447000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                "outOfGas": false,
                "output": "0x000000000000000000000000000000000000000000000000000421bab3f40fbc",
                "to": "0x0000000071727de22e5e9d8baf0edac6f37da032",
                "type": "CALL",
                "usedOpcodes": {"0x20":1, "0x30":1, "0x33":1, "0x34":1, "0x35":19, "0x36":7, "0x37":2, "0x48":1, "0x51":26, "0x52":31, "0x54":1, "0x55":1, "0x56":27, "0x57":33, "0x5a":5, "0x5b":35, "0xa4":1, "0xf1":1, "0xf3":1},
                "value": "0x0"
              },
              {
                "accessedSlots": {
                  "reads": {},
                  "transientReads": {},
                  "transientWrites": {},
                  "writes": {}
                },
                "contractSize": {},
                "extCodeAccessInfo": [],
                "from": "0x0000000071727de22e5e9d8baf0edac6f37da032",
                "gas": "0x44ece",
                "gasUsed": "0x0",
                "input": "0x",
                "outOfGas": false,
                "to": "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266",
                "type": "CALL",
                "usedOpcodes": {},
                "value": "0x421bab3f40fbc"
              }
            ],
            "contractSize": {
              "0x0000000071727de22e5e9d8baf0edac6f37da032": {
                "contractSize": 16035,
                "opcode": 241
              },
              "0x8c9d927336adc963536122f8e0d269319e79ed7a": {
                "contractSize": 2946,
                "opcode": 241
              },
              "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266": {
                "contractSize": 0,
                "opcode": 241
              }
            },
            "extCodeAccessInfo": [],
            "from": "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266",
            "gas": "0x5fe79",
            "gasUsed": "0x19415",
            "input": "0x765e827f0000000000000000000000000000000000000000000000000000000000000040000000000000000000000000f39fd6e51aad88f6f4ce6ab8827279cfffb92266000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000008c9d927336adc963536122f8e0d269319e79ed7a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001200000000000000000000000000000000000000000000000000000000000000140000000000000000000000000000f4240000000000000000000000000000493e000000000000000000000000000000000000000000000000000000000000493e0000000000000000000000000b2d05e00000000000000000000000000ee6b280000000000000000000000000000000000000000000000000000000000000001a000000000000000000000000000000000000000000000000000000000000001c000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000024a9e966b7000000000000000000000000000000000000000000000000000000000010f4470000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002face000000000000000000000000000000000000000000000000000000000000",
            "keccak": [
                "0x",
                "0x0000000000000000000000000000000000000000000000000000000000000000916f81e4e1b2122d13f0474f4c323777192f91bb579723004f6f3062b5fedc68",
                "0x0000000000000000000000008c9d927336adc963536122f8e0d269319e79ed7a0000000000000000000000000000000000000000000000000000000000000000",
                "0x0000000000000000000000008c9d927336adc963536122f8e0d269319e79ed7a0000000000000000000000000000000000000000000000000000000000000000c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470ede3d138a3c0ac5537239d818f52c7c86b466472a500982b0e7dff43ab38975d000000000000000000000000000f4240000000000000000000000000000493e000000000000000000000000000000000000000000000000000000000000493e0000000000000000000000000b2d05e00000000000000000000000000ee6b2800c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470",
                "0x0000000000000000000000008c9d927336adc963536122f8e0d269319e79ed7a0000000000000000000000000000000000000000000000000000000000000001",
                "0xa9e966b7000000000000000000000000000000000000000000000000000000000010f447",
                "0xc72bc304a44b01f425579bd71219ccd5676c732ce2aed103da05dc145f00fe340000000000000000000000000000000071727de22e5e9d8baf0edac6f37da0320000000000000000000000000000000000000000000000000000000000000539"
              ],
            "outOfGas": false,
            "to": "0x0000000071727de22e5e9d8baf0edac6f37da032",
            "type": "CALL",
            "usedOpcodes": {"0x0":1, "0x20":9, "0x30":2, "0x34":1, "0x35":43, "0x36":24, "0x37":8, "0x38":2, "0x3d":2, "0x46":1, "0x51":56, "0x52":104, "0x54":4, "0x55":4, "0x56":92, "0x57":101, "0x5a":4, "0x5b":116, "0xa1":1, "0xf1":3},
            "value": "0x0"
          }
        "#;

        let trace: Erc7562Frame = serde_json::from_str(trace).unwrap();

        assert_eq!(trace.accessed_slots.reads.len(), 3);
        assert_eq!(trace.accessed_slots.writes.len(), 3);

        assert_eq!(trace.contract_size.len(), 3);

        assert_eq!(trace.calls[1].calls.len(), 1);

        assert_eq!(trace.used_opcodes[&53], 43);
    }
}
