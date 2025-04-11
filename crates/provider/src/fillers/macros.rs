use crate::{
    fillers::{FillerControlFlow, Fillers, TuplePush, TxFiller},
    provider::Provider,
    Identity, SendableTx, WalletProvider,
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
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12, 12 => T13);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12, 12 => T13, 13 => T14);
impl_tx_filler!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12, 12 => T13, 13 => T14, 14 => T15);

impl_from!(0 => T1);
impl_from!(0 => T1, 1 => T2);
impl_from!(0 => T1, 1 => T2, 2 => T3);
impl_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4);
impl_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5);
impl_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6);
impl_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7);
impl_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8);
impl_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9);
impl_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10);
impl_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11);
impl_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12);
impl_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12, 12 => T13);
impl_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12, 12 => T13, 13 => T14);
impl_from!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12, 12 => T13, 13 => T14, 14 => T15);

impl_tuple!(0 => T1);
impl_tuple!(0 => T1, 1 => T2);
impl_tuple!(0 => T1, 1 => T2, 2 => T3);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12, 12 => T13);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12, 12 => T13, 13 => T14);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12, 12 => T13, 13 => T14, 14 => T15);

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
impl_wallet_provider_at!(9 => T0, T1, T2, T3, T4, T5, T6, T7, T8);
impl_wallet_provider_at!(10 => T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_wallet_provider_at!(11 => T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_wallet_provider_at!(12 => T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_wallet_provider_at!(13 => T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_wallet_provider_at!(14 => T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_wallet_provider_at!(15 => T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);

/// Macro to implment [`std::fmt::Debug`] for tuples of different sizes
///
/// This is because rust only allows deriving `Debug` for tuples of size upto 12.
///
/// See: <https://doc.rust-lang.org/std/primitive.tuple.html#trait-implementations-1>
macro_rules! impl_debug {
    ($($idx:tt => $ty:ident),+) => {
        impl<$($ty: std::fmt::Debug,)+ N: Network> std::fmt::Debug for Fillers<($($ty,)+), N> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple("Fillers")
                    $(.field(&self.inner().$idx))+
                    .finish()
            }
        }
    };
}

impl_debug!(0 => T1);
impl_debug!(0 => T1, 1 => T2);
impl_debug!(0 => T1, 1 => T2, 2 => T3);
impl_debug!(0 => T1, 1 => T2, 2 => T3, 3 => T4);
impl_debug!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5);
impl_debug!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6);
impl_debug!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7);
impl_debug!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8);
impl_debug!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9);
impl_debug!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10);
impl_debug!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11);
impl_debug!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12);
impl_debug!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12, 12 => T13);
impl_debug!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12, 12 => T13, 13 => T14);
impl_debug!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12, 12 => T13, 13 => T14, 14 => T15);
