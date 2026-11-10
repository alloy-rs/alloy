# alloy-transport-http

HTTP transport implementation.

## Providing HTTP headers

The HTTP request headers are extended when a `http::HeaderMap` is present in
the request metadata. For a batch, header maps are appended in request order;
repeated names retain every value rather than replacing earlier ones. Because a
batch is sent as one HTTP request, headers are packet-wide. Avoid duplicate
authorization or tenant headers, and do not batch calls that require different
values.

## Response size limit

By default, the transport rejects a response body larger than
`alloy_transport::DEFAULT_MAX_RESPONSE_SIZE` (256 MiB) while reading it, before
JSON decoding, so that a malicious or misbehaving endpoint cannot exhaust the
client's memory. Use `HttpTransportSettings::with_max_response_size` to set
another limit, or `HttpTransportSettings::unlimited` to remove it.

## Timeouts

The default clients, created by `Http::new` and `HyperClient::new`, have a 10
second connect timeout and a 120 second read timeout. The read timeout bounds
the wait for the response headers and each pause while the body arrives, but
not the whole request, so slow but steady responses like large traces are not
cut off. These timeouts are not available on WASM.

A client passed to `Http::with_client` keeps its own timeouts. A total deadline
per request can be set with `HttpTransportSettings::with_request_timeout`.
