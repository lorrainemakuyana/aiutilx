mod common;
use common::run_json_forced;

// These tests run against the unix-x repo itself, which is always a git repo
// with at least one commit. Path "." resolves to the workspace root at test time.

#[test]
fn gitx_log_returns_commits() {
    let output = run_json_forced("gitx", &["log", "."]);
    assert!(output["commits"].is_array(), "commits must be an array");
    assert!(
        output["count"].as_u64().unwrap_or(0) > 0,
        "repo must have at least one commit"
    );
    assert!(output["repo"].is_string());
}

#[test]
fn gitx_log_commit_fields() {
    let output = run_json_forced("gitx", &["log", ".", "-n", "1"]);
    let commit = &output["commits"][0];
    assert_eq!(commit["short_hash"].as_str().unwrap().len(), 7);
    assert_eq!(commit["hash"].as_str().unwrap().len(), 40);
    assert!(commit["author_name"].is_string());
    assert!(commit["author_email"].is_string());
    assert!(commit["author_time"].is_number());
    assert!(commit["commit_time"].is_number());
    assert!(commit["summary"].is_string());
    assert!(commit["is_merge"].is_boolean());
    assert!(commit["parents"].is_array());
}

#[test]
fn gitx_log_limit_respected() {
    let output = run_json_forced("gitx", &["log", ".", "-n", "3"]);
    let count = output["commits"].as_array().unwrap().len();
    assert!(count <= 3, "limit=3 must return at most 3 commits, got {}", count);
    assert_eq!(output["count"].as_u64().unwrap() as usize, count);
}

#[test]
fn gitx_log_no_merges_flag() {
    let output = run_json_forced("gitx", &["log", ".", "--no-merges"]);
    for commit in output["commits"].as_array().unwrap() {
        assert_eq!(
            commit["is_merge"].as_bool().unwrap(),
            false,
            "is_merge must be false when --no-merges is set"
        );
    }
}

#[test]
fn gitx_status_structure() {
    let output = run_json_forced("gitx", &["status", "."]);
    assert!(output["repo"].is_string());
    assert!(output["clean"].is_boolean());
    assert!(output["entries"].is_array());
    assert!(output["staged"].is_number());
    assert!(output["unstaged"].is_number());
    assert!(output["untracked"].is_number());
}

#[test]
fn gitx_status_branch_present() {
    let output = run_json_forced("gitx", &["status", "."]);
    // branch may be null on detached HEAD, but the key must exist
    assert!(output.get("branch").is_some());
}

#[test]
fn gitx_branches_returns_at_least_one() {
    // Use --all so remote-tracking refs are included; CI checkouts are detached HEAD
    // with no local branches but always have at least one remote ref (e.g. origin/main).
    let output = run_json_forced("gitx", &["branches", ".", "--all"]);
    assert!(output["branches"].is_array());
    assert!(
        output["count"].as_u64().unwrap_or(0) >= 1,
        "must have at least one branch"
    );
    assert!(output["repo"].is_string());
}

#[test]
fn gitx_branches_fields() {
    // Use --all so remote-tracking refs are included in CI detached HEAD checkouts.
    let output = run_json_forced("gitx", &["branches", ".", "--all"]);
    let branch = &output["branches"][0];
    assert!(branch["name"].is_string());
    assert!(branch["current"].is_boolean());
    assert!(branch["remote"].is_boolean());
    assert!(branch["commit"].is_string());
    assert_eq!(branch["commit"].as_str().unwrap().len(), 7);
    assert!(branch["commit_time"].is_number());
    assert!(branch["summary"].is_string());
}

#[test]
fn gitx_branches_current_first() {
    let output = run_json_forced("gitx", &["branches", "."]);
    let branches = output["branches"].as_array().unwrap();
    if branches.len() > 1 {
        // If any branch is current, it must be first
        let has_current = branches.iter().any(|b| b["current"].as_bool().unwrap_or(false));
        if has_current {
            assert!(
                branches[0]["current"].as_bool().unwrap_or(false),
                "current branch must sort first"
            );
        }
    }
}

#[test]
fn gitx_stash_structure() {
    let output = run_json_forced("gitx", &["stash", "."]);
    assert!(output["repo"].is_string());
    assert!(output["count"].is_number());
    assert!(output["entries"].is_array());
    let count = output["count"].as_u64().unwrap() as usize;
    assert_eq!(output["entries"].as_array().unwrap().len(), count);
}

#[test]
fn gitx_tags_structure() {
    let output = run_json_forced("gitx", &["tags", "."]);
    assert!(output["repo"].is_string());
    assert!(output["count"].is_number());
    assert!(output["tags"].is_array());
    let count = output["count"].as_u64().unwrap() as usize;
    assert_eq!(output["tags"].as_array().unwrap().len(), count);
}

#[test]
fn gitx_tags_limit() {
    // Even if there are no tags, limit=1 should return at most 1
    let output = run_json_forced("gitx", &["tags", ".", "-n", "1"]);
    let count = output["tags"].as_array().unwrap().len();
    assert!(count <= 1, "limit=1 must return at most 1 tag, got {}", count);
}
