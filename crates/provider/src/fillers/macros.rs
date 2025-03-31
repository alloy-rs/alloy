use crate::{
    fillers::{FillProvider, FillerControlFlow, FillerNetwork, Fillers, TuplePush, TxFiller},
    provider::Provider,
    Identity, ProviderLayer, SendableTx, WalletProvider,
};
use alloy_network::Network;
use alloy_transport::TransportResult;
use futures::try_join;

/// Macro to implement [`TxFiller`] for tuples of different sizes
macro_rules! impl_tx_filler {
    ($($idx:tt => $ty:ident),+) => {
        impl<$($ty: TxFiller<N>,)+ N: Network> TxFiller<N> for Fillers<($($ty,)+), N> {
            type Fillable = ($(Option<$ty::Fillable>,)+);

            fn status(&self, tx: &N::TransactionRequest) -> FillerControlFlow {
                let mut flow = FillerControlFlow::Finished;
                $(
                    flow = flow.absorb($ty::status(&self.inner().$idx, tx));
                )+
                flow
            }

            fn fill_sync(&self, tx: &mut SendableTx<N>) {
                $($ty::fill_sync(&self.inner().$idx, tx);)+
            }

            async fn prepare<P>(
                &self,
                provider: &P,
                tx: &N::TransactionRequest,
            ) -> TransportResult<Self::Fillable>
            where
                P: Provider<N>,
            {
                try_join!(
                    $(
                        async {
                            if $ty::ready(&self.inner().$idx, tx) {
                                $ty::prepare(&self.inner().$idx, provider, tx).await.map(Some)
                            } else {
                                Ok(None)
                            }
                        },
                    )+
                )
            }

            async fn fill(
                &self,
                to_fill: Self::Fillable,
                mut tx: SendableTx<N>,
            ) -> TransportResult<SendableTx<N>> {
                $(
                    if let Some(to_fill) = to_fill.$idx {
                        tx = $ty::fill(&self.inner().$idx, to_fill, tx).await?;
                    }
                )+
                Ok(tx)
            }

            async fn prepare_call(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
                $($ty::prepare_call(&self.inner().$idx, tx).await?;)+
                Ok(())
            }

            fn prepare_call_sync(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
                $($ty::prepare_call_sync(&self.inner().$idx, tx)?;)+
                Ok(())
            }
        }
    };
}

/// Macro to implement [`ProviderLayer`] for tuples of different sizes
macro_rules! impl_provider_layer {
    ($($idx:tt => $ty:ident),+) => {
        impl<$($ty: TxFiller<N>,)+ P: Provider<N>, N: Network> ProviderLayer<P, N> for Fillers<($($ty,)+), N> {
            type Provider = FillProvider<Self, P, N>;
            fn layer(&self, inner: P) -> Self::Provider {
                FillProvider::new(inner, self.clone())
            }
        }
    };
}

/// Macro to implement FillerNetwork for tuples of different sizes
///
/// This helps change the network associated with the [`Fillers`] stack.
macro_rules! impl_filler_network {
    ($($idx:tt => $ty:ident),+) => {
        impl<$($ty,)+ N: Network> FillerNetwork<N> for Fillers<($($ty,)+), N> {
            type CurrentFillers = ($($ty,)+);

            fn network<Net: Network>(self) -> Fillers<($($ty,)+), Net> {
                self.network::<Net>()
            }
        }
    };
}

/// Macro to implement [`TuplePush`] functionality for tuples of different sizes
macro_rules! impl_tuple {
    ($($idx:tt => $ty:ident),+) => {
        impl<T: TxFiller<N>, $($ty: TxFiller<N>,)+ N: Network> TuplePush<T, N> for ($($ty,)+) {
            type Pushed = ($($ty,)+ T,);
        }
    };
}

/// Macro to implement [`From`] for [`Fillers`] of different sizes
///
/// This is useful in [`Fillers::push`]
///
/// Implements the following
///
/// ```ignore
/// impl<T: TxFiller<N>, N: Network> From<((T1, T2), T)> for Fillers<(T1, T2, T), N> // `T` is the new incoming filler being added to the tuple
/// impl<T: TxFiller<N>, N: Network> From<((T1, T2, T3), T)> for Fillers<(T1, T2, T3, T), N>
/// ```
macro_rules! impl_from {
    ($($idx:tt => $ty:ident),+) => {
        impl<$($ty: TxFiller<N>,)+ T: TxFiller<N>, N: Network> From<(($($ty,)+), T)> for Fillers<($($ty,)+ T,), N> {
            fn from((tuple, t): (($($ty,)+), T)) -> Self {
                Fillers::new(($(tuple.$idx,)+ t))
            }
        }
    };
}

// Special cases
impl<T: TxFiller<N>, N: Network> From<(Identity, T)> for Fillers<(T,), N> {
    fn from((_, t): (Identity, T)) -> Self {
        Self::new((t,))
    }
}

impl<T: TxFiller<N>, N: Network> From<T> for Fillers<(T,), N> {
    fn from(t: T) -> Self {
        Self::new((t,))
    }
}

impl_tx_filler!(0 => T1);
impl_tx_filler!(0 => T1, 1 => T2);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3, 3 => T4);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8);

impl_provider_layer!(0 => T1);
impl_provider_layer!(0 => T1, 1 => T2);
impl_provider_layer!(0 => T1, 1 => T2, 2 => T3);
impl_provider_layer!(0 => T1, 1 => T2, 2 => T3, 3 => T4);
impl_provider_layer!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5);
impl_provider_layer!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6);
impl_provider_layer!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7);
impl_provider_layer!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8);

impl_filler_network!(0 => T1);
impl_filler_network!(0 => T1, 1 => T2);
impl_filler_network!(0 => T1, 1 => T2, 2 => T3);
impl_filler_network!(0 => T1, 1 => T2, 2 => T3, 3 => T4);
impl_filler_network!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5);
impl_filler_network!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6);
impl_filler_network!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7);
impl_filler_network!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8);

impl_from!(0 => T1);
impl_from!(0 => T1, 1 => T2);
impl_from!(0 => T1, 1 => T2, 2 => T3);
impl_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4);
impl_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5);
impl_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6);
impl_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7);
impl_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8);

impl_tuple!(0 => T1);
impl_tuple!(0 => T1, 1 => T2);
impl_tuple!(0 => T1, 1 => T2, 2 => T3);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8);

/// Implement [`WalletProvider`] for [`Fillers`] where the last (idx) element is
/// a [`WalletProvider`].
macro_rules! impl_wallet_provider_at {
    ($idx:tt => $($other:ident),*) => {
        impl<$($other,)* W, N> WalletProvider<N>
            for ($($other,)* W,)
        where
            W: WalletProvider<N>,
            N: Network,
        {
            type Wallet = W::Wallet;

            #[inline(always)]
            fn wallet(&self) -> &Self::Wallet {
                self.$idx.wallet()
            }

            #[inline(always)]
            fn wallet_mut(&mut self) -> &mut Self::Wallet {
                self.$idx.wallet_mut()
            }
        }

        impl<$($other,)* W, N>
            WalletProvider<N> for Fillers<($($other,)* W,), N>
        where
            W: WalletProvider<N>,
            N: Network,
        {
            type Wallet = W::Wallet;

            #[inline(always)]
            fn wallet(&self) -> &Self::Wallet {
                self.inner().wallet()
            }

            #[inline(always)]
            fn wallet_mut(&mut self) -> &mut Self::Wallet {
                self.inner_mut().wallet_mut()
            }
        }
    };
}

impl_wallet_provider_at!(0 => ); // (W,)
impl_wallet_provider_at!(1 => T0); // (T0, W)
impl_wallet_provider_at!(2 => T0, T1); // (T0, T1, W)
impl_wallet_provider_at!(3 => T0, T1, T2); // (T0, T1, T2, W)
impl_wallet_provider_at!(4 => T0, T1, T2, T3); // (T0, T1, T2, T3, W)
impl_wallet_provider_at!(5 => T0, T1, T2, T3, T4);
impl_wallet_provider_at!(6 => T0, T1, T2, T3, T4, T5);
impl_wallet_provider_at!(7 => T0, T1, T2, T3, T4, T5, T6);
impl_wallet_provider_at!(8 => T0, T1, T2, T3, T4, T5, T6, T7);
