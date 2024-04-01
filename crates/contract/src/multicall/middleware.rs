use crate::{
    contract::IMulticall3::{self, Call3, IMulticall3Instance},
    CallDecoder, DynCallBuilder,
};
use alloy_dyn_abi::DynSolValue;
use alloy_json_abi::Function;
use alloy_primitives::{Address, Bytes};
use alloy_provider::Provider;

use std::result::Result as StdResult;

/// Alias for [std::result::Result]<T, [MulticallError]>
pub type Result<T> = std::result::Result<T, MulticallError>;

use crate::constants;

use super::error::MulticallError;
use IMulticall3::Result as MulticallResult;

/// An individual call within a multicall
#[derive(Debug, Clone)]
pub struct Call {
    /// The target
    target: Address,
    /// The calldata
    calldata: Bytes,

    /// Whether the call is allowed to fail
    allow_failure: bool,
    /// The decoder
    decoder: Function,
}

/// An abstraction for sending batched calls.
#[derive(Debug, Clone)]
pub struct Multicall<N, T, P>
where
    N: crate::private::Network,
    T: crate::private::Transport + Clone,
    P: Provider<N, T>,
{
    /// The internal calls vector
    calls: Vec<Call>,
    /// The Multicall3 contract
    contract: IMulticall3Instance<N, T, P>,
}

impl<N, T, P> Multicall<N, T, P>
where
    N: crate::private::Network,
    T: crate::private::Transport + Clone,
    P: Provider<N, T>,
{
    /// Creates a new instance of [Multicall]
    pub async fn new(provider: P, address: Option<Address>) -> Result<Self> {
        // If an address is provided by the user, we'll use this.
        // Otherwise fetch the chain ID to confirm it's supported.
        let address = match address {
            Some(address) => address,
            None => {
                let chain_id = provider
                    .get_chain_id()
                    .await
                    .map_err(MulticallError::TransportError)?
                    .to::<u64>();

                if !constants::MULTICALL_SUPPORTED_CHAINS.contains(&chain_id) {
                    return Err(MulticallError::InvalidChainId(chain_id));
                }
                constants::MULTICALL_ADDRESS
            }
        };

        // Create the multicall contract
        let contract = IMulticall3::new(address, provider);

        Ok(Self { calls: vec![], contract })
    }

    /// Adds a [Call] to the internal calls vector
    pub fn add_call(&mut self, call: DynCallBuilder<N, T, P>, allow_failure: bool) {
        let call = Call {
            allow_failure,
            target: call.get_to(),
            calldata: call.calldata().clone(),
            decoder: call.get_decoder(),
        };

        self.calls.push(call)
    }

    /// Builder pattern to add a [Call] to the internal calls vector and return the [Multicall]
    pub fn with_call(&mut self, call: DynCallBuilder<N, T, P>, allow_failure: bool) -> &Self {
        let call = Call {
            allow_failure,
            target: call.get_to(),
            calldata: call.calldata().clone(),
            decoder: call.get_decoder(),
        };

        self.calls.push(call);

        self
    }

    /// Queries the multicall contract via `eth_call` and returns the decoded result
    ///
    /// Returns a vector of Result<DynSolValue, Bytes> for each internal call:
    /// `Err(Bytes)` if the individual call failed whilst `allowFailure` was true, or if the return
    /// data was empty.
    /// `Ok(DynSolValue)` if the call was successful.
    ///
    /// # Errors
    ///
    /// Returns a [MulticallError] if the Multicall call failed.
    pub async fn call(&self) -> Result<Vec<StdResult<DynSolValue, Bytes>>> {
        let calls = self
            .calls
            .clone()
            .into_iter()
            .map(|call| Call3 {
                target: call.target,
                callData: call.calldata,
                allowFailure: call.allow_failure,
            })
            .collect();

        let multicall_result = self.contract.aggregate3(calls).call().await?;

        self.parse_multicall_result(multicall_result.returnData)
    }

    /// Decodes the return data for each individual call result within a multicall.
    fn parse_multicall_result(
        &self,
        return_data: Vec<MulticallResult>,
    ) -> Result<Vec<StdResult<DynSolValue, Bytes>>> {
        let iter = return_data.into_iter();

        let mut results = Vec::with_capacity(self.calls.len());

        for (call, MulticallResult { success, returnData }) in self.calls.iter().zip(iter) {
            // TODO - should empty return data also be considered a call failure, and returns an
            // error when allow_failure = false?

            let result = if !success {
                if !call.allow_failure {
                    return Err(MulticallError::FailedCall);
                }

                Err(returnData)
            } else {
                let decoded = call
                    .decoder
                    .abi_decode_output(returnData, false)
                    .map_err(MulticallError::ContractError);

                if let Err(err) = decoded {
                    // TODO should we return out of the function here, or assign empty bytes to
                    // result? Linked to above TODO

                    return Err(err);
                } else {
                    let mut decoded = decoded.unwrap();

                    // Return the single `DynSolValue` if there's only 1 return value, otherwise
                    // return a tuple of `DynSolValue` elements
                    Ok(if decoded.len() == 1 {
                        decoded.pop().unwrap()
                    } else {
                        DynSolValue::Tuple(decoded)
                    })
                }
            };

            results.push(result);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {

    use alloy_primitives::address;
    use alloy_sol_types::sol;
    use test_utils::{spawn_anvil, spawn_anvil_fork};

    use crate::{ContractInstance, Interface};

    use super::*;

    sol! {
        #[derive(Debug, PartialEq)]
        #[sol(rpc, abi, extra_methods)]
        interface ERC20 {
            function totalSupply() external view returns (uint256 totalSupply);
            function balanceOf(address owner) external view returns (uint256 balance);
            function name() external view returns (string memory);
            function symbol() external view returns (string memory);
            function decimals() external view returns (uint8);
        }
    }

    #[tokio::test]
    async fn test_create_multicall() {
        let (provider, _anvil) = spawn_anvil();

        // New Multicall with default address 0xcA11bde05977b3631167028862bE2a173976CA11
        let multicall = Multicall::new(&provider, None).await.unwrap();
        assert_eq!(multicall.contract.address(), &constants::MULTICALL_ADDRESS);

        // New Multicall with user provided address
        let multicall_address = Address::ZERO;
        let multicall = Multicall::new(&provider, Some(multicall_address)).await.unwrap();
        assert_eq!(multicall.contract.address(), &multicall_address);
    }

    #[tokio::test]
    async fn test_multicall_weth() {
        let (provider, _anvil) = spawn_anvil_fork("https://rpc.ankr.com/eth");
        let weth_address = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");

        // Create the multicall instance
        let mut multicall = Multicall::new(&provider, None).await.unwrap();

        // Generate the WETH ERC20 instance we'll be using to create the individual calls
        let abi = ERC20::abi::contract();
        let weth_contract =
            ContractInstance::new(weth_address, provider.clone(), Interface::new(abi));

        // Create the individual calls
        let total_supply_call = weth_contract.function("totalSupply", &[]).unwrap();
        let name_call = weth_contract.function("name", &[]).unwrap();
        let decimals_call = weth_contract.function("decimals", &[]).unwrap();
        let symbol_call = weth_contract.function("symbol", &[]).unwrap();

        // Add the calls
        multicall.add_call(total_supply_call, true);
        multicall.add_call(name_call, true);
        multicall.add_call(decimals_call, true);
        multicall.add_call(symbol_call, true);

        // Send and await the multicall results
        let results = multicall.call().await.unwrap();

        // Get the expected individual results.
        let name_result = results.get(1).unwrap().as_ref();
        let decimals_result = results.get(2).unwrap().as_ref();
        let symbol_result = results.get(3).unwrap().as_ref();

        let name = name_result.unwrap().as_str().unwrap();
        let symbol = symbol_result.unwrap().as_str().unwrap();
        let decimals = decimals_result.unwrap().as_uint().unwrap().0.to::<u8>();

        // Assert the returned results are as expected
        assert_eq!(name, "Wrapped Ether");
        assert_eq!(symbol, "WETH");
        assert_eq!(decimals, 18);
    }
}
