use alloy_network::Ethereum;
use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_primitives::fixed_bytes;
use alloy_provider::{HttpProvider, Provider};
use alloy_rpc_client::RpcClient;
use alloy_rpc_trace_types::geth::{
    GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingOptions,
    GethDefaultTracingOptions,
};
use alloy_transport_http::Http;
use eyre::Result;
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<()> {
    let (provider, _anvil) = init();
    let hash = fixed_bytes!("97a02abf405d36939e5b232a5d4ef5206980c5a6661845436058f30600c52df7"); // Hash of the tx we want to trace

    // Default tracing
    let default_options = GethDebugTracingOptions::default();
    let res = provider.debug_trace_transaction(hash, default_options).await?;

    println!("DEFAULT_TRACE: {:?}", res);

    // Call tracing
    let call_options = GethDebugTracingOptions {
        config: GethDefaultTracingOptions {
            disable_storage: Some(true),
            enable_memory: Some(false),
            ..Default::default()
        },
        tracer: Some(GethDebugTracerType::BuiltInTracer(GethDebugBuiltInTracerType::CallTracer)),
        ..Default::default()
    };
    let res = provider.debug_trace_transaction(hash, call_options).await?;

    println!("CALL_TRACE: {:?}", res);

    // JS tracer
    let js_options = GethDebugTracingOptions {
        tracer: Some(GethDebugTracerType::JsTracer("{data: [], fault: function(log) {}, step: function(log) { if(log.op.toString() == \"DELEGATECALL\") this.data.push(log.stack.peek(0)); }, result: function() { return this.data; }}".into())),
        ..Default::default()
    };

    let res = provider.debug_trace_transaction(hash, js_options).await?;

    println!("JS_TRACER: {:?}", res);

    Ok(())
}

fn init() -> (HttpProvider<Ethereum>, AnvilInstance) {
    let anvil = Anvil::new().fork("https://eth.llamarpc.com").spawn();
    let url = anvil.endpoint().parse().unwrap();
    let http = Http::<Client>::new(url);
    (HttpProvider::new(RpcClient::new(http, true)), anvil)
}
