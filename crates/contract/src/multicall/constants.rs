use alloy_chains::NamedChain;
use alloy_primitives::{address, Address};

/// The Multicall3 contract address that is deployed to each [`MULTICALL_SUPPORTED_CHAINS`]:
/// [`0xcA11bde05977b3631167028862bE2a173976CA11`](https://etherscan.io/address/0xcA11bde05977b3631167028862bE2a173976CA11)
pub const MULTICALL_ADDRESS: Address = address!("cA11bde05977b3631167028862bE2a173976CA11");

/// The chain IDs that Multicall3 has been deployed to at [`MULTICALL_ADDRESS`].
///
/// Taken from <https://www.multicall3.com/deployments>
pub const MULTICALL_SUPPORTED_CHAINS: &[u64] = {
    use NamedChain::*;
    &[
        Mainnet as u64,                  // Mainnet
        Kovan as u64,                    // Kovan
        Rinkeby as u64,                  // Rinkeby
        Goerli as u64,                   // Görli
        Ropsten as u64,                  // Ropsten
        Sepolia as u64,                  // Sepolia
        Holesky as u64,                  // Holesky
        Optimism as u64,                 // Optimism
        OptimismKovan as u64,            // Optimism Kovan
        OptimismGoerli as u64,           // Optimism Görli
        OptimismSepolia as u64,          // Optimism Sepolia
        Arbitrum as u64,                 // Arbitrum
        ArbitrumNova as u64,             // Arbitrum Nova
        ArbitrumGoerli as u64,           // Arbitrum Görli
        ArbitrumSepolia as u64,          // Arbitrum Sepolia
        ArbitrumTestnet as u64,          // Arbitrum Rinkeby
        23011913,                        // Stylus Testnet
        Polygon as u64,                  // Polygon
        PolygonMumbai as u64,            // Polygon Mumbai
        PolygonZkEvm as u64,             // Polygon zkEVM
        PolygonZkEvmTestnet as u64,      // Polygon zkEVM Testnet
        Gnosis as u64,                   // Gnosis (xDai) Chain
        Chiado as u64,                   // Gnosis Chain Testnet
        Avalanche as u64,                // Avalanche
        AvalancheFuji as u64,            // Avalanche Fuji
        FantomTestnet as u64,            // Fantom Testnet
        Fantom as u64,                   // Fantom Opera
        64240,                           // Fantom Sonic
        BinanceSmartChain as u64,        // BNB Smart Chain
        BinanceSmartChainTestnet as u64, // BNB Smart Chain Testnet
        5611,                            // opBNB Testnet
        204,                             // opBNB
        Moonbeam as u64,                 // Moonbeam
        Moonriver as u64,                // Moonriver
        Moonbase as u64,                 // Moonbase Alpha Testnet
        11297108109,                     // Palm
        11297108099,                     // Palm Testnet
        1666600000,                      // Harmony
        Cronos as u64,                   // Cronos
        CronosTestnet as u64,            // Cronos Testnet
        122,                             // Fuse
        14,                              // Flare Mainnet
        19,                              // Songbird Canary Network
        16,                              // Coston Testnet
        114,                             // Coston2 Testnet
        Boba as u64,                     // Boba
        Aurora as u64,                   // Aurora
        592,                             // Astar
        6038361,                         // Astar zKyoto Testnet
        3776,                            // Astar zkEVM
        66,                              // OKC
        128,                             // Heco Chain
        Metis as u64,                    // Metis
        599,                             // Metis Goerli
        Rsk as u64,                      // Rsk
        31,                              // Rsk Testnet
        Evmos as u64,                    // Evmos
        EvmosTestnet as u64,             // Evmos Testnet
        108,                             // Thundercore
        18,                              // Thundercore Testnet
        Oasis as u64,                    // Oasis
        23294,                           // Oasis Sapphire
        Celo as u64,                     // Celo
        CeloAlfajores as u64,            // Celo Alfajores Testnet
        71402,                           // Godwoken
        71401,                           // Godwoken Testnet
        8217,                            // Klaytn
        1001,                            // Klaytn Testnet (Baobab)
        2001,                            // Milkomeda
        321,                             // KCC
        106,                             // Velas
        40,                              // Telos
        1234,                            // Step Network
        Canto as u64,                    // Canto
        CantoTestnet as u64,             // Canto Testnet
        4689,                            // Iotex
        32520,                           // Bitgert
        2222,                            // Kava
        5003,                            // Mantle Sepolia
        MantleTestnet as u64,            // Mantle Testnet
        Mantle as u64,                   // Mantle
        8082,                            // Shardeum Sphinx
        BaseGoerli as u64,               // Base Görli
        BaseSepolia as u64,              // Base Sepolia
        Base as u64,                     // Base
        2358,                            // Kroma Testnet (Sepolia)
        1130,                            // DeFiChain EVM Mainnet
        1131,                            // DeFiChain EVM Testnet
        335,                             // DFK Chain Testnet
        53935,                           // DFK Chain
        // TODO - add remaining chains
        1131,                // DeFiChain EVM Testnet
        BlastSepolia as u64, // Blast Sepolia
        Mode as u64,         // Mode Mainnet
        #[cfg(test)]
        31337, // Anvil
    ]
};
