# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial implementation of `TurnkeySigner` for Alloy following AWS/GCP signer patterns
- Synchronous constructor accepting Turnkey client, organization ID, and address
- Convenience constructor `from_api_key()` for simplified initialization
- Hash-only signing via Turnkey's `sign_raw_payload` API with `HASH_FUNCTION_NO_OP`
- Complete trait implementations: `Signer` (hash-based with auto-impls), `TxSigner`
- Comprehensive error handling with `TurnkeySignerError` type
- Environment-gated integration tests (`sign_hash`, `sign_message`, `signer_properties`)
