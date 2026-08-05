//! Sentinel Authenticator — application entry point.
//!
//! The actual UI scaffolding is wired in `lib.rs::run`. `main.rs` is kept
//! intentionally thin so that the same code can be exercised from
//! integration tests (`cargo test`) without spinning a window.

fn main() {
    sentinel_authenticator::run();
}
