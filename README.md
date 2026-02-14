# Lune but with more stuff

- Patched standalone binaries
- Tauri support

This is likely never going upstream, as these features are
implemented via vibecoding, which is strongly discouraged
in the OSS community. Feel free to use cargo install
to install it instead.

## How to build

```bash
# Build the CLI
cargo install --path crates/lune --features std-tauri --force
```
