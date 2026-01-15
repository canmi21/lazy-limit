/* src/limiter.rs */

use crate::config::LimiterConfig;
use crate::gc::GarbageCollector;
use crate::types::{RequestRecord, RuleConfig};
use http::Method;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main rate limiter implementation
pub struct RateLimiter {
    config: LimiterConfig,
    records: Arc<RwLock<HashMap<String, HashMap<String, RequestRecord>>>>,
}

impl RateLimiter {
    pub async fn new(config: LimiterConfig) -> Self {
        let records = Arc::new(RwLock::new(HashMap::new()));

        let gc = GarbageCollector::new(records.clone(), config.clone());
        tokio::spawn(async move {
            gc.start().await;
        });

        Self { config, records }
    }

    pub async fn check_limit(
        &mut self,
        who: &str,
        route: &str,
        method: Option<Method>,
        override_mode: bool,
    ) -> bool {
        let (global_rule, route_rule_opt) = if override_mode {
            let rule = if self.config.has_route_rule(route, &method) {
                Some(self.config.get_rule_for_route(route, &method))
            } else {
                None
            };
            (None, rule)
        } else {
            // Check if this is a prefix route (not exact match)
            let is_prefix_route = self.config.is_prefix_route(route, &method);

            // Check if route has method-specific rules
            let has_method_specific_rule = if let Some(rule) = self.config.route_rules.get(route) {
                rule.methods.is_some()
            } else {
                false
            };

            if is_prefix_route || has_method_specific_rule {
                // For prefix routes and method-specific routes, only apply the specific rule, not global limit
                let rule = self.config.get_rule_for_route(route, &method);
                (None, Some(rule))
            } else {
                // For exact routes or default, apply global rule as before
                let rule = if self.config.has_route_rule(route, &method) {
                    self.config.get_rule_for_route(route, &method)
                } else {
                    &self.config.default_rule
                };
                (Some(&self.config.default_rule), Some(rule))
            }
        };

        if override_mode && route_rule_opt.is_none() {
            return true;
        }

        let records = self.records.read().await;

        let mut allow = true;

        if let Some(rule) = global_rule {
            let global_key = format!("__global__{}", who);
            if self.is_record_exceeded(&records, &global_key, "__global__", rule) {
                allow = false;
            }
        }

        if allow {
            if let Some(rule) = route_rule_opt {
                let record_key = self.get_record_key(route, &method, rule);
                if self.is_record_exceeded(&records, who, &record_key, rule) {
                    allow = false;
                }
            }
        }

        drop(records);

        if allow {
            let mut records = self.records.write().await;
            if let Some(rule) = global_rule {
                let global_key = format!("__global__{}", who);
                self.update_record(&mut records, &global_key, "__global__", rule);
            }
            if let Some(rule) = route_rule_opt {
                let record_key = self.get_record_key(route, &method, rule);
                self.update_record(&mut records, who, &record_key, rule);
            }
        }

        allow
    }

    /// Generate a unique record key based on route and method (if method-specific rule)
    fn get_record_key(&self, route: &str, method: &Option<Method>, rule: &RuleConfig) -> String {
        if rule.methods.is_some() && method.is_some() {
            // For method-specific rules, include the method in the key
            format!("{}::{}", route, method.as_ref().unwrap().as_str())
        } else {
            // For non-method-specific rules, just use the route
            route.to_string()
        }
    }

    fn is_record_exceeded(
        &self,
        records: &HashMap<String, HashMap<String, RequestRecord>>,
        who: &str,
        route: &str,
        rule: &RuleConfig,
    ) -> bool {
        let is_short_interval = rule.interval.is_short_interval();
        let window_size = rule.interval.as_seconds();

        if let Some(route_records) = records.get(who) {
            if let Some(record) = route_records.get(route) {
                return record.is_limit_exceeded(rule.limit, is_short_interval, window_size);
            }
        }
        false
    }

    fn update_record(
        &self,
        records: &mut HashMap<String, HashMap<String, RequestRecord>>,
        who: &str,
        route: &str,
        rule: &RuleConfig,
    ) {
        let is_short_interval = rule.interval.is_short_interval();
        let window_size = rule.interval.as_seconds();

        let route_records = records.entry(who.to_string()).or_insert_with(HashMap::new);
        let record = route_records
            .entry(route.to_string())
            .or_insert_with(|| RequestRecord::new(is_short_interval));

        record.add_request(is_short_interval, window_size);
    }

    #[allow(dead_code)]
    pub async fn get_stats(&self) -> (usize, usize) {
        let records = self.records.read().await;
        let total_users = records.len();
        let total_routes = records.values().map(|r| r.len()).sum();
        (total_users, total_routes)
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub async fn clear_all(&mut self) {
        let mut records = self.records.write().await;
        records.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Duration, RuleConfig};
    use std::time::Duration as StdDuration;

    #[tokio::test]
    async fn test_rate_limiting_basic() {
        let config = LimiterConfig::new(RuleConfig::new(Duration::seconds(1), 2));
        let mut limiter = RateLimiter::new(config).await;

        let who = "test_user_basic";
        let route = "/test";

        assert!(limiter.check_limit(who, route, None, false).await);
        assert!(limiter.check_limit(who, route, None, false).await);
        assert!(!limiter.check_limit(who, route, None, false).await);

        tokio::time::sleep(StdDuration::from_millis(1100)).await;
        assert!(limiter.check_limit(who, route, None, false).await);
    }

    #[tokio::test]
    async fn test_route_specific_rules_and_global_limit() {
        let config = LimiterConfig::new(RuleConfig::new(Duration::seconds(1), 2))
            .add_route_rule("/special", RuleConfig::new(Duration::seconds(1), 5));

        let mut limiter = RateLimiter::new(config).await;
        let who = "test_user_route";

        assert!(
            limiter.check_limit(who, "/special", None, false).await,
            "Req 1 to /special should pass"
        );
        assert!(
            limiter.check_limit(who, "/special", None, false).await,
            "Req 2 to /special should pass"
        );

        assert!(
            !limiter.check_limit(who, "/special", None, false).await,
            "Req 3 to /special should fail due to global limit"
        );

        assert!(
            !limiter.check_limit(who, "/regular", None, false).await,
            "Req to /regular should fail as global limit is reached"
        );

        tokio::time::sleep(StdDuration::from_millis(1100)).await;

        assert!(
            limiter.check_limit(who, "/regular", None, false).await,
            "Req 1 to /regular after wait should pass"
        );
        assert!(
            limiter.check_limit(who, "/regular", None, false).await,
            "Req 2 to /regular after wait should pass"
        );
        assert!(
            !limiter.check_limit(who, "/regular", None, false).await,
            "Req 3 to /regular after wait should fail"
        );
    }

    #[tokio::test]
    async fn test_rate_limiting_prefix_matching() {
        let config = LimiterConfig::new(RuleConfig::new(Duration::minutes(1), 1)).add_route_rule(
            "/prefix/",
            RuleConfig::new(Duration::seconds(1), 2).match_prefix(true),
        );
        let mut limiter = RateLimiter::new(config).await;

        let who = "test_user_basic";
        let route = "/prefix/{test}";

        assert!(
            limiter.check_limit(who, route, None, true).await,
            "Req 1 to /prefix/* should pass"
        );
        assert!(
            limiter.check_limit(who, route, None, true).await,
            "Req 2 to /prefix/* should pass"
        );
        assert!(
            !limiter.check_limit(who, route, None, true).await,
            "Req 3 to /prefix/* should fail"
        );

        tokio::time::sleep(StdDuration::from_millis(1100)).await;
        assert!(
            limiter.check_limit(who, route, None, true).await,
            "Req 4 to /prefix/* after wait should pass"
        );
    }

    #[tokio::test]
    async fn test_override_mode() {
        let config = LimiterConfig::new(RuleConfig::new(Duration::seconds(1), 1))
            .add_route_rule("/premium", RuleConfig::new(Duration::seconds(1), 5));

        let mut limiter = RateLimiter::new(config).await;
        let who = "test_user_override";

        for i in 1..=5 {
            assert!(
                limiter.check_limit(who, "/premium", None, true).await,
                "Override request {} should pass",
                i
            );
        }
        assert!(
            !limiter.check_limit(who, "/premium", None, true).await,
            "Override request 6 should fail"
        );

        assert!(
            limiter.check_limit(who, "/other", None, true).await,
            "/other should be allowed in override"
        );
    }

    #[tokio::test]
    async fn test_different_users() {
        let config = LimiterConfig::new(RuleConfig::new(Duration::seconds(1), 1));
        let mut limiter = RateLimiter::new(config).await;
        let route = "/test_multi_user";

        assert!(limiter.check_limit("user1", route, None, false).await);
        assert!(!limiter.check_limit("user1", route, None, false).await);

        assert!(limiter.check_limit("user2", route, None, false).await);
        assert!(!limiter.check_limit("user2", route, None, false).await);
    }

    #[tokio::test]
    async fn test_method_specific_rules() {
        let config = LimiterConfig::new(RuleConfig::new(Duration::seconds(1), 1))
            .add_route_rule(
                "/api/data",
                RuleConfig::new(Duration::seconds(1), 5).for_methods(vec![Method::GET]),
            )
            .add_route_rule(
                "/api/create",
                RuleConfig::new(Duration::seconds(1), 2).for_methods(vec![Method::POST]),
            );

        let mut limiter = RateLimiter::new(config).await;
        let who = "test_user_methods";

        // Test GET on /api/data with limit 5
        for i in 1..=5 {
            assert!(
                limiter
                    .check_limit(who, "/api/data", Some(Method::GET), false)
                    .await,
                "GET request {} to /api/data should pass",
                i
            );
        }
        assert!(
            !limiter
                .check_limit(who, "/api/data", Some(Method::GET), false)
                .await,
            "GET request 6 to /api/data should fail"
        );

        // Test POST on /api/data should use default rule (limit 1)
        assert!(
            limiter
                .check_limit(who, "/api/data", Some(Method::POST), false)
                .await,
            "POST request 1 to /api/data should pass with default rule"
        );
        assert!(
            !limiter
                .check_limit(who, "/api/data", Some(Method::POST), false)
                .await,
            "POST request 2 to /api/data should fail (default limit 1)"
        );

        // Test POST on /api/create with limit 2
        let who2 = "test_user_methods2";
        assert!(
            limiter
                .check_limit(who2, "/api/create", Some(Method::POST), false)
                .await,
            "POST request 1 to /api/create should pass"
        );
        assert!(
            limiter
                .check_limit(who2, "/api/create", Some(Method::POST), false)
                .await,
            "POST request 2 to /api/create should pass"
        );
        assert!(
            !limiter
                .check_limit(who2, "/api/create", Some(Method::POST), false)
                .await,
            "POST request 3 to /api/create should fail"
        );

        // Test GET on /api/create should use default rule (limit 1)
        let who3 = "test_user_methods3";
        assert!(
            limiter
                .check_limit(who3, "/api/create", Some(Method::GET), false)
                .await,
            "GET request 1 to /api/create should pass with default rule"
        );
        assert!(
            !limiter
                .check_limit(who3, "/api/create", Some(Method::GET), false)
                .await,
            "GET request 2 to /api/create should fail (default limit 1)"
        );
    }

    #[tokio::test]
    async fn test_method_isolation() {
        // Test that different methods on different routes have separate counters
        let config = LimiterConfig::new(RuleConfig::new(Duration::seconds(1), 10))
            .add_route_rule(
                "/api/post-only",
                RuleConfig::new(Duration::seconds(1), 3).for_methods(vec![Method::POST]),
            )
            .add_route_rule(
                "/api/get-only",
                RuleConfig::new(Duration::seconds(1), 5).for_methods(vec![Method::GET]),
            );

        let mut limiter = RateLimiter::new(config).await;
        let who = "test_user_isolation";

        // POST requests to /api/post-only should have limit 3
        for i in 1..=3 {
            assert!(
                limiter
                    .check_limit(who, "/api/post-only", Some(Method::POST), false)
                    .await,
                "POST request {} should pass",
                i
            );
        }
        assert!(
            !limiter
                .check_limit(who, "/api/post-only", Some(Method::POST), false)
                .await,
            "POST request 4 should fail"
        );

        // GET requests to /api/get-only should still work (separate counter with limit 5)
        for i in 1..=5 {
            assert!(
                limiter
                    .check_limit(who, "/api/get-only", Some(Method::GET), false)
                    .await,
                "GET request {} should pass (separate counter)",
                i
            );
        }
        assert!(
            !limiter
                .check_limit(who, "/api/get-only", Some(Method::GET), false)
                .await,
            "GET request 6 should fail"
        );

        // GET on /api/post-only should use default rule (limit 10)
        for i in 1..=10 {
            assert!(
                limiter
                    .check_limit(who, "/api/post-only", Some(Method::GET), false)
                    .await,
                "GET request {} to POST-only route should pass with default",
                i
            );
        }
        assert!(
            !limiter
                .check_limit(who, "/api/post-only", Some(Method::GET), false)
                .await,
            "GET request 11 to POST-only route should fail"
        );
    }

    #[tokio::test]
    async fn test_multiple_methods_same_rule() {
        // Test a rule that applies to multiple methods (they share the same counter)
        let config = LimiterConfig::new(RuleConfig::new(Duration::seconds(1), 1)).add_route_rule(
            "/api/modify",
            RuleConfig::new(Duration::seconds(1), 3).for_methods(vec![
                Method::POST,
                Method::PUT,
                Method::PATCH,
            ]),
        );

        let mut limiter = RateLimiter::new(config).await;
        let who = "test_user_multi_methods";

        // POST, PUT, and PATCH all match the same rule, so they get separate counters per method
        // (because record_key includes the method)
        assert!(
            limiter
                .check_limit(who, "/api/modify", Some(Method::POST), false)
                .await,
            "POST request 1 should pass"
        );
        assert!(
            limiter
                .check_limit(who, "/api/modify", Some(Method::POST), false)
                .await,
            "POST request 2 should pass"
        );
        assert!(
            limiter
                .check_limit(who, "/api/modify", Some(Method::POST), false)
                .await,
            "POST request 3 should pass"
        );
        assert!(
            !limiter
                .check_limit(who, "/api/modify", Some(Method::POST), false)
                .await,
            "POST request 4 should fail"
        );

        // PUT should have its own counter
        assert!(
            limiter
                .check_limit(who, "/api/modify", Some(Method::PUT), false)
                .await,
            "PUT request 1 should pass"
        );
        assert!(
            limiter
                .check_limit(who, "/api/modify", Some(Method::PUT), false)
                .await,
            "PUT request 2 should pass"
        );

        // GET should use default rule
        let who2 = "test_user_multi_methods2";
        assert!(
            limiter
                .check_limit(who2, "/api/modify", Some(Method::GET), false)
                .await,
            "GET request should pass with default rule"
        );
        assert!(
            !limiter
                .check_limit(who2, "/api/modify", Some(Method::GET), false)
                .await,
            "2nd GET request should fail (default limit 1)"
        );
    }
}
