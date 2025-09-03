# Lazy-Limit

**Lazy-Limit** is a lightweight and flexible Rust library for implementing rate limiting based on IP addresses or custom identifiers. It supports global rate limits, route-specific rules, and an override mode for fine-grained control. Designed for ease of use, it integrates seamlessly with asynchronous Rust applications using Tokio, making it ideal for web servers, APIs, or any networked application requiring rate limiting.

## Features

- **Global Rate Limiting**: Apply a default rate limit across all requests.
- **Route-Specific Rules**: Define custom rate limits for specific routes or endpoints.
- **Override Mode**: Bypass global limits to enforce only route-specific rules when needed.
- **Memory Management**: Built-in garbage collection to manage memory usage for request records.
- **Asynchronous Design**: Built on Tokio for non-blocking, high-performance rate limiting.
- **Customizable Configuration**: Set maximum memory usage, garbage collection intervals, and more.
- **Thread-Safe**: Uses `Arc` and `RwLock` for safe concurrent access.
- **Extensive Testing**: Comprehensive unit tests and a demo example to verify functionality.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
lazy-limit = "1"
```

Ensure you have the required dependencies:

```toml
tokio = { version = "1", features = ["full"] }
once_cell = "1"
```

## Usage

### Initializing the Rate Limiter

The rate limiter must be initialized once at application startup using the `init_rate_limiter!` macro. You can specify a default global rule, optional route-specific rules, and memory limits.

```rust
use lazy_limit::*;
use std::time::Duration as StdDuration;

#[tokio::main]
async fn main() {
    init_rate_limiter!(
        default: RuleConfig::new(Duration::seconds(1), 5), // 5 req/s globally
        max_memory: Some(64 * 1024 * 1024), // 64MB max memory
        routes: [
            ("/api/login", RuleConfig::new(Duration::minutes(1), 3)), // 3 req/min
            ("/api/public", RuleConfig::new(Duration::seconds(1), 10)), // 10 req/s
            ("/api/premium", RuleConfig::new(Duration::seconds(1), 20)), // 20 req/s
        ]
    ).await;

    // Your application logic here
}
```

### Checking Rate Limits

Use the `limit!` macro to check if a request should be allowed based on the identifier (e.g., IP address) and route.

```rust
let allowed = limit!("1.1.1.1", "/api/public").await;
if allowed {
    println!("Request allowed!");
} else {
    println!("Request denied: rate limit exceeded.");
}
```

### Override Mode

Use the `limit_override!` macro to apply only route-specific rules, ignoring the global limit.

```rust
let allowed = limit_override!("1.1.1.1", "/api/premium").await;
if allowed {
    println!("Request allowed in override mode!");
} else {
    println!("Request denied in override mode.");
}
```

### Example Demo

The library includes a demo in `examples/demo.rs` that showcases its features:

- **Basic Global Rate Limiting**: Tests the global limit of 5 requests per second.
- **Route-Specific Rules**: Demonstrates how global and route-specific limits interact.
- **Override Mode**: Shows how to bypass global limits for specific routes.
- **Multiple Users**: Verifies independent rate limiting for different identifiers.
- **Long Interval Rules**: Tests rules with longer time windows (e.g., 3 requests per minute).

To run the demo:

```bash
cargo run --example demo
```

Expected output includes detailed test results for each scenario, confirming the rate limiter's behavior.

## Project Structure

```plaintext
lazy-limit/
├── examples/
│   └── demo.rs         # Example demonstrating rate limiting features
├── src/
│   ├── config.rs       # Configuration for rate limiter rules
│   ├── gc.rs           # Garbage collection for memory management
│   ├── lib.rs          # Main library entry point and macros
│   ├── limiter.rs      # Core rate limiter implementation
│   └── types.rs        # Data types for duration, rules, and request records
├── Cargo.toml          # Project metadata and dependencies
├── LICENSE             # MIT License
└── README.md           # This file
```

## Configuration Options

- **Default Rule**: Set a global rate limit using `RuleConfig::new(Duration, limit)`.
- **Route-Specific Rules**: Add rules for specific routes using the `routes` field in `init_rate_limiter!`.
- **Max Memory**: Limit memory usage for request records (default: 64MB).
- **Garbage Collection Interval**: Configure how often stale records are cleaned (default: 10 seconds).

Example configuration:

```rust
let config = LimiterConfig::new(RuleConfig::new(Duration::seconds(1), 5))
    .add_route_rule("/api/login", RuleConfig::new(Duration::minutes(1), 3))
    .with_max_memory(32 * 1024 * 1024) // 32MB
    .with_gc_interval(5); // GC every 5 seconds
```

## Testing

The library includes comprehensive unit tests to ensure reliability:

```bash
cargo test --all
```

This runs tests in `src/lib.rs` and `src/limiter.rs`, covering:

- Basic rate limiting
- Route-specific rules with global limits
- Override mode
- Multiple users
- Long interval rules

## Memory Management

The rate limiter includes a garbage collector (`gc.rs`) that:

- **Routine Cleanup**: Removes stale records older than the longest rule interval plus a 5-minute buffer.
- **Aggressive Cleanup**: Triggered when memory usage exceeds the configured limit, removing oldest entries to stay within 80% of the max memory.

The garbage collector runs asynchronously in a Tokio task, ensuring non-blocking operation.

## Limitations

- **Single Initialization**: The rate limiter can only be initialized once. Attempting to call `init_rate_limiter!` multiple times will panic.
- **Static Configuration**: Rules are set at initialization and cannot be modified at runtime.
- **Memory Estimation**: Memory usage calculations are approximate and may vary based on the Rust allocator.

## Contributing

Contributions are welcome! Please submit issues or pull requests to the [GitHub repository](https://github.com/canmi21/lazy-limit).

1. Fork the repository.
2. Create a new branch (`git checkout -b feature/your-feature`).
3. Make your changes and commit (`git commit -m "Add your feature"`).
4. Push to the branch (`git push origin feature/your-feature`).
5. Open a pull request.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contact

For questions or support, please open an issue on the [GitHub repository](https://github.com/canmi21/lazy-limit).
