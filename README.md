# SMTP STRIPTLS proxy demo
> demo for a smtp STRIPTLS attack, runs as smtp proxy server, written in rust

## Getting started

```sh
cargo install --path .
```
> NOTE: checkout the `--help` options for all features

```sh
RUST_LOG=debug smtp-proxy --server some.smtp-server.com
```

## Further reading

As a reference I suggest  
["Understanding how tls downgrade attacks prevent email encryption"](https://elie.net/blog/understanding-how-tls-downgrade-attacks-prevent-email-encryption/)
in order to understand the idea behind this program.
