/* src/types.rs */

use std::time::{SystemTime, UNIX_EPOCH};

/// HTTP Method enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
    CONNECT,
    TRACE,
    OTHER,
}

impl HttpMethod {
    pub fn as_str(&self) -> &str {
        match self {
            Self::GET => "GET",
            Self::POST => "POST",
            Self::PUT => "PUT",
            Self::DELETE => "DELETE",
            Self::PATCH => "PATCH",
            Self::HEAD => "HEAD",
            Self::OPTIONS => "OPTIONS",
            Self::CONNECT => "CONNECT",
            Self::TRACE => "TRACE",
            Self::OTHER => "OTHER",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(Self::GET),
            "POST" => Some(Self::POST),
            "PUT" => Some(Self::PUT),
            "DELETE" => Some(Self::DELETE),
            "PATCH" => Some(Self::PATCH),
            "HEAD" => Some(Self::HEAD),
            "OPTIONS" => Some(Self::OPTIONS),
            "CONNECT" => Some(Self::CONNECT),
            "TRACE" => Some(Self::TRACE),
            _ => Some(Self::OTHER),
        }
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for HttpMethod {
    fn from(s: &str) -> Self {
        HttpMethod::from_str(s).unwrap_or_else(|| {
            panic!("Invalid HTTP method: {}. Expected one of: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS, CONNECT, TRACE", s)
        })
    }
}

impl From<&String> for HttpMethod {
    fn from(s: &String) -> Self {
        HttpMethod::from(s.as_str())
    }
}

impl From<String> for HttpMethod {
    fn from(s: String) -> Self {
        HttpMethod::from(s.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Duration {
    Seconds(u64),
    Minutes(u64),
    Hours(u64),
    Days(u64),
}

impl Duration {
    pub fn seconds(n: u64) -> Self {
        Duration::Seconds(n)
    }
    pub fn minutes(n: u64) -> Self {
        Duration::Minutes(n)
    }
    pub fn hours(n: u64) -> Self {
        Duration::Hours(n)
    }
    pub fn days(n: u64) -> Self {
        Duration::Days(n)
    }

    pub fn as_seconds(&self) -> u64 {
        match self {
            Duration::Seconds(n) => *n,
            Duration::Minutes(n) => n * 60,
            Duration::Hours(n) => n * 3600,
            Duration::Days(n) => n * 86400,
        }
    }

    pub fn is_short_interval(&self) -> bool {
        self.as_seconds() <= 300 // 5 minutes
    }
}

#[derive(Debug, Clone)]
pub struct RuleConfig {
    pub interval: Duration,
    pub limit: u32,
    pub is_prefix: bool,
    pub methods: Option<Vec<HttpMethod>>,
}

impl RuleConfig {
    pub fn new(interval: Duration, limit: u32) -> Self {
        Self {
            interval,
            limit,
            is_prefix: false,
            methods: None,
        }
    }

    pub fn match_prefix(mut self, is_prefix: bool) -> Self {
        self.is_prefix = is_prefix;
        self
    }

    pub fn for_methods(mut self, methods: Vec<HttpMethod>) -> Self {
        self.methods = Some(methods);
        self
    }

    /// Check if this rule applies to the given HTTP method
    pub fn matches_method(&self, method: &Option<HttpMethod>) -> bool {
        match (&self.methods, method) {
            // If no methods specified, rule applies to all methods
            (None, _) => true,
            // If methods specified but no method provided, no match
            (Some(_), None) => false,
            // If both specified, check if method is in the list
            (Some(allowed), Some(m)) => allowed.contains(m),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestRecord {
    pub count: u32,
    pub window_start: u64,
    pub timestamps: Vec<u64>,
}

impl RequestRecord {
    pub fn new(is_short_interval: bool) -> Self {
        Self {
            count: 0,
            window_start: current_timestamp(),
            timestamps: if is_short_interval {
                Vec::new()
            } else {
                Vec::with_capacity(16)
            },
        }
    }

    pub fn add_request(&mut self, is_short_interval: bool, window_size: u64) {
        let now = current_timestamp();

        if is_short_interval {
            if now.saturating_sub(self.window_start) >= window_size {
                self.window_start = now;
                self.count = 1;
            } else {
                self.count += 1;
            }
        } else {
            self.timestamps.push(now);
            let cutoff = now.saturating_sub(window_size);
            self.timestamps.retain(|&t| t > cutoff);
            self.count = self.timestamps.len() as u32;
        }
    }

    pub fn is_limit_exceeded(&self, limit: u32, is_short_interval: bool, window_size: u64) -> bool {
        let now = current_timestamp();
        if is_short_interval {
            if now.saturating_sub(self.window_start) >= window_size {
                false
            } else {
                self.count >= limit
            }
        } else {
            let cutoff = now.saturating_sub(window_size);
            let valid_requests = self.timestamps.iter().filter(|&&t| t > cutoff).count() as u32;
            valid_requests >= limit
        }
    }

    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + self.timestamps.capacity() * std::mem::size_of::<u64>()
    }

    pub fn should_cleanup(&self, max_age_seconds: u64) -> bool {
        let now = current_timestamp();
        let last_activity = if !self.timestamps.is_empty() {
            *self.timestamps.last().unwrap_or(&self.window_start)
        } else {
            self.window_start
        };
        now.saturating_sub(last_activity) > max_age_seconds
    }
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}
