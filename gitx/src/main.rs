use clap::{Parser, Subcommand};
use git2::{BranchType, Repository, StatusOptions};
use serde::Serialize;
use std::path::PathBuf;
use ux_output::{emit, OutMode};

#[derive(Parser)]
#[command(
    name = "gitx",
    about = "Structured git inspection for AI agents.\nReplaces: git log, git status, git branch, git stash, git tag",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Structured commit history (replaces: git log)
    Log {
        /// Path to git repository
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Maximum number of commits to return
        #[arg(short = 'n', long, default_value = "50")]
        limit: usize,
        /// Filter by author name or email (substring match)
        #[arg(long)]
        author: Option<String>,
        /// Filter commits whose message contains this string
        #[arg(long)]
        grep: Option<String>,
        /// Exclude merge commits
        #[arg(long)]
        no_merges: bool,
        /// Output mode: auto, json, pretty, table, ndjson
        #[arg(short, long, default_value = "auto")]
        out: String,
    },
    /// Working tree and index state (replaces: git status)
    Status {
        /// Path to git repository
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Output mode: auto, json, pretty, table, ndjson
        #[arg(short, long, default_value = "auto")]
        out: String,
    },
    /// Branch listing with tracking info (replaces: git branch -vv)
    Branches {
        /// Path to git repository
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Show remote branches only
        #[arg(short, long)]
        remote: bool,
        /// Show both local and remote branches
        #[arg(short, long)]
        all: bool,
        /// Output mode: auto, json, pretty, table, ndjson
        #[arg(short, long, default_value = "auto")]
        out: String,
    },
    /// Stash list (replaces: git stash list)
    Stash {
        /// Path to git repository
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Output mode: auto, json, pretty, table, ndjson
        #[arg(short, long, default_value = "auto")]
        out: String,
    },
    /// Tag listing (replaces: git tag -l)
    Tags {
        /// Path to git repository
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Maximum number of tags to return (0 = all)
        #[arg(short = 'n', long, default_value = "0")]
        limit: usize,
        /// Output mode: auto, json, pretty, table, ndjson
        #[arg(short, long, default_value = "auto")]
        out: String,
    },
}

// ── Output types ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct LogOutput {
    repo: String,
    branch: Option<String>,
    count: usize,
    commits: Vec<CommitEntry>,
}

#[derive(Serialize)]
struct CommitEntry {
    hash: String,
    short_hash: String,
    author_name: String,
    author_email: String,
    author_time: i64,
    committer_name: String,
    committer_email: String,
    commit_time: i64,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
    parents: Vec<String>,
    is_merge: bool,
}

#[derive(Serialize)]
struct StatusOutput {
    repo: String,
    branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    upstream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ahead: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    behind: Option<usize>,
    clean: bool,
    staged: usize,
    unstaged: usize,
    untracked: usize,
    entries: Vec<StatusEntry>,
}

#[derive(Serialize)]
struct StatusEntry {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    old_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    index_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workdir_status: Option<String>,
}

#[derive(Serialize)]
struct BranchesOutput {
    repo: String,
    current: Option<String>,
    count: usize,
    branches: Vec<BranchEntry>,
}

#[derive(Serialize)]
struct BranchEntry {
    name: String,
    current: bool,
    remote: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    upstream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ahead: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    behind: Option<usize>,
    commit: String,
    commit_time: i64,
    summary: String,
}

#[derive(Serialize)]
struct StashOutput {
    repo: String,
    count: usize,
    entries: Vec<StashEntry>,
}

#[derive(Serialize)]
struct StashEntry {
    index: usize,
    message: String,
    commit: String,
    time: i64,
}

#[derive(Serialize)]
struct TagsOutput {
    repo: String,
    count: usize,
    tags: Vec<TagEntry>,
}

#[derive(Serialize)]
struct TagEntry {
    name: String,
    commit: String,
    commit_time: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tagger_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tagger_time: Option<i64>,
    is_annotated: bool,
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Command::Log {
            path,
            limit,
            author,
            grep,
            no_merges,
            out,
        } => run_log(
            path,
            *limit,
            author.as_deref(),
            grep.as_deref(),
            *no_merges,
            out,
        ),
        Command::Status { path, out } => run_status(path, out),
        Command::Branches {
            path,
            remote,
            all,
            out,
        } => run_branches(path, *remote, *all, out),
        Command::Stash { path, out } => run_stash(path, out),
        Command::Tags { path, limit, out } => run_tags(path, *limit, out),
    };

    if let Err(e) = result {
        let msg = serde_json::json!({"error": e.message()});
        eprintln!("{}", serde_json::to_string(&msg).unwrap());
        std::process::exit(1);
    }
}

fn open_repo(path: &PathBuf) -> Result<Repository, git2::Error> {
    Repository::discover(path)
}

fn repo_workdir(repo: &Repository) -> String {
    repo.workdir()
        .map(|p| p.to_string_lossy().trim_end_matches('/').to_string())
        .unwrap_or_else(|| ".".to_string())
}

fn head_branch(repo: &Repository) -> Option<String> {
    repo.head()
        .ok()
        .filter(|h| h.is_branch())
        .and_then(|h| h.shorthand().map(|s| s.to_string()))
}

// ── log ───────────────────────────────────────────────────────────────────────

fn run_log(
    path: &PathBuf,
    limit: usize,
    author: Option<&str>,
    grep: Option<&str>,
    no_merges: bool,
    out: &str,
) -> Result<(), git2::Error> {
    let repo = open_repo(path)?;
    let repo_str = repo_workdir(&repo);
    let branch = head_branch(&repo);
    let mode = OutMode::from_str(out);

    let mut revwalk = repo.revwalk()?;
    // push_head fails on empty repos — treat that as zero commits
    let _ = revwalk.push_head();
    revwalk.set_sorting(git2::Sort::TIME)?;

    let grep_lower = grep.map(|g| g.to_lowercase());
    let author_lower = author.map(|a| a.to_lowercase());

    let mut commits: Vec<CommitEntry> = Vec::new();

    for oid_result in revwalk {
        if commits.len() >= limit {
            break;
        }
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        if no_merges && commit.parent_count() > 1 {
            continue;
        }

        let message = commit.message().unwrap_or("").to_string();
        let summary = commit.summary().unwrap_or("").to_string();

        if let Some(ref g) = grep_lower {
            if !message.to_lowercase().contains(g.as_str()) {
                continue;
            }
        }

        let author_sig = commit.author();
        let author_name = author_sig.name().unwrap_or("").to_string();
        let author_email = author_sig.email().unwrap_or("").to_string();

        if let Some(ref a) = author_lower {
            if !author_name.to_lowercase().contains(a.as_str())
                && !author_email.to_lowercase().contains(a.as_str())
            {
                continue;
            }
        }

        let committer = commit.committer();
        let hash = oid.to_string();
        let short_hash = hash[..7].to_string();

        // Body is everything after the first blank line
        let body: Option<String> = {
            let rest: String = message.lines().skip(2).collect::<Vec<_>>().join("\n");
            let trimmed = rest.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        };

        let parents: Vec<String> = commit
            .parents()
            .map(|p| p.id().to_string()[..7].to_string())
            .collect();

        commits.push(CommitEntry {
            hash,
            short_hash,
            author_name,
            author_email,
            author_time: author_sig.when().seconds(),
            committer_name: committer.name().unwrap_or("").to_string(),
            committer_email: committer.email().unwrap_or("").to_string(),
            commit_time: committer.when().seconds(),
            summary,
            body,
            parents,
            is_merge: commit.parent_count() > 1,
        });
    }

    let count = commits.len();
    let output = LogOutput {
        repo: repo_str,
        branch,
        count,
        commits,
    };

    if out == "table" {
        print_log_table(&output);
    } else if mode == OutMode::Ndjson {
        for c in &output.commits {
            println!("{}", serde_json::to_string(c).unwrap());
        }
    } else {
        emit(&output, &mode);
    }
    Ok(())
}

// ── status ────────────────────────────────────────────────────────────────────

fn run_status(path: &PathBuf, out: &str) -> Result<(), git2::Error> {
    let repo = open_repo(path)?;
    let repo_str = repo_workdir(&repo);
    let branch = head_branch(&repo);
    let mode = OutMode::from_str(out);

    // Resolve upstream + ahead/behind for the current branch
    let (upstream, ahead, behind) = branch
        .as_deref()
        .and_then(|b| {
            let local_branch = repo.find_branch(b, BranchType::Local).ok()?;
            let up = local_branch.upstream().ok()?;
            let upstream_name = up.name().ok()??.to_string();
            let local_oid = local_branch.get().target()?;
            let up_oid = up.get().target()?;
            let (a, b) = repo.graph_ahead_behind(local_oid, up_oid).ok()?;
            Some((upstream_name, a, b))
        })
        .map(|(u, a, b)| (Some(u), Some(a), Some(b)))
        .unwrap_or((None, None, None));

    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);

    let statuses = repo.statuses(Some(&mut opts))?;

    let mut entries: Vec<StatusEntry> = Vec::new();
    let (mut staged, mut unstaged, mut untracked) = (0usize, 0usize, 0usize);

    for entry in statuses.iter() {
        let s = entry.status();
        let path_str = entry.path().unwrap_or("").to_string();

        let old_path = entry
            .head_to_index()
            .and_then(|d| d.old_file().path())
            .map(|p| p.to_string_lossy().to_string());

        let index_status = map_index_status(s);
        let workdir_status = map_workdir_status(s);

        if index_status.is_some() {
            staged += 1;
        }
        if let Some(ref ws) = workdir_status {
            if ws == "untracked" {
                untracked += 1;
            } else {
                unstaged += 1;
            }
        }

        entries.push(StatusEntry {
            path: path_str,
            old_path,
            index_status,
            workdir_status,
        });
    }

    let clean = entries.is_empty();
    let output = StatusOutput {
        repo: repo_str,
        branch,
        upstream,
        ahead,
        behind,
        clean,
        staged,
        unstaged,
        untracked,
        entries,
    };

    if out == "table" {
        print_status_table(&output);
    } else if mode == OutMode::Ndjson {
        for e in &output.entries {
            println!("{}", serde_json::to_string(e).unwrap());
        }
    } else {
        emit(&output, &mode);
    }
    Ok(())
}

fn map_index_status(s: git2::Status) -> Option<String> {
    if s.contains(git2::Status::INDEX_NEW) {
        Some("added".into())
    } else if s.contains(git2::Status::INDEX_MODIFIED) {
        Some("modified".into())
    } else if s.contains(git2::Status::INDEX_DELETED) {
        Some("deleted".into())
    } else if s.contains(git2::Status::INDEX_RENAMED) {
        Some("renamed".into())
    } else if s.contains(git2::Status::INDEX_TYPECHANGE) {
        Some("typechange".into())
    } else {
        None
    }
}

fn map_workdir_status(s: git2::Status) -> Option<String> {
    if s.contains(git2::Status::WT_NEW) {
        Some("untracked".into())
    } else if s.contains(git2::Status::WT_MODIFIED) {
        Some("modified".into())
    } else if s.contains(git2::Status::WT_DELETED) {
        Some("deleted".into())
    } else if s.contains(git2::Status::WT_RENAMED) {
        Some("renamed".into())
    } else if s.contains(git2::Status::WT_TYPECHANGE) {
        Some("typechange".into())
    } else if s.contains(git2::Status::CONFLICTED) {
        Some("conflicted".into())
    } else {
        None
    }
}

// ── branches ──────────────────────────────────────────────────────────────────

fn run_branches(path: &PathBuf, remote: bool, all: bool, out: &str) -> Result<(), git2::Error> {
    let repo = open_repo(path)?;
    let repo_str = repo_workdir(&repo);
    let current = head_branch(&repo);
    let mode = OutMode::from_str(out);

    let branch_type = if all {
        None
    } else if remote {
        Some(BranchType::Remote)
    } else {
        Some(BranchType::Local)
    };

    let mut branches: Vec<BranchEntry> = Vec::new();

    for item in repo.branches(branch_type)? {
        let (branch, btype) = item?;
        let name = match branch.name()? {
            Some(n) => n.to_string(),
            None => continue,
        };

        let is_remote = btype == BranchType::Remote;
        let is_current = !is_remote && current.as_deref() == Some(&name);

        let tip = match branch.get().peel_to_commit() {
            Ok(c) => c,
            Err(_) => continue,
        };
        let commit = tip.id().to_string()[..7].to_string();
        let commit_time = tip.committer().when().seconds();
        let summary = tip.summary().unwrap_or("").to_string();

        let (upstream, ahead, behind) = if !is_remote {
            branch
                .upstream()
                .ok()
                .and_then(|up| {
                    let upstream_name = up.name().ok()??.to_string();
                    let local_oid = branch.get().target()?;
                    let up_oid = up.get().target()?;
                    let (a, b) = repo.graph_ahead_behind(local_oid, up_oid).ok()?;
                    Some((upstream_name, a, b))
                })
                .map(|(u, a, b)| (Some(u), Some(a), Some(b)))
                .unwrap_or((None, None, None))
        } else {
            (None, None, None)
        };

        branches.push(BranchEntry {
            name,
            current: is_current,
            remote: is_remote,
            upstream,
            ahead,
            behind,
            commit,
            commit_time,
            summary,
        });
    }

    // Current branch first, then alphabetical
    branches.sort_by(|a, b| b.current.cmp(&a.current).then(a.name.cmp(&b.name)));

    let count = branches.len();
    let output = BranchesOutput {
        repo: repo_str,
        current,
        count,
        branches,
    };

    if out == "table" {
        print_branches_table(&output);
    } else if mode == OutMode::Ndjson {
        for b in &output.branches {
            println!("{}", serde_json::to_string(b).unwrap());
        }
    } else {
        emit(&output, &mode);
    }
    Ok(())
}

// ── stash ─────────────────────────────────────────────────────────────────────

fn run_stash(path: &PathBuf, out: &str) -> Result<(), git2::Error> {
    let mut repo = open_repo(path)?;
    let repo_str = repo_workdir(&repo);
    let mode = OutMode::from_str(out);

    // Collect raw stash entries first (closure borrows &mut repo)
    let mut raw: Vec<(usize, String, git2::Oid)> = Vec::new();
    repo.stash_foreach(|index, message, oid| {
        raw.push((index, message.to_string(), *oid));
        true
    })?;

    // Look up commit times after stash_foreach releases the mutable borrow
    let entries: Vec<StashEntry> = raw
        .into_iter()
        .map(|(index, message, oid)| {
            let time = repo
                .find_commit(oid)
                .map(|c| c.committer().when().seconds())
                .unwrap_or(0);
            StashEntry {
                index,
                message,
                commit: oid.to_string()[..7].to_string(),
                time,
            }
        })
        .collect();

    let count = entries.len();
    let output = StashOutput {
        repo: repo_str,
        count,
        entries,
    };

    if out == "table" {
        print_stash_table(&output);
    } else if mode == OutMode::Ndjson {
        for e in &output.entries {
            println!("{}", serde_json::to_string(e).unwrap());
        }
    } else {
        emit(&output, &mode);
    }
    Ok(())
}

// ── tags ──────────────────────────────────────────────────────────────────────

fn run_tags(path: &PathBuf, limit: usize, out: &str) -> Result<(), git2::Error> {
    let repo = open_repo(path)?;
    let repo_str = repo_workdir(&repo);
    let mode = OutMode::from_str(out);

    let mut tags: Vec<TagEntry> = Vec::new();

    repo.tag_foreach(|oid, name_bytes| {
        let name = std::str::from_utf8(name_bytes)
            .unwrap_or("")
            .trim_start_matches("refs/tags/")
            .to_string();

        // Annotated tag: peel to commit via the tag object
        if let Ok(tag_obj) = repo.find_tag(oid) {
            let target_oid = tag_obj.target_id();
            let commit_time = repo
                .find_commit(target_oid)
                .map(|c| c.committer().when().seconds())
                .unwrap_or(0);
            let message = tag_obj
                .message()
                .map(|m| m.trim().to_string())
                .filter(|m| !m.is_empty());
            let tagger = tag_obj.tagger();
            tags.push(TagEntry {
                name,
                commit: target_oid.to_string()[..7].to_string(),
                commit_time,
                message,
                tagger_name: tagger
                    .as_ref()
                    .and_then(|t| t.name())
                    .map(|s| s.to_string()),
                tagger_time: tagger.as_ref().map(|t| t.when().seconds()),
                is_annotated: true,
            });
        } else if let Ok(commit) = repo.find_commit(oid) {
            // Lightweight tag pointing directly at a commit
            tags.push(TagEntry {
                name,
                commit: oid.to_string()[..7].to_string(),
                commit_time: commit.committer().when().seconds(),
                message: None,
                tagger_name: None,
                tagger_time: None,
                is_annotated: false,
            });
        }
        // Non-commit tags (rare) are silently skipped
        true
    })?;

    // Most recent commits first
    tags.sort_by(|a, b| b.commit_time.cmp(&a.commit_time));
    if limit > 0 {
        tags.truncate(limit);
    }

    let count = tags.len();
    let output = TagsOutput {
        repo: repo_str,
        count,
        tags,
    };

    if out == "table" {
        print_tags_table(&output);
    } else if mode == OutMode::Ndjson {
        for t in &output.tags {
            println!("{}", serde_json::to_string(t).unwrap());
        }
    } else {
        emit(&output, &mode);
    }
    Ok(())
}

// ── table printers ────────────────────────────────────────────────────────────

fn print_log_table(output: &LogOutput) {
    let branch_str = output.branch.as_deref().unwrap_or("(detached HEAD)");
    println!(
        "repo: {}  branch: {}  commits: {}",
        output.repo, branch_str, output.count
    );
    println!("{}", "-".repeat(100));
    println!("{:<9} {:<22} {:<12} MESSAGE", "HASH", "AUTHOR", "DATE");
    println!("{}", "-".repeat(100));
    for c in &output.commits {
        let date = format_epoch(c.commit_time);
        let author = truncate(&c.author_name, 20);
        let summary = truncate(&c.summary, 55);
        let merge_mark = if c.is_merge { "⎇ " } else { "  " };
        println!(
            "{:<9} {:<22} {:<12} {}{}",
            c.short_hash, author, date, merge_mark, summary
        );
    }
    println!("{}", "-".repeat(100));
}

fn print_status_table(output: &StatusOutput) {
    let branch_str = output.branch.as_deref().unwrap_or("(detached HEAD)");
    let upstream_str = match (&output.upstream, output.ahead, output.behind) {
        (Some(u), Some(a), Some(b)) => format!(" → {} (+{} -{})", u, a, b),
        (Some(u), _, _) => format!(" → {}", u),
        _ => String::new(),
    };
    println!("branch: {}{}", branch_str, upstream_str);
    if output.clean {
        println!("working tree clean");
        return;
    }
    println!(
        "staged: {}  unstaged: {}  untracked: {}",
        output.staged, output.unstaged, output.untracked
    );
    println!("{}", "-".repeat(70));
    for e in &output.entries {
        let idx = e.index_status.as_deref().map(status_char).unwrap_or(' ');
        let wt = e.workdir_status.as_deref().map(status_char).unwrap_or(' ');
        let rename = e
            .old_path
            .as_ref()
            .map(|op| format!(" (from {})", op))
            .unwrap_or_default();
        println!(" {}{} {}{}", idx, wt, e.path, rename);
    }
}

fn print_branches_table(output: &BranchesOutput) {
    println!("repo: {}  branches: {}", output.repo, output.count);
    println!("{}", "-".repeat(90));
    println!(
        "  {:<35} {:<30} {:>6} {:>7}  {:<9} MESSAGE",
        "BRANCH", "UPSTREAM", "AHEAD", "BEHIND", "COMMIT"
    );
    println!("{}", "-".repeat(90));
    for b in &output.branches {
        let marker = if b.current { "* " } else { "  " };
        let upstream = b.upstream.as_deref().unwrap_or("-");
        let ahead = b.ahead.map(|n| n.to_string()).unwrap_or_else(|| "-".into());
        let behind = b
            .behind
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".into());
        let remote_tag = if b.remote { " [remote]" } else { "" };
        println!(
            "{}{:<35} {:<30} {:>6} {:>7}  {:<9} {}{}",
            marker,
            truncate(&b.name, 34),
            truncate(upstream, 29),
            ahead,
            behind,
            b.commit,
            truncate(&b.summary, 30),
            remote_tag,
        );
    }
}

fn print_stash_table(output: &StashOutput) {
    println!("repo: {}  stash entries: {}", output.repo, output.count);
    if output.entries.is_empty() {
        println!("(no stash entries)");
        return;
    }
    println!("{}", "-".repeat(80));
    println!("{:<6} {:<12} {:<12} MESSAGE", "INDEX", "COMMIT", "DATE");
    println!("{}", "-".repeat(80));
    for e in &output.entries {
        println!(
            "{:<6} {:<12} {:<12} {}",
            e.index,
            e.commit,
            format_epoch(e.time),
            truncate(&e.message, 45),
        );
    }
}

fn print_tags_table(output: &TagsOutput) {
    println!("repo: {}  tags: {}", output.repo, output.count);
    if output.tags.is_empty() {
        println!("(no tags)");
        return;
    }
    println!("{}", "-".repeat(80));
    println!(
        "{:<25} {:<9} {:<12} {:<5} MESSAGE",
        "TAG", "COMMIT", "DATE", "TYPE"
    );
    println!("{}", "-".repeat(80));
    for t in &output.tags {
        let kind = if t.is_annotated { "ann" } else { "lite" };
        let msg = t.message.as_deref().unwrap_or("-");
        println!(
            "{:<25} {:<9} {:<12} {:<5} {}",
            truncate(&t.name, 24),
            t.commit,
            format_epoch(t.commit_time),
            kind,
            truncate(msg, 30),
        );
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn status_char(s: &str) -> char {
    match s {
        "added" => 'A',
        "modified" => 'M',
        "deleted" => 'D',
        "renamed" => 'R',
        "typechange" => 'T',
        "untracked" => '?',
        "conflicted" => 'U',
        _ => ' ',
    }
}

fn format_epoch(ts: i64) -> String {
    // Simple ISO-style date from epoch without external crates
    // Seconds since 1970-01-01
    let secs = ts as u64;
    let days = secs / 86400;
    // Approximate: just show YYYY-MM-DD
    let (year, month, day) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    // Algorithm: civil calendar from days since epoch
    days += 719468;
    let era = days / 146097;
    let doe = days % 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
