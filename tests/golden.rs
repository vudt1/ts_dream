//! Golden runner — reads a golden file, runs it against a live Rust server,
//! and diffs byte-exact. Skipped automatically when the server isn't running
//! (no `GOLDEN_ADDR`); set `GOLDEN_ADDR=127.0.0.1:6414` to enable.

use std::sync::Once;

static INIT: Once = Once::new();

fn init() {
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::ERROR).try_init();
    });
}

fn server_addr() -> Option<String> {
    std::env::var("GOLDEN_ADDR").ok()
}

/// Parse + validate a golden file (unit of the harness, no socket needed).
#[tokio::test]
async fn golden_parse_and_validate_format() {
    init();
    let text = r#"// login success
<<F444010000
<<F4440A000100000
>>F4440300010901
>>F44402000106

>>F4440300010300
"#;
    let g = ts_dream::harness::Golden::parse(text, "login").expect("parse ok");
    assert_eq!(g.c2s.len(), 2);
    assert_eq!(g.s2c.len(), 3);
    // Case is normalised to uppercase.
    assert!(g.c2s[0].starts_with("F444"));
}

/// Run against a live server if `GOLDEN_ADDR` is set. This is the CI/parity
/// gate once captures exist; otherwise skipped.
#[tokio::test]
async fn golden_driven_scenarios_gate() {
    init();
    let Some(addr) = server_addr() else {
        eprintln!("GOLDEN_ADDR unset — skipping golden gate");
        return;
    };
    // Only run the login-scaffold golden that ships with the harness; the
    // full suite (tickets 23) is a separate gate.
    let text = "\
<<F444010000
>>F4440300010901
";
    let g = ts_dream::harness::Golden::parse(text, "login-hello").unwrap();
    let received = ts_dream::harness::run_golden(&g, &addr).await;
    assert!(received.is_ok(), "golden failed: {:?}", received);
}