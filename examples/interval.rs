/* examples/interval.rs */

use lazy_limit::*;
use std::time::Duration as StdDuration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    println!("Starting interval-specific rate limit demo...\n");

    init_rate_limiter!(
        default: RuleConfig::new(Duration::seconds(2), 5),
        routes: [
            ("/long_task", RuleConfig::new(Duration::minutes(1), 3)),
        ]
    )
    .await;

    println!("Rate limiter initialized with rules:");
    println!("  - Default (Short Interval): 5 requests / 2 seconds");
    println!("  - /long_task (Long Interval): 3 requests / 1 minute");
    println!();

    println!("--- Test 1: Short Interval (Counter-based) ---");
    let ip_short = "10.0.0.1";
    println!(
        "  Hitting default route with IP {}. Limit is 5 req / 2 sec.",
        ip_short
    );

    for i in 1..=7 {
        let allowed = limit!(ip_short, "/any/path").await;
        println!(
            "  Request #{}: {}",
            i,
            if allowed { "Allowed" } else { "Denied" }
        );
        assert_eq!(allowed, i <= 5);
    }

    println!("\n  Waiting for 2 seconds for the window to reset...");
    sleep(StdDuration::from_secs(2)).await;

    let allowed_after_wait = limit!(ip_short, "/any/path").await;
    println!(
        "  Request after 2s wait: {}",
        if allowed_after_wait {
            "Allowed"
        } else {
            "Denied"
        }
    );
    assert!(allowed_after_wait);
    println!("  Short interval test passed.\n");

    println!("--- Test 2: Long Interval (Timestamp-based) ---");
    let ip_long = "20.0.0.1";
    println!(
        "  Hitting /long_task route with IP {}. Limit is 3 req / 1 min.",
        ip_long
    );

    println!("  Making 3 requests, which should be allowed...");
    assert!(limit!(ip_long, "/long_task").await);
    sleep(StdDuration::from_millis(100)).await;
    assert!(limit!(ip_long, "/long_task").await);
    sleep(StdDuration::from_millis(100)).await;
    assert!(limit!(ip_long, "/long_task").await);
    println!("  First 3 requests were allowed.");

    let fourth_request_allowed = limit!(ip_long, "/long_task").await;
    println!(
        "  Making 4th request: {}",
        if fourth_request_allowed {
            "Allowed"
        } else {
            "Denied"
        }
    );
    assert!(!fourth_request_allowed);

    println!("\n  Waiting for 60 seconds for the long interval window to pass...");
    sleep(StdDuration::from_secs(60)).await;

    let after_long_wait_allowed = limit!(ip_long, "/long_task").await;
    println!(
        "  Making request after 60s wait: {}",
        if after_long_wait_allowed {
            "Allowed"
        } else {
            "Denied"
        }
    );
    assert!(
        after_long_wait_allowed,
        "Request should be allowed after the 1-minute window expired."
    );

    println!("\n  This demonstrates the long-interval logic is working.");
    println!("  Long interval test passed.\n");
    println!("All interval tests completed successfully.");
}
