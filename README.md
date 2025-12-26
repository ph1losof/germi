# Germi 🌱

**Germi** is an ultra-high-performance, feature-rich environment variable interpolation engine for Rust. It is designed to be the fastest and most correct interpolation library available, making it ideal for high-throughput configuration systems and tooling.

## 🚀 Features

- **⚡ Blazing Fast**: Uses SIMD (`memchr`) for scanning, resulting in sub-microsecond performance for most payloads.
- **🚫 Zero-Copy Friendly**: Returns `Cow<'a, str>` to avoid allocations whenever possible (zero heap allocation for variable-free strings).
- **🐚 Shell-Compatible Syntax**: Supports a wide range of standard shell parameter expansions.
- **🔄 Iterative & Recursive**: Correctly handles nested variables (`${A${B}}`) and recursive definitions with configurable depth limits.
- **⌨️ Asynchronous Command Substitution**: Supports `$(command)` expansion (requires `async` feature).
- **🛡️ Safe**: Recursion detection, depth limits, and strict error handling options.
- **🎛️ Highly Configurable**: Enable/disable specific features (commands, recursion, defaults) via `Config`.

## 📦 Installation

Add `germi` to your `Cargo.toml`:

```toml
[dependencies]
germi = "0.1.0"
# For async command substitution:
# germi = { version = "0.1.0", features = ["async"] }
```

## 📖 Usage

### Basic Usage

```rust
use germi::Germi;

fn main() {
    let mut germi = Germi::new();
    germi.add_variable("USER", "Alice");
    germi.add_variable("GREETING", "Hello");

    // Simple interpolation
    let result = germi.interpolate("${GREETING}, ${USER}!").unwrap();
    assert_eq!(result, "Hello, Alice!");

    // With defaults
    let result = germi.interpolate("Value: ${MISSING:-Default}").unwrap();
    assert_eq!(result, "Value: Default");
}
```

### Async Command Substitution

_Requires `features = ["async"]`_

```rust
use germi::Germi;

#[tokio::main]
async fn main() {
    let germi = Germi::new();
    // Executes command and substitutes output (trimmed)
    let result = germi.interpolate_async("Date: $(date +%Y)").await.unwrap();
    println!("{}", result); // "Date: 2024"
}
```

## 📝 Syntax Support

Germi supports a growing subset of standard shell expansions:

| Syntax            | Description                                                                  | Strict vs Loose                |
| ----------------- | ---------------------------------------------------------------------------- | ------------------------------ |
| `${VAR}`          | Basic substitution                                                           | -                              |
| `${VAR:-default}` | **Use Default**. Use `default` if VAR is unset or empty.                     | Strict (`:`) checks for empty. |
| `${VAR-default}`  | **Use Default**. Use `default` only if VAR is unset (empty string is valid). | Loose.                         |
| `${VAR:+alt}`     | **Use Alternate**. Use `alt` if VAR is set and not empty.                    | Strict.                        |
| `${VAR+alt}`      | **Use Alternate**. Use `alt` if VAR is set (even if empty).                  | Loose.                         |
| `$(command)`      | **Command Substitution**. Executes command and substitutes stdout.           | Requires `async`.              |
| `\n`, `\$`        | **Escapes**. Standard escape sequences.                                      | -                              |

## ⚡ Performance

Germi is built for speed. Recent benchmarks (running on Apple Silicon) show:

- **Simple Variables**: ~8 ns
- **Nested Variables**: ~8 ns
- **Large Payloads (100+ vars)**: ~90 ns
- **Literals**: ~12 ns

It achieves this by using `memchr::memchr3` to skip non-special characters using SIMD, avoiding expensive per-character iteration for the bulk of string processing.

## ⚙️ Configuration

You can fine-tune the engine:

```rust
use germi::{Config, Germi};

let mut config = Config::default();
config.max_depth = 5;            // Limit recursion depth
config.features.commands = false; // Disable $(cmd) for security

let germi = Germi::with_config(config);
```

## 📄 License

MIT
