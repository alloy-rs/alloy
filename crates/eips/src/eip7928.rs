use alloy_primitives::{Address, Bytes, B256};

#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PerTxAccess {
    pub tx_index: u16,
    pub value_after: B256,
}

#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SlotAccess {
    pub slot: B256,
    pub accesses: Vec<PerTxAccess>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AccountAccess {
    pub address: Address,
    pub accesses: Vec<SlotAccess>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BalanceChange {
    pub tx_index: u16,
    pub delta: i64,
}

#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AccountBalanceDiff {
    pub address: Address,
    pub changes: Vec<BalanceChange>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CodeChange {
    pub tx_index: u16,
    pub new_code: Bytes,
}

#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AccountCodeDiff {
    pub address: Address,
    pub change: CodeChange,
}

#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AccountNonce {
    pub address: Address,
    pub nonce_before: u64,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockAccessList {
    pub account_addresses: Vec<AccountAccess>,
    pub balance_diffs: Vec<AccountBalanceDiff>,
    pub code_diffs: Vec<AccountCodeDiff>,
    pub nonce_diffs: Vec<AccountNonce>,
}

#[cfg(feature = "ssz")]
pub mod ssz_helpers {
    use super::*;
    use alloy_primitives::FixedBytes;
    use ssz_types::{
        typenum::{
            Sum, Unsigned, U1024, U128, U16, U16384, U256, U262144, U32, U32768, U4096, U512, U64,
            U8192,
        },
        VariableList,
    };

    pub const MAX_TXS: usize = 30_000;
    pub const MAX_SLOTS: usize = 300_000;
    pub const MAX_ACCOUNTS: usize = 300_000;
    pub const MAX_CODE_SIZE: usize = 24_576;

    type TxIndex = u16;

    type U30_000 = Sum<U16384, Sum<U8192, Sum<U4096, Sum<U1024, Sum<U256, Sum<U32, U16>>>>>>;

    type U300_000 =
        Sum<U262144, Sum<U32768, Sum<U4096, Sum<U512, Sum<U256, Sum<U128, Sum<U64, U32>>>>>>>;

    #[derive(ssz_derive::Encode, ssz_derive::Decode)]
    struct BlockAccessListSszHelper {
        account_addresses: VariableList<AccountAccessSszHelper, U300_000>,
        balance_diffs: VariableList<AccountBalanceDiffSszHelper, U300_000>,
        code_diffs: VariableList<AccountCodeDiffSszHelper, U300_000>,
        nonce_diffs: VariableList<AccountNonceSszHelper, U30_000>,
    }

    impl ssz::Encode for BlockAccessList {
        fn is_ssz_fixed_len() -> bool {
            false
        }
    }

    #[derive(ssz_derive::Encode, ssz_derive::Decode)]
    struct AccountAccessSszHelper {
        address: Address,
        accesses: VariableList<SlotAccessSszHelper, U300_000>,
    }

    #[derive(ssz_derive::Encode, ssz_derive::Decode)]
    struct SlotAccessSszHelper {
        slot: B256,
        accesses: VariableList<PerTxAccessSszHelper, U30_000>,
    }

    #[derive(ssz_derive::Encode, ssz_derive::Decode)]
    struct PerTxAccessSszHelper {
        tx_index: TxIndex,
        value_after: B256,
    }

    #[derive(ssz_derive::Encode, ssz_derive::Decode)]
    struct AccountBalanceDiffSszHelper {
        address: Address,
        changes: VariableList<BalanceChangeSszHelper, U30_000>,
    }

    #[derive(ssz_derive::Encode, ssz_derive::Decode)]
    struct BalanceChangeSszHelper {
        tx_index: TxIndex,
        delta: FixedBytes<12>,
    }

    #[derive(ssz_derive::Encode, ssz_derive::Decode)]
    struct AccountCodeDiffSszHelper {
        address: Address,
        changes: CodeChangeSszHelper,
    }

    #[derive(ssz_derive::Encode, ssz_derive::Decode)]
    struct CodeChangeSszHelper {
        tx_index: TxIndex,
        new_code: FixedBytes<MAX_CODE_SIZE>,
    }

    #[derive(ssz_derive::Encode, ssz_derive::Decode)]
    struct AccountNonceSszHelper {
        address: Address,
        nonce_before: u64,
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_list_size_consts() {
            assert_eq!(U30_000::to_u64(), MAX_TXS as u64);
            assert_eq!(U300_000::to_u64(), MAX_SLOTS as u64);
            assert_eq!(U300_000::to_u64(), MAX_ACCOUNTS as u64);
        }
    }
}
