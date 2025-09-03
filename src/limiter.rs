/* src/limiter.rs */

use crate::config::LimiterConfig;
use crate::gc::GarbageCollector;
use crate::types::{RequestRecord, RuleConfig};
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

    pub async fn check_limit(&mut self, who: &str, route: &str, override_mode: bool) -> bool {
        let (global_rule, route_rule_opt) = if override_mode {
            let rule = if self.config.has_route_rule(route) {
                Some(self.config.get_rule_for_route(route))
            } else {
                None
            };
            (None, rule)
        } else {
            let rule = if self.config.has_route_rule(route) {
                self.config.get_rule_for_route(route)
            } else {
                &self.config.default_rule
            };
            (Some(&self.config.default_rule), Some(rule))
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
                if self.is_record_exceeded(&records, who, route, rule) {
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
                self.update_record(&mut records, who, route, rule);
            }
        }

        allow
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

        assert!(limiter.check_limit(who, route, false).await);
        assert!(limiter.check_limit(who, route, false).await);
        assert!(!limiter.check_limit(who, route, false).await);

        tokio::time::sleep(StdDuration::from_millis(1100)).await;
        assert!(limiter.check_limit(who, route, false).await);
    }

    #[tokio::test]
    async fn test_route_specific_rules_and_global_limit() {
        let config = LimiterConfig::new(RuleConfig::new(Duration::seconds(1), 2))
            .add_route_rule("/special", RuleConfig::new(Duration::seconds(1), 5));

        let mut limiter = RateLimiter::new(config).await;
        let who = "test_user_route";

        assert!(
            limiter.check_limit(who, "/special", false).await,
            "Req 1 to /special should pass"
        );
        assert!(
            limiter.check_limit(who, "/special", false).await,
            "Req 2 to /special should pass"
        );

        assert!(
            !limiter.check_limit(who, "/special", false).await,
            "Req 3 to /special should fail due to global limit"
        );

        assert!(
            !limiter.check_limit(who, "/regular", false).await,
            "Req to /regular should fail as global limit is reached"
        );

        tokio::time::sleep(StdDuration::from_millis(1100)).await;

        assert!(
            limiter.check_limit(who, "/regular", false).await,
            "Req 1 to /regular after wait should pass"
        );
        assert!(
            limiter.check_limit(who, "/regular", false).await,
            "Req 2 to /regular after wait should pass"
        );
        assert!(
            !limiter.check_limit(who, "/regular", false).await,
            "Req 3 to /regular after wait should fail"
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
                limiter.check_limit(who, "/premium", true).await,
                "Override request {} should pass",
                i
            );
        }
        assert!(
            !limiter.check_limit(who, "/premium", true).await,
            "Override request 6 should fail"
        );

        assert!(
            limiter.check_limit(who, "/other", true).await,
            "/other should be allowed in override"
        );
    }

    #[tokio::test]
    async fn test_different_users() {
        let config = LimiterConfig::new(RuleConfig::new(Duration::seconds(1), 1));
        let mut limiter = RateLimiter::new(config).await;
        let route = "/test_multi_user";

        assert!(limiter.check_limit("user1", route, false).await);
        assert!(!limiter.check_limit("user1", route, false).await);

        assert!(limiter.check_limit("user2", route, false).await);
        assert!(!limiter.check_limit("user2", route, false).await);
    }
}
