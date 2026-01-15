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
    pub prefix_matching: bool,
}

impl LimiterConfig {
    pub fn new(default_rule: RuleConfig) -> Self {
        Self {
            default_rule,
            route_rules: HashMap::new(),
            max_memory: 64 * 1024 * 1024, // 64MB default
            gc_interval: 10,              // 10 seconds default
            prefix_matching: false,       // Disabled by default
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

    pub fn with_prefix_matching(mut self, enable: bool) -> Self {
        self.prefix_matching = enable;
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
        // First try exact match
        if let Some(rule) = self.route_rules.get(route) {
            return rule;
        }

        // Then try to find parent route if prefix matching is enabled
        if self.prefix_matching {
            if let Some(rule) = self.find_parent_route_rule(route) {
                return rule;
            }
        }

        &self.default_rule
    }

    pub fn has_route_rule(&self, route: &str) -> bool {
        // Check for exact match
        if self.route_rules.contains_key(route) {
            return true;
        }
        // Check for parent route if prefix matching is enabled
        if self.prefix_matching {
            return self.find_parent_route_rule(route).is_some();
        }
        false
    }

    fn find_parent_route_rule(&self, route: &str) -> Option<&RuleConfig> {
        // Find the longest parent route (prefix match)
        // e.g. for "/api/contact/123" find "/api/contact/"
        let mut longest_prefix: Option<&String> = None;
        let mut longest_len = 0;

        for configured_route in self.route_rules.keys() {
            // Parent route must end with "/" and be a prefix
            if configured_route.ends_with('/') && route.starts_with(configured_route) {
                if configured_route.len() > longest_len {
                    longest_prefix = Some(configured_route);
                    longest_len = configured_route.len();
                }
            }
        }

        longest_prefix.and_then(|prefix| self.route_rules.get(prefix))
    }
}
