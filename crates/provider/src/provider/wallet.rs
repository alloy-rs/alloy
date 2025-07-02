use crate::{
    fillers::{FillProvider, JoinFill, TxFiller, WalletFiller},
    Provider,
};
use alloy_network::{Ethereum, Network, NetworkWallet};
use alloy_primitives::Address;

/// Trait for Providers, Fill stacks, etc, which contain [`NetworkWallet`].
pub trait WalletProvider<N: Network = Ethereum> {
    /// The underlying [`NetworkWallet`] type contained in this stack.
    type Wallet: NetworkWallet<N>;

    /// Get a reference to the underlying wallet.
    fn wallet(&self) -> &Self::Wallet;

    /// Get a mutable reference to the underlying wallet.
    fn wallet_mut(&mut self) -> &mut Self::Wallet;

    /// Get the default signer address.
    fn default_signer_address(&self) -> Address {
        self.wallet().default_signer_address()
    }

    /// Check if the signer can sign for the given address.
    fn has_signer_for(&self, address: &Address) -> bool {
        self.wallet().has_signer_for(address)
    }

    /// Get an iterator of all signer addresses. Note that because the signer
    /// always has at least one address, this iterator will always have at least
    /// one element.
    fn signer_addresses(&self) -> impl Iterator<Item = Address> {
        self.wallet().signer_addresses()
    }
}

impl<W, N> WalletProvider<N> for WalletFiller<W>
where
    W: NetworkWallet<N> + Clone,
    N: Network,
{
    type Wallet = W;

    #[inline(always)]
    fn wallet(&self) -> &Self::Wallet {
        self.as_ref()
    }

    #[inline(always)]
    fn wallet_mut(&mut self) -> &mut Self::Wallet {
        self.as_mut()
    }
}

impl<L, R, N> WalletProvider<N> for JoinFill<L, R>
where
    R: WalletProvider<N>,
    N: Network,
{
    type Wallet = R::Wallet;

    #[inline(always)]
    fn wallet(&self) -> &Self::Wallet {
        self.right().wallet()
    }

    #[inline(always)]
    fn wallet_mut(&mut self) -> &mut Self::Wallet {
        self.right_mut().wallet_mut()
    }
}

impl<F, P, N> WalletProvider<N> for FillProvider<F, P, N>
where
    F: TxFiller<N> + WalletProvider<N>,
    P: Provider<N>,
    N: Network,
{
    type Wallet = F::Wallet;

    #[inline(always)]
    fn wallet(&self) -> &Self::Wallet {
        self.filler.wallet()
    }

    #[inline(always)]
    fn wallet_mut(&mut self) -> &mut Self::Wallet {
        self.filler.wallet_mut()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ProviderBuilder;
    use alloy_primitives::{address, U256};
    use alloy_sol_types::{sol, Eip712Domain};
    use itertools::Itertools;

    #[test]
    fn basic_usage() {
        let provider =
            ProviderBuilder::new().disable_recommended_fillers().connect_anvil_with_wallet();

        assert!(provider.signer_addresses().contains(&provider.default_signer_address()));
    }

    #[test]
    fn bubbles_through_fillers() {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

        assert!(provider.signer_addresses().contains(&provider.default_signer_address()));
    }

    #[tokio::test]
    async fn sign_hash() {
        sol! {
            struct Test {
                uint256 value;
            }
        }
        use alloy_sol_types::SolStruct;
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();

        let t = Test { value: U256::from(0x42) };
        let domain = Eip712Domain::default();
        let hash = t.eip712_signing_hash(&domain);
        let signer = address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let _ = provider.wallet().sign_hash_with(signer, &hash).await.unwrap();
    }
}
