# Germi

**Germi** is a high-performance, pure Rust environment variable interpolation engine designed for tooling and configuration systems. It allows for safe, deterministic, and flexible variable substitution in strings.

## Features

- **Pure Rust**: Minimal dependencies (only `regex` is used if enabled, but core is dependency-free).
- **High Performance**: Designed with zero-copy principles. Returns `Cow<'a, str>` to avoid allocations when no substitution occurs.
- **Comprehensive Syntax**:
    - `${VAR}` and `$VAR`
    - Defaults: `${VAR:-default}`, `${VAR-default}`
    - Conditionals: `${VAR:+val}`, `${VAR+val}`
    - Escapes: `\n`, `\t`, `\\`, etc.
- **Iterative Resolution**: Supports nested variable references (`VAR="Hello ${USER}"`).
- **Safe**: Configurable recursion depth to prevent cycles.

## Usage

```rust
use germi::Germi;

fn main() {
    let mut germi = Germi::new();
    germi.add_variable("USER", "world");

    let result = germi.interpolate("Hello, ${USER}!").unwrap();
    assert_eq!(result, "Hello, world!");
}
```

## Performance & Tradeoffs

- **Scanning**: Uses a single-pass scanner that identifies variables and literals.
- **Interpolation**: 
    - **Zero-copy fast path**: If the input string contains no variables or recursive resolutions, `germi` returns `Cow::Borrowed(input)`, resulting in zero heap allocations.
    - **Allocation on write**: Only when a variable is substituted or an escape sequence is processed does `germi` allocate a new `String`.
- **Recursion**: Variable values are recursively resolved. Deep recursion is limited by `max_depth` (default 10) to prevent stack overflow or infinite loops.

## License

MIT
