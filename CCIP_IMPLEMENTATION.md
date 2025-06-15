# CCIP (EIP-3668) Implementation Summary

This document summarizes the CCIP implementation added to the Alloy library.

## Overview

Cross Chain Interoperability Protocol (CCIP) as defined in EIP-3668 has been implemented, allowing smart contracts to fetch external data securely and transparently. The implementation provides both automatic and manual handling of OffchainLookup errors.

## Key Components

### 1. Core Types

- **`OffchainLookup`**: The Solidity error type decoded from contract reverts
- **`CcipCall`**: Builder for CCIP-enabled calls with configuration options
- **`CcipOffchainLookup`**: Handler for resolving offchain lookups
- **`GatewayClient`**: Trait for custom gateway implementations
- **`DefaultGatewayClient`**: Default HTTP gateway client using reqwest

### 2. Provider Extension

The `ProviderCcipExt` trait extends all providers with CCIP support:

```rust
provider.call_with_ccip(&tx)
    .max_recursion(2)
    .gateway_timeout(Duration::from_secs(10))
    .await?
```

### 3. Contract Integration

Contract calls can enable CCIP support via the `ccip()` method:

```rust
contract.my_method()
    .ccip()
    .await?
```

## Features

- **Automatic OffchainLookup handling**: Transparent resolution of CCIP redirects
- **Configurable recursion limits**: Default 4, per EIP-3668 spec
- **Custom gateway clients**: Pluggable gateway implementations
- **Timeout configuration**: Configurable gateway request timeouts
- **Manual handling**: Advanced API for intercepting OffchainLookup errors
- **Provider specialization**: Providers that implement GatewayClient can reuse their transport

## Error Handling

The implementation provides comprehensive error types:
- `CcipError`: Main error type with transport, gateway, and recursion errors
- `GatewayError`: Specific errors for gateway requests

## Example Usage

### Basic Usage
```rust
let result = provider
    .call_with_ccip(&tx)
    .await?;
```

### Advanced Configuration
```rust
let result = provider
    .call_with_ccip(&tx)
    .block(BlockId::latest())
    .max_recursion(2)
    .gateway_timeout(Duration::from_secs(10))
    .with_gateway_client(custom_client)
    .await?;
```

### Manual Handling
```rust
match call.try_call().await {
    Ok(data) => // Direct success
    Err(Ok(offchain_lookup)) => {
        // Access intermediate state
        println!("URLs: {:?}", offchain_lookup.urls());
        // Resolve manually
        offchain_lookup.resolve().await
    }
    Err(Err(e)) => // Other error
}
```

## Implementation Details

- Uses `sol!` macro for OffchainLookup error decoding
- Implements proper URL substitution ({sender}, {data})
- Supports both GET and POST requests based on URL length
- Validates gateway responses
- Handles recursive OffchainLookup errors
- Thread-safe via Arc<dyn GatewayClient>

## Feature Flags

The CCIP functionality requires the `reqwest` feature to be enabled for the default HTTP gateway client.