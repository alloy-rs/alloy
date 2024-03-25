//! Provider-related utilities.
use std::str::FromStr;

use alloy_json_rpc::RpcError;
use alloy_primitives::U256;
use alloy_transport::{Authorization, TransportErrorKind};
use reqwest::Url;
use std::{net::SocketAddr, path::Path};

/// The number of blocks from the past for which the fee rewards are fetched for fee estimation.
pub const EIP1559_FEE_ESTIMATION_PAST_BLOCKS: u64 = 10;
/// Multiplier for the current base fee to estimate max base fee for the next block.
pub const EIP1559_BASE_FEE_MULTIPLIER: f64 = 2.0;
/// The default percentile of gas premiums that are fetched for fee estimation.
pub const EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE: f64 = 20.0;

/// An estimator function for EIP1559 fees.
pub type EstimatorFunction = fn(U256, &[Vec<U256>]) -> Eip1559Estimation;

/// Return type of EIP1155 gas fee estimator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Eip1559Estimation {
    /// The base fee per gas.
    pub max_fee_per_gas: U256,
    /// The max priority fee per gas.
    pub max_priority_fee_per_gas: U256,
}

fn estimate_priority_fee(rewards: &[Vec<U256>]) -> U256 {
    let mut rewards =
        rewards.iter().filter_map(|r| r.first()).filter(|r| **r > U256::ZERO).collect::<Vec<_>>();
    if rewards.is_empty() {
        return U256::ZERO;
    }

    rewards.sort_unstable();

    // Return the median.
    let n = rewards.len();

    if n % 2 == 0 {
        (*rewards[n / 2 - 1] + *rewards[n / 2]) / U256::from(2)
    } else {
        *rewards[n / 2]
    }
}

/// The default EIP-1559 fee estimator which is based on the work by [MetaMask](https://github.com/MetaMask/core/blob/main/packages/gas-fee-controller/src/fetchGasEstimatesViaEthFeeHistory/calculateGasFeeEstimatesForPriorityLevels.ts#L56)
/// (constants for "medium" priority level are used)
pub fn eip1559_default_estimator(
    base_fee_per_gas: U256,
    rewards: &[Vec<U256>],
) -> Eip1559Estimation {
    let max_priority_fee_per_gas = estimate_priority_fee(rewards);
    let potential_max_fee = base_fee_per_gas * U256::from(EIP1559_BASE_FEE_MULTIPLIER);

    Eip1559Estimation { max_fee_per_gas: potential_max_fee, max_priority_fee_per_gas }
}

/// Extracts the authorization information from the given URL.
pub fn extract_auth_info(url: &Url) -> Option<Authorization> {
    if url.has_authority() {
        let username = url.username();
        let pass = url.password().unwrap_or_default();
        Some(Authorization::basic(username, pass))
    } else {
        None
    }
}

/// The built-in transport types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuiltInTransportType {
    /// HTTP transport.
    Http(Url),
    /// WebSocket transport.
    Ws(Url),
    /// IPC transport.
    Ipc(String),
}

impl FromStr for BuiltInTransportType {
    type Err = RpcError<TransportErrorKind>;

    fn from_str(s: &str) -> Result<BuiltInTransportType, Self::Err> {
        if s.parse::<SocketAddr>().is_ok() {
            let s = format!("http://{}", s);
            let url = Url::parse(&s).map_err(TransportErrorKind::custom)?;
            return Ok(BuiltInTransportType::Http(url));
        }
        // Check if s is a path and it exists
        let path = Path::new(&s);
        if path.is_file() {
            // IPC if it exists
            return Ok(BuiltInTransportType::Ipc(s.to_string()));
        }

        // Handle the case where string is "localhost:*"
        if s.starts_with("localhost:") {
            let url = Url::parse(&format!("http://{}", s)).map_err(TransportErrorKind::custom)?;
            return Ok(BuiltInTransportType::Http(url));
        }
        // Parse the URL or return an error
        let url = Url::parse(s).map_err(TransportErrorKind::custom)?;

        if s.starts_with("http://") || s.starts_with("https://") {
            Ok(BuiltInTransportType::Http(url))
        } else if s.starts_with("ws://") || s.starts_with("wss://") {
            Ok(BuiltInTransportType::Ws(url))
        } else {
            Ok(BuiltInTransportType::Ipc(s.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn test_estimate_priority_fee() {
        let rewards = vec![
            vec![U256::from(10_000_000_000_u64)],
            vec![U256::from(200_000_000_000_u64)],
            vec![U256::from(3_000_000_000_u64)],
        ];
        assert_eq!(super::estimate_priority_fee(&rewards), U256::from(10_000_000_000_u64));

        let rewards = vec![
            vec![U256::from(400_000_000_000_u64)],
            vec![U256::from(2_000_000_000_u64)],
            vec![U256::from(5_000_000_000_u64)],
            vec![U256::from(3_000_000_000_u64)],
        ];

        assert_eq!(super::estimate_priority_fee(&rewards), U256::from(4_000_000_000_u64));

        let rewards = vec![vec![U256::from(0)], vec![U256::from(0)], vec![U256::from(0)]];

        assert_eq!(super::estimate_priority_fee(&rewards), U256::from(0));

        assert_eq!(super::estimate_priority_fee(&[]), U256::from(0));
    }

    #[test]
    fn test_eip1559_default_estimator() {
        let base_fee_per_gas = U256::from(1_000_000_000_u64);
        let rewards = vec![
            vec![U256::from(200_000_000_000_u64)],
            vec![U256::from(200_000_000_000_u64)],
            vec![U256::from(300_000_000_000_u64)],
        ];
        assert_eq!(
            super::eip1559_default_estimator(base_fee_per_gas, &rewards),
            Eip1559Estimation {
                max_fee_per_gas: U256::from(2_000_000_000_u64),
                max_priority_fee_per_gas: U256::from(200_000_000_000_u64)
            }
        );
    }

    #[test]
    fn test_parsing_urls() {
        assert_eq!(
            BuiltInTransportType::from_str("http://localhost:8545").unwrap(),
            BuiltInTransportType::Http("http://localhost:8545".parse::<Url>().unwrap())
        );
        assert_eq!(
            BuiltInTransportType::from_str("localhost:8545").unwrap(),
            BuiltInTransportType::Http("http://localhost:8545".parse::<Url>().unwrap())
        );
        assert_eq!(
            BuiltInTransportType::from_str("https://localhost:8545").unwrap(),
            BuiltInTransportType::Http("https://localhost:8545".parse::<Url>().unwrap())
        );
        assert_eq!(
            BuiltInTransportType::from_str("ws://localhost:8545").unwrap(),
            BuiltInTransportType::Ws("ws://localhost:8545".parse::<Url>().unwrap())
        );
        assert_eq!(
            BuiltInTransportType::from_str("wss://localhost:8545").unwrap(),
            BuiltInTransportType::Ws("wss://localhost:8545".parse::<Url>().unwrap())
        );
        assert_eq!(
            BuiltInTransportType::from_str("ipc:///tmp/reth.ipc").unwrap(),
            BuiltInTransportType::Ipc("ipc:///tmp/reth.ipc".to_string())
        );
        assert_eq!(
            BuiltInTransportType::from_str("localhost:8545").unwrap(),
            BuiltInTransportType::Http("http://localhost:8545".parse::<Url>().unwrap())
        );
        assert_eq!(
            BuiltInTransportType::from_str("http://127.0.0.1:8545").unwrap(),
            BuiltInTransportType::Http("http://127.0.0.1:8545".parse::<Url>().unwrap())
        );
        assert_eq!(
            BuiltInTransportType::from_str("ws://127.0.0.1:8545").unwrap(),
            BuiltInTransportType::Ws("ws://127.0.0.1:8545".parse::<Url>().unwrap())
        );
        assert_eq!(
            BuiltInTransportType::from_str("http://localhost").unwrap(),
            BuiltInTransportType::Http("http://localhost".parse::<Url>().unwrap())
        );
        assert_eq!(
            BuiltInTransportType::from_str("127.0.0.1:8545").unwrap(),
            BuiltInTransportType::Http("http://127.0.0.1:8545".parse::<Url>().unwrap())
        );
        assert_eq!(
            BuiltInTransportType::from_str("file:///tmp/reth.ipc").unwrap(),
            BuiltInTransportType::Ipc("file:///tmp/reth.ipc".to_string())
        );

        // Create a temp file and save it.
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_file = temp_dir.path().join("reth.ipc");

        // Save it
        std::fs::write(&temp_file, "reth ipc").unwrap();

        assert_eq!(
            BuiltInTransportType::from_str(temp_file.to_str().unwrap()).unwrap(),
            BuiltInTransportType::Ipc(temp_file.to_str().unwrap().to_string())
        );
        // Delete the written file after test
        std::fs::remove_file(temp_file).unwrap();
        assert_eq!(
            BuiltInTransportType::from_str("http://user:pass@example.com").unwrap(),
            BuiltInTransportType::Http("http://user:pass@example.com".parse::<Url>().unwrap())
        );
    }
}
