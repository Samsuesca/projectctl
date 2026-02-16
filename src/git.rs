use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

/// Git information for a project
#[derive(Debug)]
pub struct GitInfo {
    pub branch: String,
    pub changed_files: usize,
    pub staged_files: usize,
    pub untracked_files: usize,
    pub unpushed_commits: usize,
    pub last_commit_message: String,
    pub last_commit_time: String,
    pub is_clean: bool,
}

impl GitInfo {
    /// Get Git information for a directory
    pub fn from_path(path: &Path) -> Result<Self> {
        let repo = git2::Repository::open(path)
            .context("Not a git repository")?;

        let branch = get_branch_name(&repo)?;
        let (changed_files, staged_files, untracked_files) = get_status_counts(&repo)?;
        let is_clean = changed_files == 0 && staged_files == 0 && untracked_files == 0;
        let (last_commit_message, last_commit_time) = get_last_commit(&repo)?;
        let unpushed_commits = count_unpushed(&repo, &branch);

        Ok(Self {
            branch,
            changed_files,
            staged_files,
            untracked_files,
            unpushed_commits,
            last_commit_message,
            last_commit_time,
            is_clean,
        })
    }

    /// Get a short status string
    pub fn status_string(&self) -> String {
        if self.is_clean {
            "clean".green().to_string()
        } else {
            let mut parts = Vec::new();
            if self.changed_files > 0 {
                parts.push(format!("{} modified", self.changed_files));
            }
            if self.staged_files > 0 {
                parts.push(format!("{} staged", self.staged_files));
            }
            if self.untracked_files > 0 {
                parts.push(format!("{} untracked", self.untracked_files));
            }
            parts.join(", ").yellow().to_string()
        }
    }

    /// Display git info block
    pub fn display(&self) {
        println!("  Branch:        {}", self.branch.cyan());
        println!("  Status:        {}", self.status_string());
        if self.unpushed_commits > 0 {
            println!(
                "  Unpushed:      {}",
                format!("{} commits", self.unpushed_commits).yellow()
            );
        }
        println!(
            "  Last commit:   {} ({})",
            self.last_commit_message, self.last_commit_time
        );
    }
}

fn get_branch_name(repo: &git2::Repository) -> Result<String> {
    if repo.head_detached().unwrap_or(false) {
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        let short = &commit.id().to_string()[..8];
        return Ok(format!("detached@{}", short));
    }
    let head = repo.head().context("Failed to get HEAD")?;
    let name = head
        .shorthand()
        .unwrap_or("unknown")
        .to_string();
    Ok(name)
}

fn get_status_counts(repo: &git2::Repository) -> Result<(usize, usize, usize)> {
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(false);

    let statuses = repo.statuses(Some(&mut opts))?;

    let mut changed = 0;
    let mut staged = 0;
    let mut untracked = 0;

    for entry in statuses.iter() {
        let s = entry.status();
        if s.contains(git2::Status::WT_MODIFIED)
            || s.contains(git2::Status::WT_DELETED)
            || s.contains(git2::Status::WT_RENAMED)
            || s.contains(git2::Status::WT_TYPECHANGE)
        {
            changed += 1;
        }
        if s.contains(git2::Status::INDEX_NEW)
            || s.contains(git2::Status::INDEX_MODIFIED)
            || s.contains(git2::Status::INDEX_DELETED)
            || s.contains(git2::Status::INDEX_RENAMED)
        {
            staged += 1;
        }
        if s.contains(git2::Status::WT_NEW) {
            untracked += 1;
        }
    }

    Ok((changed, staged, untracked))
}

fn get_last_commit(repo: &git2::Repository) -> Result<(String, String)> {
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return Ok(("No commits yet".to_string(), "".to_string())),
    };
    let commit = head.peel_to_commit()?;
    let message = commit
        .summary()
        .unwrap_or("(no message)")
        .to_string();
    let time = commit.time();
    let secs = time.seconds();
    let dt = chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_default();
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(dt);

    let time_ago = if duration.num_minutes() < 1 {
        "just now".to_string()
    } else if duration.num_hours() < 1 {
        format!("{} min ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{} hours ago", duration.num_hours())
    } else if duration.num_days() < 7 {
        format!("{} days ago", duration.num_days())
    } else {
        format!("{} weeks ago", duration.num_weeks())
    };

    Ok((message, time_ago))
}

fn count_unpushed(repo: &git2::Repository, branch: &str) -> usize {
    let remote_branch = format!("origin/{}", branch);
    let local = match repo.revparse_single(&format!("refs/heads/{}", branch)) {
        Ok(obj) => obj.id(),
        Err(_) => return 0,
    };
    let remote = match repo.revparse_single(&format!("refs/remotes/{}", remote_branch)) {
        Ok(obj) => obj.id(),
        Err(_) => return 0,
    };

    let mut count = 0;
    if let Ok(mut walk) = repo.revwalk() {
        walk.push(local).ok();
        walk.hide(remote).ok();
        count = walk.count();
    }
    count
}
