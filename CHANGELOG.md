# 0.4.3 (released on 2020-02-06)

- Automatic Docker tags per git tag
- Lockfile bumps

# 0.4.2 (released on 2019-11-15)

- Never send `transfer-encoding: chunked` when receiving a body.
  Wait for the body to complete and then just send a proper response with `content-length`.

# 0.4.1 (released on 2019-10-09)

- Fix body forwarding behavior to always receive the whole body first.
  This might come back later to bite us with HTTP/2, HTTP/3 or websockets but
  for now gets rid of a problem where `awc` would send both `Content-Length`
  and `Transfer-Encoding` at the same time which is forbidden by
  https://tools.ietf.org/html/rfc7230#section-3.3.2

# 0.4.0 (released on 2019-10-01)

- Add option (`-k`/`--insecure`) to allow connecting to insecure TLS upstreams
- Allow logging to work if not run with an allocated terminal

# 0.3.0 (released on 2019-09-25)

- Add verbose logging option (#2)

# 0.2.0 (released on 2019-09-12)

- First proper release
