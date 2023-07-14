use alloy_transports::{RpcParam, RpcResp};

trait RpcMethod<N>: RpcParam {
    const METHOD: &'static str;
    type Output: RpcResp;
}
