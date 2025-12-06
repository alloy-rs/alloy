//! Geth call tracer types.

use crate::parity::{ActionType, CallType, CreationMethod, LocalizedTransactionTrace};
use alloy_primitives::{Address, Bytes, Selector, B256, U256};
use serde::{Deserialize, Serialize};

/// The response object for `debug_traceTransaction` with `"tracer": "callTracer"`.
///
/// <https://github.com/ethereum/go-ethereum/blob/91cb6f863a965481e51d5d9c0e5ccd54796fd967/eth/tracers/native/call.go#L44>
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallFrame {
    /// The address of that initiated the call.
    pub from: Address,
    /// How much gas was left before the call.
    #[serde(default)]
    pub gas: U256,
    /// How much gas was used by the call.
    #[serde(default, rename = "gasUsed")]
    pub gas_used: U256,
    /// The address of the contract that was called.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Address>,
    /// Calldata input.
    pub input: Bytes,
    /// Output of the call, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Bytes>,
    /// Error message, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Why this call reverted, if it reverted.
    #[serde(default, rename = "revertReason", skip_serializing_if = "Option::is_none")]
    pub revert_reason: Option<String>,
    /// Recorded child calls.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calls: Vec<Self>,
    /// Logs emitted by this call.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub logs: Vec<CallLogFrame>,
    /// Value transferred.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<U256>,
    /// The type of the call.
    #[serde(rename = "type")]
    pub typ: String,
}

impl CallFrame {
    /// Error selector is the first 4 bytes of calldata
    pub fn selector(&self) -> Option<Selector> {
        if self.input.len() < 4 {
            return None;
        }
        Some(Selector::from_slice(&self.input[..4]))
    }

    /// Returns true if this call reverted.
    pub fn is_revert(&self) -> bool {
        if self.revert_reason.is_some() {
            return true;
        }
        matches!(self.error.as_deref(), Some("execution reverted"))
    }

    /// Returns true if this is a regular call
    pub fn is_call(&self) -> bool {
        self.typ == CallKind::Call
    }

    /// Returns true if this is a delegate call
    pub fn is_delegate_call(&self) -> bool {
        self.typ == CallKind::DelegateCall
    }

    /// Returns true if this is a static call
    pub fn is_static_call(&self) -> bool {
        self.typ == CallKind::StaticCall
    }

    /// Returns true if this is a auth call
    pub fn is_auth_call(&self) -> bool {
        self.typ == CallKind::AuthCall
    }
}

/// Represents a recorded log that is emitted during a trace call.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallLogFrame {
    /// The address of the contract that was called.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<Address>,
    /// The topics of the log.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topics: Option<Vec<B256>>,
    /// The data of the log.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Bytes>,
    /// The position of the log relative to subcalls within the same trace.
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub position: Option<u64>,
    /// The index of the log in the trace.
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub index: Option<u64>,
}

impl CallLogFrame {
    /// Converts this log frame into a primitives log object
    pub fn into_log(self) -> alloy_primitives::Log {
        alloy_primitives::Log::new_unchecked(
            self.address.unwrap_or_default(),
            self.topics.unwrap_or_default(),
            self.data.unwrap_or_default(),
        )
    }
}

impl From<CallLogFrame> for alloy_primitives::Log {
    fn from(value: CallLogFrame) -> Self {
        value.into_log()
    }
}

/// The configuration for the call tracer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallConfig {
    /// When set to true, this will only trace the primary (top-level) call and not any sub-calls.
    /// It eliminates the additional processing for each call frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub only_top_call: Option<bool>,
    /// When set to true, this will include the logs emitted by the call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_log: Option<bool>,
}

impl CallConfig {
    /// Sets the only top call flag.
    pub const fn only_top_call(mut self) -> Self {
        self.only_top_call = Some(true);
        self
    }

    /// Sets the with log flag.
    pub const fn with_log(mut self) -> Self {
        self.with_log = Some(true);
        self
    }
}

/// The response object for `debug_traceTransaction` with `"tracer": "flatCallTracer"`.
///
/// That is equivalent to parity's [`LocalizedTransactionTrace`]
/// <https://github.com/ethereum/go-ethereum/blob/0dd173a727dd2d2409b8e401b22e85d20c25b71f/eth/tracers/native/call_flat.go#L62-L62>
pub type FlatCallFrame = Vec<LocalizedTransactionTrace>;

/// The configuration for the flat call tracer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlatCallConfig {
    /// If true, call tracer converts errors to parity format
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convert_parity_errors: Option<bool>,
    /// If true, call tracer includes calls to precompiled contracts
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_precompiles: Option<bool>,
}

impl FlatCallConfig {
    /// Converts errors to parity format.
    pub const fn parity_errors(mut self) -> Self {
        self.convert_parity_errors = Some(true);
        self
    }

    /// Include calls to precompiled contracts.
    pub const fn with_precompiles(mut self) -> Self {
        self.include_precompiles = Some(true);
        self
    }
}

/// A single item yielded by [`CallFrameIter`].
///
/// It gives access to the current frame, its parent, and
/// allows skipping traversal into its children.
#[derive(Debug, PartialEq)]
pub struct CallFrameItem<'a> {
    frame: &'a CallFrame,
    parent: Option<&'a CallFrame>,
}

impl<'a> CallFrameItem<'a> {
    /// The current frame.
    pub const fn frame(&self) -> &CallFrame {
        self.frame
    }

    /// The parent of this frame, if any.
    pub const fn parent(&self) -> Option<&CallFrame> {
        self.parent
    }
}

/// An iterator for traversing `CallFrame` hierarchies.
///
/// Traversal is **depth-first** by default.
/// Children of a frame can be skipped using [`CallFrameIter::skip_children`].
#[derive(Debug)]
pub struct CallFrameIter<'a> {
    /// Stack of (frame-item reference, parent reference)
    stack: Vec<(&'a CallFrame, Option<&'a CallFrame>)>,
    /// The last frame that was yielded.
    last_frame: Option<&'a CallFrame>,
}

impl<'a> CallFrameIter<'a> {
    /// Creates a new iterator starting from `root`.
    pub fn new(root: &'a CallFrame) -> Self {
        Self { stack: vec![(root, None)], last_frame: None }
    }

    /// Skips children for the most recently yielded item.
    /// Note: this would panic if there are no parent owning the children to
    /// skip. `next` must be called before `skip_children`.
    pub fn skip_children(&mut self) {
        if let Some(parent) = self.last_frame {
            let _ = self.stack.split_off(self.stack.len() - parent.calls.len());
        }
    }
}

impl<'a> Iterator for CallFrameIter<'a> {
    type Item = CallFrameItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (frame, parent) = self.stack.pop()?;

        // Add children in reverse order so they're processed in the correct order
        for child in frame.calls.iter().rev() {
            self.stack.push((child, Some(frame)));
        }

        // Create and return the item
        let item = CallFrameItem { frame, parent };
        self.last_frame = Some(frame);

        Some(item)
    }
}

/// A unified representation of a call.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CallKind {
    /// Represents a regular call.
    #[default]
    Call,
    /// Represents a static call.
    StaticCall,
    /// Represents a call code operation.
    CallCode,
    /// Represents a delegate call.
    DelegateCall,
    /// Represents an authorized call.
    AuthCall,
    /// Represents a contract creation operation.
    Create,
    /// Represents a contract creation operation using the CREATE2 opcode.
    Create2,
}

impl CallKind {
    /// Returns the string representation of the call kind.
    pub const fn to_str(self) -> &'static str {
        match self {
            Self::Call => "CALL",
            Self::StaticCall => "STATICCALL",
            Self::CallCode => "CALLCODE",
            Self::DelegateCall => "DELEGATECALL",
            Self::AuthCall => "AUTHCALL",
            Self::Create => "CREATE",
            Self::Create2 => "CREATE2",
        }
    }

    /// Returns true if the call is a create
    #[inline]
    pub const fn is_any_create(&self) -> bool {
        matches!(self, Self::Create | Self::Create2)
    }

    /// Returns true if the call is a delegate of some sorts
    #[inline]
    pub const fn is_delegate(&self) -> bool {
        matches!(self, Self::DelegateCall | Self::CallCode)
    }

    /// Returns true if the call is [CallKind::StaticCall].
    #[inline]
    pub const fn is_static_call(&self) -> bool {
        matches!(self, Self::StaticCall)
    }

    /// Returns true if the call is [CallKind::AuthCall].
    #[inline]
    pub const fn is_auth_call(&self) -> bool {
        matches!(self, Self::AuthCall)
    }
}

impl core::fmt::Display for CallKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.to_str())
    }
}

impl PartialEq<String> for CallKind {
    fn eq(&self, other: &String) -> bool {
        self.to_str() == other.as_str()
    }
}

impl PartialEq<CallKind> for String {
    fn eq(&self, other: &CallKind) -> bool {
        self.as_str() == other.to_str()
    }
}

impl PartialEq<&str> for CallKind {
    fn eq(&self, other: &&str) -> bool {
        self.to_str() == *other
    }
}

impl PartialEq<CallKind> for &str {
    fn eq(&self, other: &CallKind) -> bool {
        *self == other.to_str()
    }
}

impl From<CallKind> for CreationMethod {
    fn from(kind: CallKind) -> Self {
        match kind {
            CallKind::Create => Self::Create,
            CallKind::Create2 => Self::Create2,
            _ => Self::None,
        }
    }
}

impl From<CallKind> for ActionType {
    fn from(kind: CallKind) -> Self {
        match kind {
            CallKind::Call
            | CallKind::StaticCall
            | CallKind::DelegateCall
            | CallKind::CallCode
            | CallKind::AuthCall => Self::Call,
            CallKind::Create | CallKind::Create2 => Self::Create,
        }
    }
}

impl From<CallKind> for CallType {
    fn from(ty: CallKind) -> Self {
        match ty {
            CallKind::Call => Self::Call,
            CallKind::StaticCall => Self::StaticCall,
            CallKind::CallCode => Self::CallCode,
            CallKind::DelegateCall => Self::DelegateCall,
            CallKind::Create | CallKind::Create2 => Self::None,
            CallKind::AuthCall => Self::AuthCall,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geth::*;
    use similar_asserts::assert_eq;

    // See <https://github.com/ethereum/go-ethereum/tree/master/eth/tracers/internal/tracetest/testdata>
    const DEFAULT: &str = include_str!("../../test_data/call_tracer/default.json");
    const MULTI_DEFAULT: &str = include_str!("../../test_data/call_tracer/multi_call_default.json");
    const DELEGATE_DEFAULT: &str =
        include_str!("../../test_data/call_tracer/default_with_delegate_call.json");
    const LEGACY: &str = include_str!("../../test_data/call_tracer/legacy.json");
    const ONLY_TOP_CALL: &str = include_str!("../../test_data/call_tracer/only_top_call.json");
    const WITH_LOG: &str = include_str!("../../test_data/call_tracer/with_log.json");

    #[test]
    fn test_serialize_call_trace() {
        let mut opts = GethDebugTracingCallOptions::default();
        opts.tracing_options.config.disable_storage = Some(false);
        opts.tracing_options.tracer =
            Some(GethDebugTracerType::BuiltInTracer(GethDebugBuiltInTracerType::CallTracer));
        opts.tracing_options.tracer_config =
            serde_json::to_value(CallConfig { only_top_call: Some(true), with_log: Some(true) })
                .unwrap()
                .into();

        assert_eq!(
            serde_json::to_string(&opts).unwrap(),
            r#"{"disableStorage":false,"tracer":"callTracer","tracerConfig":{"onlyTopCall":true,"withLog":true}}"#
        );
    }

    #[test]
    fn test_deserialize_call_trace() {
        let _trace: CallFrame = serde_json::from_str(DEFAULT).unwrap();
        let _trace: CallFrame = serde_json::from_str(LEGACY).unwrap();
        let _trace: CallFrame = serde_json::from_str(ONLY_TOP_CALL).unwrap();
        let _trace: CallFrame = serde_json::from_str(WITH_LOG).unwrap();
    }

    #[test]
    fn test_call_frame_iter() {
        let mut init_frame: CallFrame = serde_json::from_str(DEFAULT).unwrap();
        init_frame.calls.push(init_frame.clone());

        let mut call_iter = CallFrameIter::new(&init_frame);

        let call_1 = call_iter.next().unwrap();
        assert_eq!(call_1.frame().calls.len(), 2);
        assert_eq!(*call_1.frame(), init_frame);

        let call_2 = call_iter.next().unwrap();
        assert_eq!(call_2.frame().calls.len(), 0);
        let init_frame_raw: CallFrame = serde_json::from_str(DEFAULT).unwrap();
        assert_eq!(*call_2.frame(), init_frame_raw.calls[0]);

        let call_3 = call_iter.next().unwrap();
        assert_eq!(call_3.frame().calls.len(), 1);
        assert_eq!(*call_3.frame(), init_frame_raw);

        let call_4 = call_iter.next().unwrap();
        assert_eq!(call_4.frame().calls.len(), 0);
        assert_eq!(*call_4.frame(), init_frame_raw.calls[0]);
    }

    #[test]
    fn test_call_frame_iter_with_delegate_call() {
        let init_frame: CallFrame = serde_json::from_str(DELEGATE_DEFAULT).unwrap();

        let mut call_iter = CallFrameIter::new(&init_frame);

        let call_1 = call_iter.next().unwrap();
        if call_1.frame().is_delegate_call() {
            call_iter.skip_children();
        }

        let call_2 = call_iter.next();
        assert_eq!(call_2, None);
    }

    #[test]
    fn test_call_frame_iter_with_multiple_skip_children() {
        let init_frame: CallFrame = serde_json::from_str(MULTI_DEFAULT).unwrap();

        let mut call_iter = CallFrameIter::new(&init_frame);

        let call_1 = call_iter.next().unwrap();
        assert_eq!(call_1.frame().value, Some(U256::from(1)));

        let call_2 = call_iter.next().unwrap();
        assert_eq!(call_2.frame().value, Some(U256::from(3)));

        let call_3 = call_iter.next().unwrap();
        assert_eq!(call_3.frame().value, Some(U256::from(5)));

        call_iter.skip_children();

        let call_4 = call_iter.next().unwrap();
        assert_eq!(call_4.frame().value, Some(U256::from(4)));

        let call_5 = call_iter.next().unwrap();
        assert_eq!(call_5.frame().value, Some(U256::from(2)));

        let call_6 = call_iter.next();
        assert_eq!(call_6, None);
    }

    #[test]
    fn test_call_log_frame_serde_with_regular_json_number() {
        // Test that CallLogFrame can deserialize index as a regular JSON number
        let json = r#"{
            "address": "0x0000000000000000000000000000000000000000",
            "topics": ["0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"],
            "data": "0x1234",
            "position": 5,
            "index": 10
        }"#;

        let log_frame: CallLogFrame = serde_json::from_str(json).unwrap();
        assert_eq!(log_frame.position, Some(5));
        assert_eq!(log_frame.index, Some(10));

        // Test serialization back to JSON with quantity format
        let serialized = serde_json::to_string(&log_frame).unwrap();
        let deserialized: CallLogFrame = serde_json::from_str(&serialized).unwrap();
        assert_eq!(log_frame, deserialized);

        // Test with hex values as well
        let json_hex = r#"{
            "address": "0x0000000000000000000000000000000000000000",
            "topics": ["0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"],
            "data": "0x1234",
            "position": "0x5",
            "index": "0xa"
        }"#;

        let log_frame_hex: CallLogFrame = serde_json::from_str(json_hex).unwrap();
        assert_eq!(log_frame_hex.position, Some(5));
        assert_eq!(log_frame_hex.index, Some(10));
        assert_eq!(log_frame, log_frame_hex);
    }
}
