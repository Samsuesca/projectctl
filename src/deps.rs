use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::path::Path;
use std::process::Command;

use crate::project::Project;

/// Dependency info for a project
#[allow(dead_code)]
#[derive(Debug)]
pub struct DepsInfo {
    pub manager: String,
    pub total_packages: usize,
    pub outdated_packages: Vec<OutdatedPackage>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct OutdatedPackage {
    pub name: String,
    pub current: String,
    pub latest: String,
}

/// Detect which package managers are in use
pub fn detect_managers(project_path: &Path) -> Vec<String> {
    let mut managers = Vec::new();

    if project_path.join("Cargo.toml").exists() {
        managers.push("cargo".to_string());
    }
    if project_path.join("package.json").exists() {
        if project_path.join("yarn.lock").exists() {
            managers.push("yarn".to_string());
        } else if project_path.join("pnpm-lock.yaml").exists() {
            managers.push("pnpm".to_string());
        } else {
            managers.push("npm".to_string());
        }
    }
    if project_path.join("requirements.txt").exists()
        || project_path.join("pyproject.toml").exists()
        || project_path.join("setup.py").exists()
    {
        if project_path.join("Pipfile").exists() {
            managers.push("pipenv".to_string());
        } else if project_path.join("poetry.lock").exists() {
            managers.push("poetry".to_string());
        } else {
            managers.push("pip".to_string());
        }
    }
    if project_path.join("go.mod").exists() {
        managers.push("go".to_string());
    }

    managers
}

/// Check for outdated packages
pub fn check_outdated(project: &Project) -> Result<()> {
    let project_path = project.expanded_path();
    if !project_path.exists() {
        bail!("Project directory does not exist: {}", project.path);
    }

    let managers = detect_managers(&project_path);
    if managers.is_empty() {
        println!("{}", "No package managers detected.".yellow());
        return Ok(());
    }

    println!(
        "Checking outdated packages for: {}\n",
        project.name.cyan().bold()
    );

    for manager in &managers {
        match manager.as_str() {
            "cargo" => check_cargo_outdated(&project_path)?,
            "npm" => check_npm_outdated(&project_path)?,
            "yarn" => check_yarn_outdated(&project_path)?,
            "pnpm" => check_pnpm_outdated(&project_path)?,
            "pip" => check_pip_outdated(&project_path)?,
            "poetry" => check_poetry_outdated(&project_path)?,
            "go" => check_go_outdated(&project_path)?,
            _ => {}
        }
    }

    Ok(())
}

/// Update dependencies
pub fn update_deps(project: &Project) -> Result<()> {
    let project_path = project.expanded_path();
    if !project_path.exists() {
        bail!("Project directory does not exist: {}", project.path);
    }

    let managers = detect_managers(&project_path);
    if managers.is_empty() {
        println!("{}", "No package managers detected.".yellow());
        return Ok(());
    }

    println!(
        "Updating dependencies for: {}\n",
        project.name.cyan().bold()
    );

    for manager in &managers {
        match manager.as_str() {
            "cargo" => update_cargo(&project_path)?,
            "npm" => update_npm(&project_path)?,
            "yarn" => update_yarn(&project_path)?,
            "pnpm" => update_pnpm(&project_path)?,
            "pip" => update_pip(&project_path)?,
            "poetry" => update_poetry(&project_path)?,
            "go" => update_go(&project_path)?,
            _ => {}
        }
    }

    println!("\n{}", "Dependencies updated!".green().bold());
    Ok(())
}

/// Show dependency summary across all projects
pub fn show_summary(projects: &[Project]) -> Result<()> {
    println!("{}\n", "Dependency Summary".cyan().bold());

    for project in projects {
        let project_path = project.expanded_path();
        if !project_path.exists() {
            continue;
        }
        let managers = detect_managers(&project_path);
        if managers.is_empty() {
            continue;
        }
        println!(
            "  {} ({})",
            project.name.bold(),
            managers.join(", ").dimmed()
        );
    }

    Ok(())
}

// --- Cargo ---

fn check_cargo_outdated(path: &Path) -> Result<()> {
    println!("  {} (Rust/Cargo):", "Backend".bold());
    let output = Command::new("cargo")
        .args(["update", "--dry-run"])
        .current_dir(path)
        .output();

    match output {
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let updating_lines: Vec<&str> = stderr
                .lines()
                .filter(|l| l.contains("Updating") || l.contains("updating"))
                .collect();
            if updating_lines.is_empty() {
                println!("    {} All dependencies up to date", "✓".green());
            } else {
                for line in &updating_lines {
                    println!("    {}", line.trim().yellow());
                }
            }
        }
        Err(_) => println!("    {} cargo not available", "✗".red()),
    }
    Ok(())
}

fn update_cargo(path: &Path) -> Result<()> {
    println!("  {} (Rust/Cargo):", "Updating".bold());
    let output = Command::new("cargo")
        .args(["update"])
        .current_dir(path)
        .output()
        .context("Failed to run cargo update")?;

    if output.status.success() {
        println!("    {} Dependencies updated", "✓".green());
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("    {} Update failed: {}", "✗".red(), stderr.trim());
    }
    Ok(())
}

// --- npm ---

fn check_npm_outdated(path: &Path) -> Result<()> {
    println!("  {} (Node/npm):", "Frontend".bold());
    let output = Command::new("npm")
        .args(["outdated", "--json"])
        .current_dir(path)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.trim().is_empty() || stdout.trim() == "{}" {
                println!("    {} All dependencies up to date", "✓".green());
            } else if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&stdout) {
                if let Some(obj) = parsed.as_object() {
                    let count = obj.len();
                    println!("    {} {} outdated package(s)", "⬆".yellow(), count);
                    for (name, info) in obj.iter().take(10) {
                        let current = info["current"].as_str().unwrap_or("?");
                        let latest = info["latest"].as_str().unwrap_or("?");
                        println!(
                            "      {}: {} → {}",
                            name,
                            current.dimmed(),
                            latest.green()
                        );
                    }
                    if count > 10 {
                        println!("      ... and {} more", count - 10);
                    }
                }
            }
        }
        Err(_) => println!("    {} npm not available", "✗".red()),
    }
    Ok(())
}

fn update_npm(path: &Path) -> Result<()> {
    println!("  {} (Node/npm):", "Updating".bold());
    let output = Command::new("npm")
        .args(["update"])
        .current_dir(path)
        .output()
        .context("Failed to run npm update")?;

    if output.status.success() {
        println!("    {} Dependencies updated", "✓".green());
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("    {} Update failed: {}", "✗".red(), stderr.trim());
    }
    Ok(())
}

// --- yarn ---

fn check_yarn_outdated(path: &Path) -> Result<()> {
    println!("  {} (Node/yarn):", "Frontend".bold());
    let output = Command::new("yarn")
        .args(["outdated"])
        .current_dir(path)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.trim().is_empty() {
                println!("    {} All dependencies up to date", "✓".green());
            } else {
                let lines: Vec<&str> = stdout.lines().collect();
                let count = lines.len().saturating_sub(1); // header line
                println!("    {} {} outdated package(s)", "⬆".yellow(), count);
            }
        }
        Err(_) => println!("    {} yarn not available", "✗".red()),
    }
    Ok(())
}

fn update_yarn(path: &Path) -> Result<()> {
    println!("  {} (Node/yarn):", "Updating".bold());
    let output = Command::new("yarn")
        .args(["upgrade"])
        .current_dir(path)
        .output()
        .context("Failed to run yarn upgrade")?;

    if output.status.success() {
        println!("    {} Dependencies updated", "✓".green());
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("    {} Update failed: {}", "✗".red(), stderr.trim());
    }
    Ok(())
}

// --- pnpm ---

fn check_pnpm_outdated(path: &Path) -> Result<()> {
    println!("  {} (Node/pnpm):", "Frontend".bold());
    let output = Command::new("pnpm")
        .args(["outdated"])
        .current_dir(path)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.trim().is_empty() {
                println!("    {} All dependencies up to date", "✓".green());
            } else {
                println!("    {}", stdout.trim());
            }
        }
        Err(_) => println!("    {} pnpm not available", "✗".red()),
    }
    Ok(())
}

fn update_pnpm(path: &Path) -> Result<()> {
    println!("  {} (Node/pnpm):", "Updating".bold());
    let output = Command::new("pnpm")
        .args(["update"])
        .current_dir(path)
        .output()
        .context("Failed to run pnpm update")?;

    if output.status.success() {
        println!("    {} Dependencies updated", "✓".green());
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("    {} Update failed: {}", "✗".red(), stderr.trim());
    }
    Ok(())
}

// --- pip ---

fn check_pip_outdated(path: &Path) -> Result<()> {
    println!("  {} (Python/pip):", "Backend".bold());
    let output = Command::new("pip")
        .args(["list", "--outdated", "--format", "json"])
        .current_dir(path)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
                if parsed.is_empty() {
                    println!("    {} All dependencies up to date", "✓".green());
                } else {
                    println!(
                        "    {} {} outdated package(s)",
                        "⬆".yellow(),
                        parsed.len()
                    );
                    for pkg in parsed.iter().take(10) {
                        let name = pkg["name"].as_str().unwrap_or("?");
                        let current = pkg["version"].as_str().unwrap_or("?");
                        let latest = pkg["latest_version"].as_str().unwrap_or("?");
                        println!(
                            "      {}: {} → {}",
                            name,
                            current.dimmed(),
                            latest.green()
                        );
                    }
                    if parsed.len() > 10 {
                        println!("      ... and {} more", parsed.len() - 10);
                    }
                }
            }
        }
        Err(_) => println!("    {} pip not available", "✗".red()),
    }
    Ok(())
}

fn update_pip(path: &Path) -> Result<()> {
    println!("  {} (Python/pip):", "Updating".bold());
    // Check for requirements.txt
    let req_file = path.join("requirements.txt");
    if req_file.exists() {
        let output = Command::new("pip")
            .args(["install", "--upgrade", "-r", "requirements.txt"])
            .current_dir(path)
            .output()
            .context("Failed to run pip install --upgrade")?;

        if output.status.success() {
            println!("    {} Dependencies updated", "✓".green());
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("    {} Update failed: {}", "✗".red(), stderr.trim());
        }
    } else {
        println!("    {} No requirements.txt found", "⚠".yellow());
    }
    Ok(())
}

// --- poetry ---

fn check_poetry_outdated(path: &Path) -> Result<()> {
    println!("  {} (Python/poetry):", "Backend".bold());
    let output = Command::new("poetry")
        .args(["show", "--outdated"])
        .current_dir(path)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.trim().is_empty() {
                println!("    {} All dependencies up to date", "✓".green());
            } else {
                let count = stdout.lines().count();
                println!("    {} {} outdated package(s)", "⬆".yellow(), count);
            }
        }
        Err(_) => println!("    {} poetry not available", "✗".red()),
    }
    Ok(())
}

fn update_poetry(path: &Path) -> Result<()> {
    println!("  {} (Python/poetry):", "Updating".bold());
    let output = Command::new("poetry")
        .args(["update"])
        .current_dir(path)
        .output()
        .context("Failed to run poetry update")?;

    if output.status.success() {
        println!("    {} Dependencies updated", "✓".green());
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("    {} Update failed: {}", "✗".red(), stderr.trim());
    }
    Ok(())
}

// --- go ---

fn check_go_outdated(path: &Path) -> Result<()> {
    println!("  {} (Go):", "Modules".bold());
    let output = Command::new("go")
        .args(["list", "-m", "-u", "all"])
        .current_dir(path)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let outdated: Vec<&str> = stdout
                .lines()
                .filter(|l| l.contains('['))
                .collect();
            if outdated.is_empty() {
                println!("    {} All modules up to date", "✓".green());
            } else {
                println!(
                    "    {} {} outdated module(s)",
                    "⬆".yellow(),
                    outdated.len()
                );
                for line in outdated.iter().take(10) {
                    println!("      {}", line.trim());
                }
            }
        }
        Err(_) => println!("    {} go not available", "✗".red()),
    }
    Ok(())
}

fn update_go(path: &Path) -> Result<()> {
    println!("  {} (Go):", "Updating".bold());
    let output = Command::new("go")
        .args(["get", "-u", "./..."])
        .current_dir(path)
        .output()
        .context("Failed to run go get -u")?;

    if output.status.success() {
        println!("    {} Modules updated", "✓".green());
        // Also tidy
        Command::new("go")
            .args(["mod", "tidy"])
            .current_dir(path)
            .output()
            .ok();
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("    {} Update failed: {}", "✗".red(), stderr.trim());
    }
    Ok(())
}
