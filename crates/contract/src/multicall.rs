//! Multicall module to organize and implement required functionality for enabling multicall in
//! alloy_provider plus related tests. This avoids cyclic deps between alloy_provider and
//! alloy_contract.
//!
//! This module is not public API.
use super::SolCallBuilder;
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::{Address, Bytes, U256};
use alloy_provider::{MulticallItem, Provider};
use alloy_sol_types::SolCall;

impl<P: Provider<N>, C: SolCall, N: Network> MulticallItem for SolCallBuilder<P, C, N> {
    type Decoder = C;

    fn value(&self) -> U256 {
        self.request.value().unwrap_or_default()
    }

    fn target(&self) -> Address {
        self.request.to().expect("`to` not set for `SolCallBuilder`")
    }

    fn input(&self) -> Bytes {
        self.calldata().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, hex, U256};
    use alloy_provider::{
        ext::AnvilApi, CallItem, Failure, MulticallBuilder, Provider, ProviderBuilder,
        MULTICALL3_ADDRESS,
    };
    use alloy_rpc_types_eth::BlockId;
    use alloy_sol_types::sol;
    use DummyThatFails::DummyThatFailsInstance;

    sol! {
        // solc 0.8.25; solc Token.sol --optimize --bin
        #[sol(rpc, bytecode = "6080604052348015600e575f80fd5b5060ca80601a5f395ff3fe6080604052348015600e575f80fd5b50600436106030575f3560e01c806318160ddd14603457806370a08231146052575b5f80fd5b69d3c21bcecceda10000005b60405190815260200160405180910390f35b6040605d3660046069565b6001600160a01b031690565b5f602082840312156078575f80fd5b81356001600160a01b0381168114608d575f80fd5b939250505056fea26469706673582212206247638e5ef60aeebc1ae1ed6fb387d828f26688d9eaff1effcf953986bee91a64736f6c63430008190033")]
        #[derive(Debug, PartialEq)]
        contract Token {
            function totalSupply() external pure returns (uint256) {
                return 1e24;
            }

            function balanceOf(address owner) external pure returns (uint256) {
                return uint160(owner);
            }
        }
    }

    sol! {
        // solc 0.8.25; solc DummyThatFails.sol --optimize --bin
        #[sol(rpc, bytecode = "6080604052348015600e575f80fd5b5060a780601a5f395ff3fe6080604052348015600e575f80fd5b50600436106030575f3560e01c80630b93381b146034578063a9cc4718146036575b5f80fd5b005b603460405162461bcd60e51b815260040160689060208082526004908201526319985a5b60e21b604082015260600190565b60405180910390fdfea2646970667358221220c90ee107375422bb3516f4f13cdd754387c374edb5d9815fb6aa5ca111a77cb264736f6c63430008190033")]
        #[derive(Debug)]
        contract DummyThatFails {
            function fail() external {
                revert("fail");
            }

            function success() external {}
        }
    }

    async fn deploy_dummy(
        provider: impl alloy_provider::Provider,
    ) -> DummyThatFailsInstance<impl alloy_provider::Provider> {
        DummyThatFails::deploy(provider).await.unwrap()
    }

    const MULTICALL3_DEPLOYED_CODE: &[u8] = &hex!("0x6080604052600436106100f35760003560e01c80634d2301cc1161008a578063a8b0574e11610059578063a8b0574e1461025a578063bce38bd714610275578063c3077fa914610288578063ee82ac5e1461029b57600080fd5b80634d2301cc146101ec57806372425d9d1461022157806382ad56cb1461023457806386d516e81461024757600080fd5b80633408e470116100c65780633408e47014610191578063399542e9146101a45780633e64a696146101c657806342cbb15c146101d957600080fd5b80630f28c97d146100f8578063174dea711461011a578063252dba421461013a57806327e86d6e1461015b575b600080fd5b34801561010457600080fd5b50425b6040519081526020015b60405180910390f35b61012d610128366004610a85565b6102ba565b6040516101119190610bbe565b61014d610148366004610a85565b6104ef565b604051610111929190610bd8565b34801561016757600080fd5b50437fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff0140610107565b34801561019d57600080fd5b5046610107565b6101b76101b2366004610c60565b610690565b60405161011193929190610cba565b3480156101d257600080fd5b5048610107565b3480156101e557600080fd5b5043610107565b3480156101f857600080fd5b50610107610207366004610ce2565b73ffffffffffffffffffffffffffffffffffffffff163190565b34801561022d57600080fd5b5044610107565b61012d610242366004610a85565b6106ab565b34801561025357600080fd5b5045610107565b34801561026657600080fd5b50604051418152602001610111565b61012d610283366004610c60565b61085a565b6101b7610296366004610a85565b610a1a565b3480156102a757600080fd5b506101076102b6366004610d18565b4090565b60606000828067ffffffffffffffff8111156102d8576102d8610d31565b60405190808252806020026020018201604052801561031e57816020015b6040805180820190915260008152606060208201528152602001906001900390816102f65790505b5092503660005b8281101561047757600085828151811061034157610341610d60565b6020026020010151905087878381811061035d5761035d610d60565b905060200281019061036f9190610d8f565b6040810135958601959093506103886020850185610ce2565b73ffffffffffffffffffffffffffffffffffffffff16816103ac6060870187610dcd565b6040516103ba929190610e32565b60006040518083038185875af1925050503d80600081146103f7576040519150601f19603f3d011682016040523d82523d6000602084013e6103fc565b606091505b50602080850191909152901515808452908501351761046d577f08c379a000000000000000000000000000000000000000000000000000000000600052602060045260176024527f4d756c746963616c6c333a2063616c6c206661696c656400000000000000000060445260846000fd5b5050600101610325565b508234146104e6576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152601a60248201527f4d756c746963616c6c333a2076616c7565206d69736d6174636800000000000060448201526064015b60405180910390fd5b50505092915050565b436060828067ffffffffffffffff81111561050c5761050c610d31565b60405190808252806020026020018201604052801561053f57816020015b606081526020019060019003908161052a5790505b5091503660005b8281101561068657600087878381811061056257610562610d60565b90506020028101906105749190610e42565b92506105836020840184610ce2565b73ffffffffffffffffffffffffffffffffffffffff166105a66020850185610dcd565b6040516105b4929190610e32565b6000604051808303816000865af19150503d80600081146105f1576040519150601f19603f3d011682016040523d82523d6000602084013e6105f6565b606091505b5086848151811061060957610609610d60565b602090810291909101015290508061067d576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152601760248201527f4d756c746963616c6c333a2063616c6c206661696c656400000000000000000060448201526064016104dd565b50600101610546565b5050509250929050565b43804060606106a086868661085a565b905093509350939050565b6060818067ffffffffffffffff8111156106c7576106c7610d31565b60405190808252806020026020018201604052801561070d57816020015b6040805180820190915260008152606060208201528152602001906001900390816106e55790505b5091503660005b828110156104e657600084828151811061073057610730610d60565b6020026020010151905086868381811061074c5761074c610d60565b905060200281019061075e9190610e76565b925061076d6020840184610ce2565b73ffffffffffffffffffffffffffffffffffffffff166107906040850185610dcd565b60405161079e929190610e32565b6000604051808303816000865af19150503d80600081146107db576040519150601f19603f3d011682016040523d82523d6000602084013e6107e0565b606091505b506020808401919091529015158083529084013517610851577f08c379a000000000000000000000000000000000000000000000000000000000600052602060045260176024527f4d756c746963616c6c333a2063616c6c206661696c656400000000000000000060445260646000fd5b50600101610714565b6060818067ffffffffffffffff81111561087657610876610d31565b6040519080825280602002602001820160405280156108bc57816020015b6040805180820190915260008152606060208201528152602001906001900390816108945790505b5091503660005b82811015610a105760008482815181106108df576108df610d60565b602002602001015190508686838181106108fb576108fb610d60565b905060200281019061090d9190610e42565b925061091c6020840184610ce2565b73ffffffffffffffffffffffffffffffffffffffff1661093f6020850185610dcd565b60405161094d929190610e32565b6000604051808303816000865af19150503d806000811461098a576040519150601f19603f3d011682016040523d82523d6000602084013e61098f565b606091505b506020830152151581528715610a07578051610a07576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152601760248201527f4d756c746963616c6c333a2063616c6c206661696c656400000000000000000060448201526064016104dd565b506001016108c3565b5050509392505050565b6000806060610a2b60018686610690565b919790965090945092505050565b60008083601f840112610a4b57600080fd5b50813567ffffffffffffffff811115610a6357600080fd5b6020830191508360208260051b8501011115610a7e57600080fd5b9250929050565b60008060208385031215610a9857600080fd5b823567ffffffffffffffff811115610aaf57600080fd5b610abb85828601610a39565b90969095509350505050565b6000815180845260005b81811015610aed57602081850181015186830182015201610ad1565b81811115610aff576000602083870101525b50601f017fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe0169290920160200192915050565b600082825180855260208086019550808260051b84010181860160005b84811015610bb1578583037fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe001895281518051151584528401516040858501819052610b9d81860183610ac7565b9a86019a9450505090830190600101610b4f565b5090979650505050505050565b602081526000610bd16020830184610b32565b9392505050565b600060408201848352602060408185015281855180845260608601915060608160051b870101935082870160005b82811015610c52577fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffa0888703018452610c40868351610ac7565b95509284019290840190600101610c06565b509398975050505050505050565b600080600060408486031215610c7557600080fd5b83358015158114610c8557600080fd5b9250602084013567ffffffffffffffff811115610ca157600080fd5b610cad86828701610a39565b9497909650939450505050565b838152826020820152606060408201526000610cd96060830184610b32565b95945050505050565b600060208284031215610cf457600080fd5b813573ffffffffffffffffffffffffffffffffffffffff81168114610bd157600080fd5b600060208284031215610d2a57600080fd5b5035919050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052604160045260246000fd5b7f4e487b7100000000000000000000000000000000000000000000000000000000600052603260045260246000fd5b600082357fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff81833603018112610dc357600080fd5b9190910192915050565b60008083357fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe1843603018112610e0257600080fd5b83018035915067ffffffffffffffff821115610e1d57600080fd5b602001915036819003821315610a7e57600080fd5b8183823760009101908152919050565b600082357fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffc1833603018112610dc357600080fd5b600082357fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffa1833603018112610dc357600080fdfea2646970667358221220bb2b5c71a328032f97c676ae39a1ec2148d3e5d6f73d95e9b17910152d61f16264736f6c634300080c0033");

    async fn install_multicall3(provider: &impl Provider) {
        provider.anvil_set_code(MULTICALL3_ADDRESS, MULTICALL3_DEPLOYED_CODE.into()).await.unwrap();
    }

    /// Spawns anvil with Multicall3 at its canonical address and a deployed [`Token`].
    async fn setup() -> (impl Provider + Clone, Address) {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
        install_multicall3(&provider).await;
        let token = *Token::deploy(&provider).await.unwrap().address();
        (provider, token)
    }

    #[tokio::test]
    async fn test_aggregate() {
        let (provider, token) = setup().await;
        let erc20 = Token::new(token, &provider);

        let multicall = provider
            .multicall()
            .add(erc20.totalSupply())
            .add(erc20.balanceOf(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")))
            .add(erc20.totalSupply())
            .add(erc20.balanceOf(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")));

        let (t1, b1, t2, b2) = multicall.aggregate().await.unwrap();

        assert_eq!(t1, t2);
        assert_eq!(b1, b2);
    }

    #[tokio::test]
    async fn test_try_aggregate_pass() {
        let (provider, token) = setup().await;
        let erc20 = Token::new(token, &provider);

        let multicall = provider
            .multicall()
            .add(erc20.totalSupply())
            .add(erc20.balanceOf(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")))
            .add(erc20.totalSupply())
            .add(erc20.balanceOf(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")));

        let (_t1, _b1, _t2, _b2) = multicall.try_aggregate(true).await.unwrap();
    }

    #[tokio::test]
    async fn aggregate3() {
        let (provider, token) = setup().await;
        let dummy = deploy_dummy(provider.clone()).await;
        let erc20 = Token::new(token, &provider);
        let multicall = provider
            .multicall()
            .add(erc20.totalSupply())
            .add(erc20.balanceOf(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")))
            .add(dummy.fail()); // Failing call that will revert the multicall.

        let err = multicall.aggregate3().await.unwrap_err();

        assert!(err.to_string().contains("Multicall3: call failed"), "{err}");

        let failing_call = dummy.fail().into_call(true);
        let multicall = provider
            .multicall()
            .add(erc20.totalSupply())
            .add(erc20.balanceOf(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")))
            .add_call(failing_call);
        let (t1, b1, failure) = multicall.aggregate3().await.unwrap();

        assert!(t1.is_ok());
        assert!(b1.is_ok());
        let err = failure.unwrap_err();
        assert!(matches!(err, Failure { idx: 2, return_data: _ }));
    }

    #[tokio::test]
    async fn test_try_aggregate_fail() {
        let (provider, token) = setup().await;
        let dummy_addr = deploy_dummy(provider.clone()).await;
        let erc20 = Token::new(token, &provider);
        let multicall = provider
            .multicall()
            .add(erc20.totalSupply())
            .add(erc20.balanceOf(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")))
            .add(erc20.totalSupply())
            .add(erc20.balanceOf(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")))
            .add(dummy_addr.fail());

        let err = multicall.try_aggregate(true).await.unwrap_err();

        assert!(err.to_string().contains("execution reverted: Multicall3: call failed"));

        let (t1, b1, t2, b2, failure) = multicall.try_aggregate(false).await.unwrap();

        assert!(t1.is_ok());
        assert!(b1.is_ok());
        assert!(t2.is_ok());
        assert!(b2.is_ok());
        let err = failure.unwrap_err();
        assert!(matches!(err, Failure { idx: 4, return_data: _ }));
    }

    #[tokio::test]
    async fn test_util() {
        let (provider, token) = setup().await;
        let erc20 = Token::new(token, &provider);
        let genesis_hash =
            provider.get_block_by_number(0.into()).await.unwrap().unwrap().header.hash;
        let multicall = provider
            .multicall()
            .add(erc20.totalSupply())
            .add(erc20.balanceOf(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")))
            .add(erc20.totalSupply())
            .add(erc20.balanceOf(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")))
            .get_block_hash(0);

        let (t1, b1, t2, b2, block_hash) = multicall.aggregate().await.unwrap();

        assert_eq!(t1, t2);
        assert_eq!(b1, b2);
        assert_eq!(block_hash, genesis_hash);
    }

    #[tokio::test]
    async fn uses_provider_default_block() {
        let provider = ProviderBuilder::new().with_default_block(BlockId::latest()).connect_anvil();
        install_multicall3(&provider).await;

        let (block_number,) = provider.multicall().get_block_number().aggregate().await.unwrap();
        assert_eq!(block_number, U256::from(provider.get_block_number().await.unwrap()));
    }

    sol! {
        // solc 0.8.25; solc PayableCounter.sol --optimize --bin
        #[sol(rpc, bytecode = "6080604052348015600e575f80fd5b5061012c8061001c5f395ff3fe6080604052600436106025575f3560e01c806361bc221a146029578063d09de08a14604d575b5f80fd5b3480156033575f80fd5b50603b5f5481565b60405190815260200160405180910390f35b60536055565b005b5f341160bc5760405162461bcd60e51b815260206004820152602c60248201527f50617961626c65436f756e7465723a2076616c7565206d75737420626520677260448201526b06561746572207468616e20360a41b606482015260840160405180910390fd5b60015f8082825460cb919060d2565b9091555050565b8082018082111560f057634e487b7160e01b5f52601160045260245ffd5b9291505056fea264697066735822122064d656316647d3dc48d7ef0466bd10bc87694802a673183058725926a5190a5564736f6c63430008190033")]
        #[derive(Debug)]
        contract PayableCounter {
            uint256 public counter;

            function increment() public payable {
                require(msg.value > 0, "PayableCounter: value must be greater than 0");
                counter += 1;
            }
        }
    }

    #[tokio::test]
    async fn aggregate3_value() {
        let (provider, _) = setup().await;

        let payable_counter = PayableCounter::deploy(provider.clone()).await.unwrap();

        let increment_call = CallItem::<PayableCounter::incrementCall>::new(
            payable_counter.increment().target(),
            payable_counter.increment().input(),
        )
        .value(U256::from(100));

        let multicall = provider
            .multicall()
            .add(payable_counter.counter())
            .add_call(increment_call)
            .add(payable_counter.counter());

        let (c1, inc, c2) = multicall.aggregate3_value().await.unwrap();

        assert_eq!(c1.unwrap(), U256::ZERO);
        assert!(inc.is_ok());
        assert_eq!(c2.unwrap(), U256::from(1));

        // Allow failure - due to no value being sent
        let increment_call = CallItem::<PayableCounter::incrementCall>::new(
            payable_counter.increment().target(),
            payable_counter.increment().input(),
        )
        .allow_failure(true);

        let multicall = provider
            .multicall()
            .add(payable_counter.counter())
            .add_call(increment_call)
            .add(payable_counter.counter());

        let (c1, inc, c2) = multicall.aggregate3_value().await.unwrap();

        assert_eq!(c1.unwrap(), U256::ZERO);
        assert!(inc.is_err_and(|failure| matches!(failure, Failure { idx: 1, return_data: _ })));
        assert_eq!(c2.unwrap(), U256::ZERO);
    }

    #[tokio::test]
    async fn add_dynamic() {
        let (provider, token) = setup().await;
        let erc20 = Token::new(token, &provider);

        let multicall = MulticallBuilder::new_dynamic(provider.clone())
            .add_dynamic(erc20.totalSupply())
            // .add(erc20.balanceOf(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"))) - WON'T
            // COMPILE
            // .add_dynamic(erc20.balanceOf(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"))) -
            // WON'T COMPILE
            .add_dynamic(erc20.totalSupply())
            .extend(vec![erc20.totalSupply(), erc20.totalSupply()]);

        let res = multicall.aggregate().await.unwrap();

        assert_eq!(res.len(), 4);
        assert_eq!(res[0], res[1]);
    }

    #[tokio::test]
    async fn test_add_call_dynamic_fallible() {
        let (provider, _) = setup().await;
        let dummy = deploy_dummy(provider.clone()).await;

        // allow failure
        let multicall = MulticallBuilder::new_dynamic(provider.clone())
            .add_call_dynamic(dummy.fail().into_call(true));
        let res = multicall.aggregate3().await.unwrap();

        assert_eq!(res.len(), 1);
        assert!(matches!(res[0].clone().unwrap_err(), Failure { idx: 0, return_data: _ }));
    }
}
