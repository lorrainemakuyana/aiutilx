mod common;
use common::{run, run_json_forced};

// These tests resolve `localhost`, which hickory answers from the system hosts
// file without touching the network — so they are deterministic offline.

#[test]
fn dnsx_resolves_localhost() {
    let out = run_json_forced("dnsx", &["localhost"]);
    assert_eq!(out["query"], "localhost");
    assert!(out["resolver"].is_string());
    assert!(out["count"].as_u64().unwrap() >= 1);
    assert!(out["elapsed_ms"].is_u64());

    let recs = out["records"].as_array().expect("records array");
    assert!(
        recs.iter().any(|r| r["type"] == "A" && r["value"] == "127.0.0.1"),
        "expected an A record for 127.0.0.1, got: {}",
        out["records"]
    );
    // TTL is an integer, not a formatted string.
    assert!(recs[0]["ttl"].is_u64());
}

#[test]
fn dnsx_multiple_types() {
    let out = run_json_forced("dnsx", &["localhost", "--type", "A,AAAA"]);
    let types = out["record_types"].as_array().unwrap();
    assert!(types.iter().any(|t| t == "A"));
    assert!(types.iter().any(|t| t == "AAAA"));

    let recs = out["records"].as_array().unwrap();
    assert!(recs.iter().any(|r| r["type"] == "A" && r["value"] == "127.0.0.1"));
    assert!(recs.iter().any(|r| r["type"] == "AAAA" && r["value"] == "::1"));
}

#[test]
fn dnsx_unknown_type_is_structured_error() {
    let out = run("dnsx", &["example.com", "--type", "BOGUS"]);
    assert!(!out.status.success(), "unknown record type should exit non-zero");
    let v: serde_json::Value =
        serde_json::from_slice(&out.stderr).expect("structured error on stderr");
    assert!(v["error"].as_str().unwrap().contains("BOGUS"));
    assert_eq!(v["query"], "example.com");
}

#[test]
fn dnsx_reverse_requires_valid_ip() {
    let out = run("dnsx", &["not-an-ip", "--reverse"]);
    assert!(!out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stderr).expect("structured error");
    assert!(v["error"].as_str().unwrap().contains("IP"));
}
