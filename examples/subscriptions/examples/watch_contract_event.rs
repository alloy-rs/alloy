use alloy_network::Ethereum;
use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_provider::{Provider, RootProvider};
use alloy_pubsub::PubSubFrontend;
use alloy_rpc_client::RpcClient;
use alloy_rpc_types::Filter;
use alloy_sol_types::{sol, SolEvent};
use eyre::Result;
use futures_util::{stream, StreamExt};

sol!(
    #[sol(rpc, bytecode = "0x60806040526000805534801561001457600080fd5b50610260806100246000396000f3fe608060405234801561001057600080fd5b50600436106100415760003560e01c80632baeceb71461004657806361bc221a14610050578063d09de08a1461006e575b600080fd5b61004e610078565b005b6100586100d9565b6040516100659190610159565b60405180910390f35b6100766100df565b005b600160008082825461008a91906101a3565b925050819055506000543373ffffffffffffffffffffffffffffffffffffffff167fdc69c403b972fc566a14058b3b18e1513da476de6ac475716e489fae0cbe4a2660405160405180910390a3565b60005481565b60016000808282546100f191906101e6565b925050819055506000543373ffffffffffffffffffffffffffffffffffffffff167ff6d1d8d205b41f9fb9549900a8dba5d669d68117a3a2b88c1ebc61163e8117ba60405160405180910390a3565b6000819050919050565b61015381610140565b82525050565b600060208201905061016e600083018461014a565b92915050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052601160045260246000fd5b60006101ae82610140565b91506101b983610140565b92508282039050818112600084121682821360008512151617156101e0576101df610174565b5b92915050565b60006101f182610140565b91506101fc83610140565b92508282019050828112156000831216838212600084121516171561022457610223610174565b5b9291505056fea26469706673582212208d0d34c26bfd2938ff07dd54c3fcc2bc4509e4ae654edff58101e5e7ab8cf18164736f6c63430008180033")]
    contract EventExample {
        int256 public counter = 0;

        event Increment(address indexed by, int256 indexed value);
        // event Decrement(address indexed by, int256 indexed value); // Uncommenting this line would cause the duplicate definitions for EventExampleInstance_filter

        function increment() public {
            counter += 1;
            emit Increment(msg.sender, counter);
        }

        // function decrement() public {
        //     counter -= 1;
        //     emit Decrement(msg.sender, counter);
        // }
    }
);

#[tokio::main]
async fn main() -> Result<()> {
    let (provider, _anvil) = init().await;

    let deployed_contract = EventExample::deploy(provider.clone()).await?;

    println!("Deployed contract at: {:?}", deployed_contract.address());

    let filter = Filter::new()
        .address(deployed_contract.address().to_owned())
        .event_signature(EventExample::Increment::SIGNATURE_HASH);

    let poller = provider.watch_logs(&filter).await?;

    println!("Watching for events...");
    println!("every {:?}", poller.poll_interval()); // Default 250ms

    let mut stream = poller.into_stream().flat_map(stream::iter).take(5);

    // Build a call to increment the counter
    let increment_call = deployed_contract.increment();

    // Send the increment call 5 times
    for _ in 0..5 {
        let _ = increment_call.send().await?;
    }

    while let Some(log) = stream.next().await {
        println!("Received log: {:?}", log);
    }

    Ok(())
}

async fn init() -> (RootProvider<Ethereum, PubSubFrontend>, AnvilInstance) {
    let anvil = Anvil::new().block_time(1).spawn();
    let ws = alloy_rpc_client::WsConnect::new(anvil.ws_endpoint());
    let client = RpcClient::connect_pubsub(ws).await.unwrap();
    let provider = RootProvider::<Ethereum, _>::new(client);

    (provider, anvil)
}
