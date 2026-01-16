/* examples/demo.rs */

use lazy_limit::*;
use std::time::Duration as StdDuration;
use tokio::time::sleep;

async fn test_basic_limit() {
    let ip = "1.1.1.1";
    println!("  Testing IP: {}", ip);
    println!("  Global rule: 15 req/s. Should allow 15, then deny 16th.");

    for i in 1..=17 {
        let allowed = limit!(ip, "/some/path").await;
        println!(
            "  Request #{}: {}",
            i,
            if allowed { "Allowed" } else { "Denied" }
        );
        assert_eq!(allowed, i <= 15);
    }

    println!("  Waiting for 1 second...");
    sleep(StdDuration::from_secs(1)).await;

    let allowed = limit!(ip, "/some/path").await;
    println!(
        "  Request #8 after 1s: {}",
        if allowed { "Allowed" } else { "Denied" }
    );
    assert!(allowed);
    println!("  Basic test passed.");
}

async fn test_route_specific() {
    let ip = "2.2.2.2";
    println!("  Testing IP: {}", ip);
    println!("  Global rule: 15 req/s. Route /api/public rule: 5 req/s.");
    println!("  Requests to /api/public are limited to 5 per second (route rule).");

    for i in 1..=7 {
        let allowed = limit!(ip, "/api/public").await;
        println!(
            "  Request #{} to /api/public: {}",
            i,
            if allowed { "Allowed" } else { "Denied" }
        );
        assert_eq!(allowed, i <= 5);
    }

    println!("  Route limit for /api/public is now reached.");
    println!("  Other routes still use the global limit (15):");
    assert!(limit!(ip, "/another/path").await);
    println!("  /another/path: Allowed");

    println!("  Route-specific test passed.");
}

async fn test_override_mode() {
    let ip = "3.3.3.3";
    println!("  Testing IP: {}", ip);
    println!("  Global rule: 5 req/s. Route /api/premium rule: 20 req/s.");
    println!("  Using override mode on /api/premium, should allow 20 requests.");

    for i in 1..=21 {
        let allowed = limit_override!(ip, "/api/premium").await;
        if i <= 20 {
            assert!(allowed);
        } else {
            println!("  Request #{}: Denied (as expected)", i);
            assert!(!allowed);
        }
    }
    println!("  Override test passed.");
}

async fn test_multiple_users() {
    let ip1 = "4.4.4.4";
    let ip2 = "5.5.5.5";
    println!("  Testing with two IPs: {} and {}", ip1, ip2);
    println!("  Global rule: 15 req/s. Each IP has its own limit.");

    for i in 1..=15 {
        assert!(
            limit!(ip1, "/multi").await,
            "IP1 req {} should be allowed",
            i
        );
        assert!(
            limit!(ip2, "/multi").await,
            "IP2 req {} should be allowed",
            i
        );
    }

    println!("  Both IPs have used their 15 requests.");
    assert!(!limit!(ip1, "/multi").await, "IP1 should now be denied");
    assert!(!limit!(ip2, "/multi").await, "IP2 should now be denied");
    println!("  Multiple users test passed.");
}

async fn test_long_interval() {
    let ip = "6.6.6.6";
    println!("  Testing IP: {}", ip);
    println!("  Route /api/login rule: 3 req/min.");
    println!("  This test demonstrates combining route-specific and global limits.\n");

    println!("  Making 3 requests to /api/login...");
    assert!(limit!(ip, "/api/login").await);
    sleep(StdDuration::from_millis(100)).await;
    assert!(limit!(ip, "/api/login").await);
    sleep(StdDuration::from_millis(100)).await;
    assert!(limit!(ip, "/api/login").await);

    println!("  Making 4th request to /api/login, should be denied by route rule.");
    assert!(!limit!(ip, "/api/login").await);

    println!("  Route limit reached. Other paths still have global limit available.");
    for i in 1..=3 {
        assert!(limit!(ip, "/global-check").await);
        println!("  Request #{} to /global-check: Allowed", i);
    }

    println!("  Long interval test passed.");
}

async fn test_prefix_matching() {
    let ip = "7.7.7.7";
    println!("  Testing IP: {}", ip);
    println!("  Prefix route /api/users/ rule: 10 req/s (matches all sub-routes).");
    println!("  All requests to /api/users/* share the same prefix limit.\n");

    // Demonstrate that multiple different routes share the same limit
    let test_routes = [
        "/api/users/123/profile",
        "/api/users/456/settings",
        "/api/users/789/posts",
        "/api/users/999/followers",
        "/api/users/111/following",
        "/api/users/222/messages",
    ];

    for (i, route) in test_routes.iter().enumerate() {
        let allowed = limit!(ip, route).await;
        println!(
            "  Request #{} to {}: {}",
            i + 1,
            route,
            if allowed { "Allowed" } else { "Denied" }
        );
        assert!(allowed, "Request {} to {} should be allowed", i + 1, route);
    }

    println!("  All 6 requests shared the same /api/users/ prefix limit.");
    println!("  Prefix matching test passed.");
}

async fn test_method_specific() {
    let ip = "8.8.8.8";
    println!("  Testing IP: {}", ip);
    println!("  Route /api/data with method-specific rules:");
    println!("    - POST requests: 3 req/s");
    println!("    - GET requests: 10 req/s (no limit, uses default 5)");
    println!("    - Other methods: use global rule (5 req/s)\n");

    // Test POST requests
    println!("  Testing POST to /api/data:");
    for i in 1..=4 {
        let allowed = limit!(ip, "/api/data", HttpMethod::POST).await;
        println!(
            "    POST Request #{}: {}",
            i,
            if allowed { "Allowed" } else { "Denied" }
        );
        assert_eq!(allowed, i <= 3);
    }

    println!("  Testing GET to /api/data (different counter from POST):");
    for i in 1..=4 {
        let allowed = limit!(ip, "/api/data", HttpMethod::GET).await;
        println!(
            "    GET Request #{}: {}",
            i,
            if allowed { "Allowed" } else { "Denied" }
        );
        // GET has limit 10 from default rule (or uses it if no method rule defined)
        assert!(allowed);
    }

    println!("  HttpMethod-specific test passed.");
}

#[tokio::main]
async fn main() {
    println!("Starting lazy-limit demo...\n");

    init_rate_limiter!(
        default: RuleConfig::new(Duration::seconds(1), 15),
        max_memory: Some(64 * 1024 * 1024),
        routes: [
            ("/api/login", RuleConfig::new(Duration::minutes(1), 3)),
            ("/api/public", RuleConfig::new(Duration::seconds(1), 5)),
            ("/api/premium", RuleConfig::new(Duration::seconds(1), 20)),
            ("/api/users/", RuleConfig::new(Duration::seconds(1), 10).match_prefix(true)),
            ("/api/data", RuleConfig::new(Duration::seconds(1), 3).for_methods(vec![HttpMethod::POST])),
        ]
    )
    .await;

    println!("Rate limiter initialized with rules:");
    println!("  - Global: 15 requests/second");
    println!("  - /api/login: 3 requests/minute");
    println!("  - /api/public: 5 requests/second");
    println!("  - /api/premium: 20 requests/second");
    println!("  - /api/users/ (prefix): 10 requests/second (matches all sub-routes)");
    println!("  - /api/data (POST only): 3 requests/second");
    println!();

    println!("--- Test 1: Basic Global Rate Limiting ---");
    test_basic_limit().await;
    println!();

    println!("--- Test 2: Route-Specific Rules (with Global Limit) ---");
    test_route_specific().await;
    println!();

    println!("--- Test 3: Override Mode ---");
    test_override_mode().await;
    println!();

    println!("--- Test 4: Multiple Users ---");
    test_multiple_users().await;
    println!();

    println!("--- Test 5: Long Interval Rules ---");
    test_long_interval().await;
    println!();

    println!("--- Test 6: Prefix Matching ---");
    test_prefix_matching().await;
    println!();

    println!("--- Test 7: HttpMethod-Specific Rules ---");
    test_method_specific().await;
    println!();

    println!("All demo tests completed.");
}
