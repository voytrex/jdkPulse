# JDK-Pulse Backend CLI

This is the Rust backend for JDK-Pulse. It can be used as a standalone CLI tool for managing JDK selection.

## Usage

### List all installed JDKs

```bash
cargo run
# or
cargo run -- --list
```

Outputs a JSON array of detected JDKs on macOS.

### Set active JDK

```bash
# By ID (from the list output)
cargo run -- --set java-21_0_10

# By home path
cargo run -- --set /opt/homebrew/Cellar/openjdk@21/21.0.10/libexec/openjdk.jdk/Contents/Home
```

This writes the selected JDK's home path to `~/.jdk_current`.

### Get current active JDK

```bash
cargo run -- --get
```

Outputs the currently active JDK (if set) as JSON, or `{}` if none is set.

## State File

The active JDK is stored in `~/.jdk_current` as a single line containing the `JAVA_HOME` path. This file is the canonical source of truth that shell hooks and other tools will read from.
