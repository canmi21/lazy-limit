/* src/lib.rs */

use std::sync::Arc;
use tokio::sync::{OnceCell, RwLock};

mod config;
mod gc;
mod limiter;
mod types;

pub use config::*;
use limiter::RateLimiter;
pub use types::*;

// Global rate limiter instance, initialized once.
static GLOBAL_LIMITER: OnceCell<Arc<RwLock<RateLimiter>>> = OnceCell::const_new();

/// Initialize the rate limiter with default and optional route-specific rules.
/// This must be called once, typically at application startup, before any calls to `limit!`.
///
/// # Panics
///
/// Panics if called more than once.
///
/// # Examples
///
/// ```rust,ignore
/// use lazy_limit::*;
///
/// #[tokio::main]
/// async fn main() {
///     init_rate_limiter!(
///         default: RuleConfig::new(Duration::seconds(1), 5),
///         max_memory: Some(64 * 1024 * 1024), // 64MB
///         routes: [
///             ("/api/login", RuleConfig::new(Duration::minutes(1), 3)),
///             ("/api/public", RuleConfig::new(Duration::seconds(1), 10)),
///         ]
///     ).await;
/// }
/// ```
#[macro_export]
macro_rules! init_rate_limiter {
    (
        default: $default_rule:expr
        $(, max_memory: $max_memory:expr)?
        $(, routes: [ $(($route:expr, $rule:expr)),* $(,)? ])?
    ) => {
        {
            let mut config = $crate::LimiterConfig::new($default_rule);

            $(
                if let Some(mem) = $max_memory {
                    config = config.with_max_memory(mem);
                }
            )?

            $(
                $(
                    config = config.add_route_rule($route, $rule);
                )*
            )?

            $crate::initialize_limiter(config)
        }
    };
}

/// Check if a request should be allowed based on rate limiting rules.
///
/// # Panics
///
/// Panics if the rate limiter has not been initialized.
#[macro_export]
macro_rules! limit {
    ($who:expr, $route:expr) => {
        $crate::check_limit($who, $route)
    };
}

/// Check rate limit with override mode (only applies route-specific rules).
///
/// # Panics
///
/// Panics if the rate limiter has not been initialized.
#[macro_export]
macro_rules! limit_override {
    ($who:expr, $route:expr) => {
        $crate::check_limit_override($who, $route)
    };
}

/// Initialize the global rate limiter. Should be called only once.
pub async fn initialize_limiter(config: LimiterConfig) {
    let limiter = RateLimiter::new(config).await;
    if GLOBAL_LIMITER.set(Arc::new(RwLock::new(limiter))).is_err() {
        panic!("Rate limiter has already been initialized.");
    }
}

/// Check if a request should be allowed.
pub async fn check_limit(who: &str, route: &str) -> bool {
    if let Some(limiter) = GLOBAL_LIMITER.get() {
        let mut limiter = limiter.write().await;
        limiter.check_limit(who, route, false).await
    } else {
        panic!("Rate limiter not initialized! Call init_rate_limiter! first.");
    }
}

/// Check rate limit with override mode.
pub async fn check_limit_override(who: &str, route: &str) -> bool {
    if let Some(limiter) = GLOBAL_LIMITER.get() {
        let mut limiter = limiter.write().await;
        limiter.check_limit(who, route, true).await
    } else {
        panic!("Rate limiter not initialized! Call init_rate_limiter! first.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Duration;
    use std::time::Duration as StdDuration;

    #[tokio::test]
    async fn test_basic_rate_limiting() {
        // We re-create the limiter for each test, which isn't ideal with a global static.
        // For a simple test suite, this works by overwriting.
        let config = LimiterConfig::new(RuleConfig::new(Duration::seconds(1), 2));
        let limiter = RateLimiter::new(config).await;
        let _ = GLOBAL_LIMITER.set(Arc::new(RwLock::new(limiter)));

        let who = "test_ip";
        let route = "/test";

        assert!(check_limit(who, route).await);
        assert!(check_limit(who, route).await);

        assert!(!check_limit(who, route).await);

        tokio::time::sleep(StdDuration::from_secs(1)).await;
        assert!(check_limit(who, route).await);
    }
}
