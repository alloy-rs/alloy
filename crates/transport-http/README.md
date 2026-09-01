# alloy-transport-http

HTTP transport implementation.

## Providing HTTP headers

The HTTP request headers are extended when a `http::HeaderMap` is present in
the request metadata. For a batch, header maps are appended in request order;
repeated names retain every value rather than replacing earlier ones. Because a
batch is sent as one HTTP request, headers are packet-wide. Avoid duplicate
authorization or tenant headers, and do not batch calls that require different
values.
