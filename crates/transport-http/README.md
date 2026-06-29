# alloy-transport-http

HTTP transport implementation.

## Providing HTTP Headers
The HTTP request headers will be extended if a `http::HeaderMap` is present in the request metadata. This extension functionality is only available for single requests and is not supported for batch requests.