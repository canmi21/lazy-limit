/* src/config.rs */

use crate::types::{Duration, RuleConfig};
use std::collections::HashMap;

/// Configuration for the rate limiter
#[derive(Debug, Clone)]
pub struct LimiterConfig {
    pub default_rule: RuleConfig,
    pub route_rules: HashMap<String, RuleConfig>,
    pub max_memory: usize,
    pub gc_interval: u64,
}

impl LimiterConfig {
    pub fn new(default_rule: RuleConfig) -> Self {
        Self {
            default_rule,
            route_rules: HashMap::new(),
            max_memory: 64 * 1024 * 1024, // 64MB default
            gc_interval: 10,              // 10 seconds default
        }
    }

    pub fn add_route_rule(mut self, route: &str, rule: RuleConfig) -> Self {
        self.route_rules.insert(route.to_string(), rule);
        self
    }

    pub fn with_max_memory(mut self, max_memory: usize) -> Self {
        self.max_memory = max_memory;
        self
    }

    pub fn with_gc_interval(mut self, gc_interval: u64) -> Self {
        self.gc_interval = gc_interval;
        self
    }

    pub fn max_interval(&self) -> Duration {
        let mut max = self.default_rule.interval;

        for rule in self.route_rules.values() {
            if rule.interval > max {
                max = rule.interval;
            }
        }

        max
    }

    pub fn get_rule_for_route(&self, route: &str) -> &RuleConfig {
        self.route_rules.get(route).unwrap_or(&self.default_rule)
    }

    pub fn has_route_rule(&self, route: &str) -> bool {
        self.route_rules.contains_key(route)
    }
}
