use crate::{
    fillers::{FillProvider, JoinFill, SignerFiller, TxFiller},
    Provider,
};
use alloy_network::{Ethereum, Network, NetworkSigner};
use alloy_primitives::Address;
use alloy_transport::Transport;

/// Trait for Providers, Fill stacks, etc, which contain [`NetworkSigner`].
pub trait WalletProvider<N: Network = Ethereum> {
    /// The underlying [`NetworkSigner`] type contained in this stack.
    type Signer: NetworkSigner<N>;

    /// Get a reference to the underlying signer.
    fn signer(&self) -> &Self::Signer;

    /// Get a mutable reference to the underlying signer.
    fn signer_mut(&mut self) -> &mut Self::Signer;

    /// Get the default signer address.
    fn default_signer(&self) -> Address {
        self.signer().default_signer()
    }

    /// Check if the signer can sign for the given address.
    fn is_signer_for(&self, address: &Address) -> bool {
        self.signer().is_signer_for(address)
    }

    /// Get an iterator of all signer addresses.
    fn signers(&self) -> impl Iterator<Item = Address> {
        self.signer().signers()
    }
}

impl<S, N> WalletProvider<N> for SignerFiller<S>
where
    S: NetworkSigner<N> + Clone,
    N: Network,
{
    type Signer = S;

    #[inline(always)]
    fn signer(&self) -> &Self::Signer {
        self.as_ref()
    }

    #[inline(always)]
    fn signer_mut(&mut self) -> &mut Self::Signer {
        self.as_mut()
    }
}

impl<L, R, N> WalletProvider<N> for JoinFill<L, R>
where
    R: WalletProvider<N>,
    N: Network,
{
    type Signer = R::Signer;

    #[inline(always)]
    fn signer(&self) -> &Self::Signer {
        self.right().signer()
    }

    #[inline(always)]
    fn signer_mut(&mut self) -> &mut Self::Signer {
        self.right_mut().signer_mut()
    }
}

impl<F, P, T, N> WalletProvider<N> for FillProvider<F, P, T, N>
where
    F: TxFiller<N> + WalletProvider<N>,
    P: Provider<T, N>,
    T: Transport + Clone,
    N: Network,
{
    type Signer = F::Signer;

    #[inline(always)]
    fn signer(&self) -> &Self::Signer {
        self.filler.signer()
    }

    #[inline(always)]
    fn signer_mut(&mut self) -> &mut Self::Signer {
        self.filler.signer_mut()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ProviderBuilder;

    #[test]
    fn basic_usage() {
        let (provider, _anvil) = ProviderBuilder::new().on_anvil_with_signer();

        assert_eq!(provider.default_signer(), provider.signers().next().unwrap());
    }

    #[test]
    fn bubbles_through_fillers() {
        let (provider, _anvil) =
            ProviderBuilder::new().with_recommended_fillers().on_anvil_with_signer();

        assert_eq!(provider.default_signer(), provider.signers().next().unwrap());
    }
}
