# jq5

An extension of [jq](https://jqlang.github.io/jq/) to work on JSON5 files, preserving comments.

Based on [Google Fuchsia's jq5](https://fuchsia.googlesource.com/fuchsia/+/refs/heads/main/tools/jq5), re-packaged as a standalone CLI with no Fuchsia ecosystem dependencies.

## How it works

1. Parse JSON5 into a decorated tree (comments attached to AST nodes)
2. Convert to plain JSON via `serde_json5`
3. Pipe through `jq` with the user's filter
4. Parse jq output back into a JSON5 tree
5. Transfer comments from original to output by matching paths
6. Serialize with comments

## Install

**Requires**: [jq](https://jqlang.github.io/jq/download/) and [Rust toolchain](https://rustup.rs/).

```bash
cargo install --path .
```

Or build manually:

```bash
cargo build --release
cp target/release/jq5 /usr/local/bin/
```

## Usage

```
jq5 [OPTIONS] <FILTER> [FILES]...
```

### From stdin

```bash
echo '{
  // Server config
  host: "localhost",
  // Port number
  port: 8080,
}' | jq5 '.'
```

Output (comments preserved):

```json5
{
    // Server config
    host: "localhost",

    // Port number
    port: 8080,
}
```

### From files

```bash
jq5 '.' config.json5 server.json5
```

### Field extraction

```bash
echo '{ name: "Alice", age: 30 }' | jq5 '.name'
# "Alice"
```

### Object restructuring

```bash
echo '{
  // First name
  first: "John",
  // Last name
  last: "Doe",
}' | jq5 '{fullName: .first, family: .last}'
```

### Custom jq path

```bash
jq5 --path-to-jq /usr/local/bin/jq '.items[]' data.json5
```

## Differences from Fuchsia original

| Aspect | Fuchsia | Standalone |
|--------|---------|------------|
| Async runtime | `fuchsia_async` | Synchronous (no async needed) |
| Default jq | `fx jq` | System `jq` in PATH |
| Build system | `BUILD.gn` | `Cargo.toml` |
| Type mismatch | Error | Graceful fallback to raw jq output |

## Known limitations

- Comments may attach to incorrect elements when array positions shift (e.g., after deletion)
- Objects get alphabetically reordered during JSON conversion (doesn't affect data integrity)
- Requires `jq` installed externally

## License

MIT License. See [LICENSE](LICENSE).

This project includes code from Google Fuchsia under BSD 2-Clause. See [NOTICE](NOTICE).
