# 🚀 H123
An experimental HTTP server in Rust that supports HTTP/1.1, HTTP/2, and HTTP/3 over QUIC.

> **Warning**
> This is an experimental project and not intended for production uses.

## 📦 Getting started
### 🐳 Using Docker (recommended)
```shell
docker run \
    -p 127.0.0.1:443:443/tcp \
    -p 127.0.0.1:443:443/udp \
    -v $(pwd)/htdocs:/htdocs \
    ghcr.io/siketyan/h123:latest
```

Easy!

### 🏗 Classic style
1. Clone this repository.
2. Prepare your TLS server certificate, or use the default self-signed one.
3. Run the server with the command:
   ```shell
   cargo run -- -d ./htdocs -b 127.0.0.1:8443 --cert-chain-pem ./cert.pem --private-key-pem ./privkey.pem
   ```
4. Boom! Your server is running.

## 🔌 API
This crate also exposes a Server API to serve your service easily in HTTP/1.1, HTTP/2, and HTTP/3.
To use the API, implement `Service<Request<Bytes>, Response = Response<Bytes>>` and call `Server::new`.

```rust
pub struct Server<S, E> {
    // ...
}

impl<S, E> Server<S, E> {
    pub fn new<A>(config: &ServerConfig, bind_to: A, service: Arc<S>) -> Self
    where
        A: Into<SocketAddr> + Copy;
}
```

## 🔬 Internals
This server implementation is made from these protocol implementations:

- HTTP/1.1 and HTTP/2
  - [hyperium/hyper](https://github.com/hyperium/hyper)
- HTTP/3 over QUIC
  - [hyperium/h3](https://github.com/hyperium/h3)
  - [quinn-rs/quinn](https://github.com/quinn-rs/quinn)

## 📄 Licence
This repository is licenced under the Apache 2.0 Licence.
For details, see [LICENCE.md](./LICENCE.md).

```
Copyright 2022 Naoki Ikeguchi <me@s6n.jp>
```
