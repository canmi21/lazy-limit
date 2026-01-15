/* src/config.rs */

use crate::types::{Duration, RuleConfig};
use axum::http::Method;
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

    pub fn get_rule_for_route(&self, route: &str, method: &Option<Method>) -> &RuleConfig {
        // First try exact match with method consideration
        if let Some(rule) = self.route_rules.get(route) {
            if rule.matches_method(method) {
                return rule;
            }
        }

        // Then try to find parent route if it's configured as prefix
        if let Some(rule) = self.find_parent_route_rule(route, method) {
            return rule;
        }

        &self.default_rule
    }

    pub fn has_route_rule(&self, route: &str, method: &Option<Method>) -> bool {
        // Check for exact match or parent route match with method consideration
        if let Some(rule) = self.route_rules.get(route) {
            if rule.matches_method(method) {
                return true;
            }
        }
        self.find_parent_route_rule(route, method).is_some()
    }

    /// Check if there's an exact route match (not a prefix match)
    pub fn is_exact_route(&self, route: &str, method: &Option<Method>) -> bool {
        if let Some(rule) = self.route_rules.get(route) {
            rule.matches_method(method)
        } else {
            false
        }
    }

    /// Check if there's a prefix route match (not an exact match)
    pub fn is_prefix_route(&self, route: &str, method: &Option<Method>) -> bool {
        !self.is_exact_route(route, method) && self.find_parent_route_rule(route, method).is_some()
    }

    fn find_parent_route_rule(&self, route: &str, method: &Option<Method>) -> Option<&RuleConfig> {
        // Find the longest parent route (prefix match)
        // Only matches routes that are configured with is_prefix=true and match the method
        // e.g. for "/api/contact/123" find "/api/contact/" if it has is_prefix=true
        let mut longest_prefix: Option<(&String, &RuleConfig)> = None;
        let mut longest_len = 0;

        for (configured_route, rule) in self.route_rules.iter() {
            // Only consider routes configured as prefix routes that match the method
            if rule.is_prefix
                && configured_route.ends_with('/')
                && route.starts_with(configured_route)
                && rule.matches_method(method)
            {
                if configured_route.len() > longest_len {
                    longest_prefix = Some((configured_route, rule));
                    longest_len = configured_route.len();
                }
            }
        }

        longest_prefix.map(|(_, rule)| rule)
    }
}
