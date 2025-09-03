/* examples/mix.rs */

use lazy_limit::*;
use std::time::Duration as StdDuration;
use tokio::time::sleep;

async fn test_user_default(ip: &str) {
    println!(
        "  -> Testing Default User (IP: {}). Limit: 5 req / 2 sec.",
        ip
    );
    for i in 1..=6 {
        let allowed = limit!(ip, "/some/default/path").await;
        assert_eq!(
            allowed,
            i <= 5,
            "Default user check failed at request #{}",
            i
        );
    }
    println!("  -> Default User limit reached as expected.");
}

async fn test_user_api(ip: &str) {
    println!("  -> Testing API User (IP: {}). Limit: 3 req / 1 sec.", ip);
    for i in 1..=4 {
        let allowed = limit!(ip, "/api/user").await;
        assert_eq!(allowed, i <= 3, "API user check failed at request #{}", i);
    }
    println!("  -> API User limit reached as expected.");
}

async fn test_user_upload(ip: &str) {
    println!(
        "  -> Testing Upload User (IP: {}). Limit: 2 req / 5 sec.",
        ip
    );
    for i in 1..=3 {
        let allowed = limit!(ip, "/api/upload").await;
        assert_eq!(
            allowed,
            i <= 2,
            "Upload user check failed at request #{}",
            i
        );
    }
    println!("  -> Upload User limit reached as expected.");
}

async fn test_user_job(ip: &str) {
    println!("  -> Testing Job User (IP: {}). Limit: 4 req / 1 min.", ip);
    for i in 1..=5 {
        let allowed = limit!(ip, "/api/job").await;
        assert_eq!(allowed, i <= 4, "Job user check failed at request #{}", i);
    }
    println!("  -> Job User limit reached as expected.");
}

#[tokio::main]
async fn main() {
    println!("Starting mixed concurrent rate limit demo...\n");

    init_rate_limiter!(
        default: RuleConfig::new(Duration::seconds(2), 5),      // short
        routes: [
            ("/api/user", RuleConfig::new(Duration::seconds(1), 3)),  // very short
            ("/api/upload", RuleConfig::new(Duration::seconds(5), 2)),// medium
            ("/api/job", RuleConfig::new(Duration::minutes(1), 4)), // long
        ]
    )
    .await;

    println!("Rate limiter initialized with mixed rules.\n");

    let (ip_default, ip_user, ip_upload, ip_job) = ("1.1.1.1", "2.2.2.2", "3.3.3.3", "4.4.4.4");

    println!("--- Phase 1: Concurrently hitting all limits ---");
    tokio::join!(
        test_user_default(ip_default),
        test_user_api(ip_user),
        test_user_upload(ip_upload),
        test_user_job(ip_job)
    );
    println!("\n--- Phase 1 Passed: All users correctly hit their individual limits. ---\n");

    let longest_cooldown = 61;
    println!(
        "--- Phase 2: Waiting {} seconds for the longest cooldown (1 minute) to expire... ---",
        longest_cooldown
    );
    sleep(StdDuration::from_secs(longest_cooldown)).await;
    println!("\n--- Cooldown finished. All limits should now be reset. ---\n");

    println!("--- Phase 3: Re-running concurrent tests to verify limits have reset ---");
    tokio::join!(
        test_user_default(ip_default),
        test_user_api(ip_user),
        test_user_upload(ip_upload),
        test_user_job(ip_job)
    );
    println!(
        "\n--- Phase 3 Passed: All users correctly hit their limits again after cooldown. ---\n"
    );

    println!("All mixed concurrency tests completed successfully!");
}
