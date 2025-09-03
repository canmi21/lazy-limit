# Lazy Limit

lazy-limit is a lightweight Rust library for rate limiting by IP or custom ID, with support for global, router-specific, and fallback rules.

## Overview

`lazy-limit` is a lightweight and flexible rate limiting library for Rust. It allows you to apply throttling rules based on IP addresses (IPv4/IPv6) or custom identifiers (any string). The crate supports global policies, fine-grained router-level rules, and fallback strategies, making it easy to integrate scalable and configurable rate limiting into your applications.