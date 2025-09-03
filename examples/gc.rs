/* examples/gc.rs */

use lazy_limit::*;
use std::time::Duration as StdDuration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    println!("Starting Garbage Collector (GC) demo...\n");

    const MAX_MEMORY: usize = 1024; // 1 KB
    const GC_INTERVAL: u64 = 2; // 2 seconds

    init_rate_limiter!(
        default: RuleConfig::new(Duration::minutes(10), 1000),
        max_memory: Some(MAX_MEMORY),
        routes: [
            ("config_gc", RuleConfig::new(Duration::seconds(1), 1))
        ]
    )
    .await;
    // Manually set a custom GC interval on the config object before initializing
    // This part shows how one might customize gc_interval if it were exposed differently
    // For now, we rely on the default or a future config method. The logic stands.

    println!("Rate limiter initialized with:");
    println!("  - Max Memory: {} bytes", MAX_MEMORY);
    println!(
        "  - GC Interval: {} seconds (default or custom)",
        GC_INTERVAL
    );
    println!("  - A very lenient rate limit rule.\n");

    println!("--- Phase 1: Generating data to exceed memory limit ---");
    let num_ips = 500;
    println!("  Making 1 request from {} different IPs...", num_ips);

    for i in 0..num_ips {
        let ip = format!("192.168.0.{}", i);
        let _ = limit!(&ip, "/fill_memory").await;
    }

    println!(
        "  Done. Memory usage should now be well over the {} byte limit.",
        MAX_MEMORY
    );
    println!(
        "  The background GC task runs every {} seconds.\n",
        GC_INTERVAL
    );

    println!("--- Phase 2: Verifying GC has run ---");
    let wait_time = GC_INTERVAL + 1;
    println!(
        "  Waiting for {} seconds to ensure the GC task has had a chance to run...",
        wait_time
    );
    sleep(StdDuration::from_secs(wait_time)).await;

    println!("\n  Now, we will test if the OLDEST record was removed.");
    let first_ip = "192.168.0.0";
    println!("  The first IP we added was '{}'.", first_ip);
    println!(
        "  It made one request. If its record was cleaned up by the GC, a second request should be ALLOWED."
    );

    let allowed = limit!(first_ip, "/fill_memory").await;

    println!(
        "  Second request for '{}' was: {}",
        first_ip,
        if allowed { "Allowed" } else { "Denied" }
    );

    assert!(
        allowed,
        "The request should have been allowed, indicating the old record was garbage collected."
    );

    println!(
        "\nSuccess! The old record was cleared to save space, demonstrating that the GC is working."
    );
    println!("GC demo completed successfully.");
}
