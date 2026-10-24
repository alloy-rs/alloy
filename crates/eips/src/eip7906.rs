//! [EIP-7906] transaction assertion selector types.
//!
//! [EIP-7906]: <https://eips.ethereum.org/EIPS/eip-7906>

/// A `TXTRACE` selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TxTraceParam {
    /// Number of accounts with a net balance change.
    BalancesChanged = 0x00,
    /// Number of account storage slots with a net value change.
    SlotsChanged = 0x01,
    /// Number of accounts with a net code-hash change.
    CodesChanged = 0x02,
    /// Number of accounts with a net account-nonce change.
    NoncesChanged = 0x03,
    /// Address for a balance-change entry.
    BalanceChangeAddress = 0x04,
    /// Pre-transaction balance for a balance-change entry.
    BalanceBefore = 0x05,
    /// Current balance for a balance-change entry.
    BalanceAfter = 0x06,
    /// Address for a storage-change entry.
    SlotChangeAddress = 0x07,
    /// Pre-transaction value for a storage-change entry.
    SlotValueBefore = 0x08,
    /// Current value for a storage-change entry.
    SlotValueAfter = 0x09,
    /// Key for a storage-change entry.
    SlotKey = 0x0A,
    /// Address for a code-hash-change entry.
    CodeChangeAddress = 0x0B,
    /// Pre-transaction stored-code hash for a code-hash-change entry.
    CodeHashBefore = 0x0C,
    /// Current stored-code hash for a code-hash-change entry.
    CodeHashAfter = 0x0D,
    /// Address for an account-nonce-change entry.
    NonceChangeAddress = 0x0E,
    /// Pre-transaction account nonce for an account-nonce-change entry.
    NonceBefore = 0x0F,
    /// Current account nonce for an account-nonce-change entry.
    NonceAfter = 0x10,
    /// Number of events emitted by the transaction.
    EventsCount = 0x11,
    /// Emitting address for an event entry.
    EventAddress = 0x12,
    /// Number of topics in an event entry.
    EventTopicCount = 0x13,
    /// Topic zero for an event entry.
    EventTopic0 = 0x14,
    /// Topic one for an event entry.
    EventTopic1 = 0x15,
    /// Topic two for an event entry.
    EventTopic2 = 0x16,
    /// Topic three for an event entry.
    EventTopic3 = 0x17,
    /// Non-indexed data length for an event entry.
    EventDataLength = 0x18,
    /// Amount deducted from the gas payer before execution.
    GasPreCharge = 0x19,
    /// Address charged the gas pre-payment.
    GasPayerAddress = 0x1A,
}

impl TxTraceParam {
    /// Returns whether this selector requires its index operand to be zero.
    pub const fn requires_zero_index(self) -> bool {
        matches!(
            self,
            Self::BalancesChanged
                | Self::SlotsChanged
                | Self::CodesChanged
                | Self::NoncesChanged
                | Self::EventsCount
                | Self::GasPreCharge
                | Self::GasPayerAddress
        )
    }
}

macro_rules! selector_conversions {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        impl From<$name> for u8 {
            fn from(value: $name) -> Self {
                value as Self
            }
        }

        impl TryFrom<u8> for $name {
            type Error = u8;

            fn try_from(value: u8) -> Result<Self, Self::Error> {
                match value {
                    $(x if x == Self::$variant as u8 => Ok(Self::$variant),)+
                    _ => Err(value),
                }
            }
        }
    };
}

selector_conversions!(TxTraceParam {
    BalancesChanged,
    SlotsChanged,
    CodesChanged,
    NoncesChanged,
    BalanceChangeAddress,
    BalanceBefore,
    BalanceAfter,
    SlotChangeAddress,
    SlotValueBefore,
    SlotValueAfter,
    SlotKey,
    CodeChangeAddress,
    CodeHashBefore,
    CodeHashAfter,
    NonceChangeAddress,
    NonceBefore,
    NonceAfter,
    EventsCount,
    EventAddress,
    EventTopicCount,
    EventTopic0,
    EventTopic1,
    EventTopic2,
    EventTopic3,
    EventDataLength,
    GasPreCharge,
    GasPayerAddress,
});
