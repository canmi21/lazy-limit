/* examples/demo.rs */

use lazy_limit::*;
use std::time::Duration as StdDuration;
use tokio::time::sleep;

async fn test_basic_limit() {
    let ip = "1.1.1.1";
    println!("  Testing IP: {}", ip);
    println!("  Global rule: 5 req/s. Should allow 5, then deny 6th.");

    for i in 1..=7 {
        let allowed = limit!(ip, "/some/path").await;
        println!(
            "  Request #{}: {}",
            i,
            if allowed { "Allowed" } else { "Denied" }
        );
        assert_eq!(allowed, i <= 5);
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
    println!("  Global rule: 5 req/s. Route /api/public rule: 10 req/s.");
    println!("  Effective limit for /api/public is min(5, 10) = 5 req/s.");

    for i in 1..=6 {
        let allowed = limit!(ip, "/api/public").await;
        println!(
            "  Request #{} to /api/public: {}",
            i,
            if allowed { "Allowed" } else { "Denied" }
        );
        assert_eq!(allowed, i <= 5);
    }

    println!("  Global limit for {} should now be reached.", ip);
    let allowed = limit!(ip, "/another/path").await;
    println!(
        "  Request to /another/path: {}",
        if allowed { "Allowed" } else { "Denied" }
    );
    assert!(!allowed);

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
    println!("  Global rule: 5 req/s. Each IP has its own limit.");

    for i in 1..=5 {
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

    println!("  Both IPs have used their 5 requests.");
    assert!(!limit!(ip1, "/multi").await, "IP1 should now be denied");
    assert!(!limit!(ip2, "/multi").await, "IP2 should now be denied");
    println!("  Multiple users test passed.");
}

async fn test_long_interval() {
    let ip = "6.6.6.6";
    println!("  Testing IP: {}", ip);
    println!("  Route /api/login rule: 3 req/min.");

    println!("  Making 3 requests to /api/login...");
    assert!(limit!(ip, "/api/login").await);
    sleep(StdDuration::from_millis(100)).await;
    assert!(limit!(ip, "/api/login").await);
    sleep(StdDuration::from_millis(100)).await;
    assert!(limit!(ip, "/api/login").await);

    println!("  Making 4th request, should be denied by route rule.");
    assert!(!limit!(ip, "/api/login").await);

    println!("  Checking global limit for {}...", ip);
    assert!(limit!(ip, "/global-check").await); // 4th global req
    assert!(limit!(ip, "/global-check").await); // 5th global req
    assert!(
        !limit!(ip, "/global-check").await,
        "6th global req should be denied"
    );

    println!("  Long interval test passed.");
}

#[tokio::main]
async fn main() {
    println!("Starting lazy-limit demo...\n");

    init_rate_limiter!(
        default: RuleConfig::new(Duration::seconds(1), 5),
        max_memory: Some(64 * 1024 * 1024),
        routes: [
            ("/api/login", RuleConfig::new(Duration::minutes(1), 3)),
            ("/api/public", RuleConfig::new(Duration::seconds(1), 10)),
            ("/api/premium", RuleConfig::new(Duration::seconds(1), 20)),
        ]
    )
    .await;

    println!("Rate limiter initialized with rules:");
    println!("  - Global: 5 requests/second");
    println!("  - /api/login: 3 requests/minute");
    println!("  - /api/public: 10 requests/second");
    println!("  - /api/premium: 20 requests/second");
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

    println!("All demo tests completed.");
}
