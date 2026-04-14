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

**Requires**: [jq](https://jqlang.github.io/jq/download/) installed on your system.

### Pre-built binaries

Download from [GitHub Releases](https://github.com/vagusX/jq5/releases/latest):

| Platform | File |
|----------|------|
| Linux x86_64 | `jq5-linux-x86_64.tar.gz` |
| Linux aarch64 | `jq5-linux-aarch64.tar.gz` |
| macOS Intel | `jq5-darwin-x86_64.tar.gz` |
| macOS Apple Silicon | `jq5-darwin-aarch64.tar.gz` |
| Windows x86_64 | `jq5-windows-x86_64.zip` |

Quick install (Linux / macOS):

```bash
# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
[ "$ARCH" = "arm64" ] && ARCH="aarch64"

curl -fLO "https://github.com/vagusX/jq5/releases/latest/download/jq5-${OS}-${ARCH}.tar.gz"
tar xzf jq5-${OS}-${ARCH}.tar.gz

# Install to system path (requires sudo), or ~/.local/bin (no sudo)
sudo mv jq5 /usr/local/bin/
# or: mv jq5 ~/.local/bin/
```

Windows (PowerShell):

```powershell
Invoke-WebRequest -Uri "https://github.com/vagusX/jq5/releases/latest/download/jq5-windows-x86_64.zip" -OutFile jq5.zip
Expand-Archive jq5.zip -DestinationPath .
Move-Item jq5.exe C:\Windows\System32\
```

### Build from source

Requires [Rust toolchain](https://rustup.rs/).

```bash
cargo install --path .
```

Or:

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

### Plain JSON output

Use `--json` to skip json5format and output jq's native JSON (preserves original formatting):

```bash
jq5 --json '.' data.json
```

### Pass arguments to jq

Use `--` to pass extra arguments (like `--arg`, `--argjson`, `--slurp`) through to jq:

```bash
jq5 --json '.name = $val' data.json -- --arg val "new_value"
jq5 --json '.count = $n' data.json -- --argjson n 42
```

### Custom jq path

```bash
jq5 --path-to-jq /usr/local/bin/jq '.items[]' data.json5
```

## Differences from Fuchsia original

| Aspect | Fuchsia | Standalone |
|--------|---------|------------|
| Async runtime | `fuchsia_async` | `tokio` + `futures::join_all` |
| Default jq | `fx jq` | System `jq` in PATH |
| Build system | `BUILD.gn` | `Cargo.toml` |
| Type mismatch | Error | Graceful fallback to raw jq output |

## Differences from jq

The filter syntax is identical — jq5 passes filters directly to jq. The differences are in output format:

| | jq | jq5 (default) | jq5 --json |
|---|---|---|---|
| Input format | JSON only | JSON & JSON5 | JSON & JSON5 |
| Output format | JSON | JSON5 | JSON (jq native) |
| Indentation | 2 spaces | 4 spaces | 2 spaces |
| Key quoting | Always quoted | Unquoted when possible | Always quoted |
| Trailing commas | No | Yes | No |
| Comments | N/A | Preserved from input | Discarded |
| `--tab`, `--indent` | Direct flags | Via `-- --tab` | Via `-- --tab` |
| `--arg`, `--argjson` | Direct flags | Via `-- --arg` | Via `-- --arg` |

**When to use which mode:**

- **JSON5 files with comments** → default mode (preserves comments, outputs JSON5)
- **JSON files** → `--json` mode (identical to jq output)

## Known limitations

- Comments may attach to incorrect elements when array positions shift (e.g., after deletion)
- Default mode uses json5format's 4-space indentation (not configurable); use `--json` with `-- --tab` or `-- --indent 4` to control indentation
- Requires `jq` installed externally

> **Note**: Linux binaries are statically linked with musl, no glibc dependency. They should work on any Linux distribution regardless of glibc version.

## License

MIT License. See [LICENSE](LICENSE).

This project includes code from Google Fuchsia under BSD 2-Clause. See [NOTICE](NOTICE).
