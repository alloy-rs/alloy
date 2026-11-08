# alloy-transport-http

HTTP transport implementation.

## Providing HTTP headers

The HTTP request headers are extended when a `http::HeaderMap` is present in
the request metadata. For a batch, header maps are appended in request order;
repeated names retain every value rather than replacing earlier ones. Because a
batch is sent as one HTTP request, headers are packet-wide. Avoid duplicate
authorization or tenant headers, and do not batch calls that require different
values.

## Resource limits

By default, the transport rejects a response body larger than
`alloy_transport::DEFAULT_MAX_RESPONSE_SIZE` (256 MiB), while reading it and
before JSON decoding, so that a malicious or misbehaving endpoint cannot exhaust
the client's memory. The non-JSON-RPC body of an HTTP error is truncated to
4 KiB in the returned error.

The default clients, created by `Http::new` and `HyperClient::new`, also have a
10 second connect timeout and a 120 second read timeout. The read timeout bounds
the wait for the response headers and each pause while the body arrives, but
not the whole request, so slow but steady responses like large traces are not
cut off. These timeouts are not available on WASM.

To change the limits:

- `HttpTransportSettings::with_max_response_size` sets another limit, and
  `HttpTransportSettings::unlimited` removes it. A total deadline per request
  can be set with `HttpTransportSettings::with_request_timeout`.
- A client passed to `Http::with_client` keeps its own timeouts, but the
  transport still has the default response size limit.
- `ConnectionConfig` in `alloy-rpc-client` exposes the response size limit and
  request deadline for connections made from a connection string.
