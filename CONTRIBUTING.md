# Contributing to Fang

## Development setup — no Razer hardware needed

Everything runs against a simulated Blade 18 on any OS (Linux, Windows, macOS):

```sh
# terminal 1 — daemon with simulated hardware
cargo run -p fangd -- --mock --tcp 127.0.0.1:7331

# terminal 2 — the app
cd app && npm install && npm run tauri dev
```

UI-only (browser simulator, no daemon or Rust toolchain):

```sh
cd app && npm run dev     # http://localhost:1420, screens deep-link via #fan etc.
```

## Tests

```sh
cargo test --workspace                       # protocol + daemon (incl. e2e mock test)
cd app/src-tauri && cargo test --bin fang    # xrandr/kscreen/colormgr parsers
```

CI also enforces `cargo fmt` and `cargo clippy -- -D warnings`.

Before cutting a release, run `node app/scripts/version.mjs set X.Y.Z`, update
the newest CHANGELOG entry, then run `node app/scripts/version.mjs check`.
CI rejects mismatched Cargo, npm, Tauri, lockfile or changelog versions.

## Adding support for your Blade

1. Run through [HARDWARE_TESTING.md](HARDWARE_TESTING.md) on your machine.
2. Add one entry to `crates/fang-protocol/src/models.rs` with your USB PID and
   fan limits (crosscheck razer-laptop-control's `laptops.json` for the
   limits Razer uses on your model).
3. Open a PR including your `lsusb -d 1532:` output and a short journal
   snippet showing the daemon driving the device — or just file a
   "Laptop model report" issue with the same info and we'll do the rest.

## EC protocol changes

Anything touching `fang-protocol::packet` needs a unit test asserting the
exact wire bytes (see the existing tests), and a pointer to where the bytes
were verified (razer-laptop-control / OpenRazer source, or a USB capture).

## License

GPL-2.0 — contributions are accepted under the same license.
