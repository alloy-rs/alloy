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
