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
echo '{"name": "test", "value": 1}' | jq5 '.'
```

Output (JSON, same as jq):

```json
{
  "name": "test",
  "value": 1
}
```

### From files

```bash
jq5 '.' config.json data.json5
```

### JSON5 mode (preserve comments)

Use `--json5` to output JSON5 format with comment preservation:

```bash
echo '{
  // Server config
  host: "localhost",
  // Port number
  port: 8080,
}' | jq5 --json5 '.'
```

Output:

```json5
{
  // Server config
  host: "localhost",

  // Port number
  port: 8080,
}
```

### Pass arguments to jq

Use `--` to pass extra arguments (like `--arg`, `--argjson`, `--slurp`) through to jq:

```bash
jq5 '.name = $val' data.json -- --arg val "new_value"
jq5 '.count = $n' data.json -- --argjson n 42
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

The filter syntax is identical — jq5 passes filters directly to jq. The differences are in output format and flag handling:

### Output format

| | jq | jq5 (default) | jq5 --json5 |
|---|---|---|---|
| Input format | JSON only | JSON & JSON5 | JSON & JSON5 |
| Output format | JSON | JSON (same as jq) | JSON5 |
| Indentation | 2 spaces | 2 spaces | 2 spaces |
| Key quoting | Always quoted | Always quoted | Unquoted when possible |
| Trailing commas | No | No | Yes |
| Comments | N/A | Discarded | Preserved from input |

### jq flags support

jq flags are not direct CLI flags of jq5. Pass them after `--`:

```bash
# jq way
jq --arg name "Alice" '.name = $name' file.json

# jq5 way
jq5 '.name = $name' file.json -- --arg name "Alice"
```

| jq flag | jq5 support | Notes |
|---------|-------------|-------|
| `--arg`, `--argjson` | Via `--` | `jq5 '.x = $v' f -- --arg v "val"` |
| `--slurp` / `-s` | Via `--` | `jq5 '.' f1 f2 -- -s` |
| `--raw-output` / `-r` | Via `--` | `jq5 '.name' f -- -r` |
| `--raw-input` / `-R` | Via `--` | `jq5 '.' f -- -R` |
| `--null-input` / `-n` | Via `--` | `jq5 'null' -- -n` |
| `--compact-output` / `-c` | Via `--` | `jq5 '.' f -- -c` |
| `--tab` | Via `--` | `jq5 '.' f -- --tab` |
| `--indent N` | Via `--` | `jq5 '.' f -- --indent 4` |
| `--sort-keys` / `-S` | Via `--` | `jq5 '.' f -- -S` |
| `--exit-status` / `-e` | Not supported | jq5 manages its own exit codes |
| `--jsonargs` | Not supported | Use `--argjson` instead |

**Auto-detection:** jq5 automatically detects if the input is JSON5 by checking:

1. File extension: `.json5` / `.jsonc` → always JSON5 mode
2. Content: if `//` or `/* */` comments are found → JSON5 mode
3. Otherwise → JSON mode (identical to jq)

Use `--json5` to force JSON5 output, even if no comments are detected.

## Known limitations

- Comments may attach to incorrect elements when array positions shift (e.g., after deletion)
- Tab indentation: use `--json` with `-- --tab` to match tab-indented files
- Requires `jq` installed externally

> **Note**: Linux binaries are statically linked with musl, no glibc dependency. They should work on any Linux distribution regardless of glibc version.

## License

MIT License. See [LICENSE](LICENSE).

This project includes code from Google Fuchsia under BSD 2-Clause. See [NOTICE](NOTICE).
