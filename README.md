`imtui-rs`
==========

`imtui-rs` is a rust crate that provides safe rust bindings for [`imtui`](https://github.com/ggerganov/imtui).

# Screenshots

Here is a screenshot of a rust port of the `hnterm` example from `imtui`:

[![imtui-rs-hnterm-demo](https://asciinema.org/a/3qbgy8bHhK9oVhmJWlUAUER3p.svg)](https://asciinema.org/a/3qbgy8bHhK9oVhmJWlUAUER3p)

# Build `imtui-rs`

```bash
git clone --recursive https://github.com/visigoth/imtui-rs
cd imtui-rs
cargo build
```

# Build and Run `hnterm`

This example illustrates combining `imtui-rs` with `tokio` to create a single threaded asynchronous terminal app with an interactive UI.

```bash
cargo run --example hnterm
```

## Debugging `hnterm`

```bash
cargo run --example hnterm -- -d
```
