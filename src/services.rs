use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::path::Path;
use std::process::Command;

use crate::project::Project;

/// Check if docker/docker compose is available
#[allow(dead_code)]
pub fn docker_available() -> bool {
    Command::new("docker")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Find the docker-compose file in a project directory
fn find_compose_file(project_path: &Path) -> Option<String> {
    let candidates = [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ];
    for file in &candidates {
        if project_path.join(file).exists() {
            return Some(file.to_string());
        }
    }
    None
}

/// Get the status of docker compose services
pub fn get_compose_status(project_path: &Path) -> Result<Vec<(String, String, String)>> {
    let compose_file = match find_compose_file(project_path) {
        Some(f) => f,
        None => return Ok(Vec::new()),
    };

    let output = Command::new("docker")
        .args(["compose", "-f", &compose_file, "ps", "--format", "json"])
        .current_dir(project_path)
        .output()
        .context("Failed to run docker compose ps")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut services = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            let name = value["Service"]
                .as_str()
                .or_else(|| value["Name"].as_str())
                .unwrap_or("unknown")
                .to_string();
            let state = value["State"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let ports = value["Ports"]
                .as_str()
                .unwrap_or("")
                .to_string();
            services.push((name, state, ports));
        }
    }

    Ok(services)
}

/// Start docker compose services
pub fn start_services(project: &Project, service: Option<&str>) -> Result<()> {
    let project_path = project.expanded_path();
    let compose_file = match find_compose_file(&project_path) {
        Some(f) => f,
        None => {
            println!("{}", "No docker-compose file found.".yellow());
            return Ok(());
        }
    };

    println!(
        "Starting services for: {}\n",
        project.name.cyan().bold()
    );

    let mut cmd = Command::new("docker");
    cmd.args(["compose", "-f", &compose_file, "up", "-d"]);
    cmd.current_dir(&project_path);

    if let Some(svc) = service {
        cmd.arg(svc);
        println!("  Starting service: {}", svc.cyan());
    }

    let output = cmd.output().context("Failed to run docker compose up")?;

    if output.status.success() {
        // Show running services
        let services = get_compose_status(&project_path)?;
        if !services.is_empty() {
            println!("  Docker Compose:");
            for (name, state, ports) in &services {
                let icon = if state == "running" {
                    "✓".green().to_string()
                } else {
                    "✗".red().to_string()
                };
                let port_info = if ports.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", ports)
                };
                println!("   {} {} {}{}", icon, name, state, port_info);
            }
        }
        println!("\n{}", "Services started!".green().bold());
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to start services:\n{}", stderr);
    }

    Ok(())
}

/// Stop docker compose services
pub fn stop_services(project: &Project, service: Option<&str>) -> Result<()> {
    let project_path = project.expanded_path();
    let compose_file = match find_compose_file(&project_path) {
        Some(f) => f,
        None => {
            println!("{}", "No docker-compose file found.".yellow());
            return Ok(());
        }
    };

    println!("Stopping services for: {}\n", project.name.cyan().bold());

    let mut cmd = Command::new("docker");
    cmd.args(["compose", "-f", &compose_file, "stop"]);
    cmd.current_dir(&project_path);

    if let Some(svc) = service {
        cmd.arg(svc);
        println!("  Stopping service: {}", svc.cyan());
    }

    let output = cmd.output().context("Failed to run docker compose stop")?;

    if output.status.success() {
        println!("{}", "Services stopped.".green().bold());
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to stop services:\n{}", stderr);
    }

    Ok(())
}

/// Restart docker compose services
pub fn restart_services(project: &Project, service: Option<&str>) -> Result<()> {
    let project_path = project.expanded_path();
    let compose_file = match find_compose_file(&project_path) {
        Some(f) => f,
        None => {
            println!("{}", "No docker-compose file found.".yellow());
            return Ok(());
        }
    };

    println!("Restarting services for: {}\n", project.name.cyan().bold());

    let mut cmd = Command::new("docker");
    cmd.args(["compose", "-f", &compose_file, "restart"]);
    cmd.current_dir(&project_path);

    if let Some(svc) = service {
        cmd.arg(svc);
    }

    let output = cmd.output().context("Failed to run docker compose restart")?;

    if output.status.success() {
        let services = get_compose_status(&project_path)?;
        if !services.is_empty() {
            println!("  Docker Compose:");
            for (name, state, ports) in &services {
                let icon = if state == "running" {
                    "✓".green().to_string()
                } else {
                    "✗".red().to_string()
                };
                let port_info = if ports.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", ports)
                };
                println!("   {} {} {}{}", icon, name, state, port_info);
            }
        }
        println!("\n{}", "Services restarted!".green().bold());
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to restart services:\n{}", stderr);
    }

    Ok(())
}

/// Show logs for docker compose services
pub fn show_logs(project: &Project, service: Option<&str>, follow: bool, lines: usize) -> Result<()> {
    let project_path = project.expanded_path();
    let compose_file = match find_compose_file(&project_path) {
        Some(f) => f,
        None => {
            bail!("No docker-compose file found.");
        }
    };

    let mut cmd = Command::new("docker");
    cmd.args(["compose", "-f", &compose_file, "logs"]);
    cmd.arg("--tail");
    cmd.arg(lines.to_string());
    cmd.current_dir(&project_path);

    if follow {
        cmd.arg("--follow");
    }

    if let Some(svc) = service {
        cmd.arg(svc);
    }

    // For follow mode, use spawn to keep the process alive
    if follow {
        let mut child = cmd
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .context("Failed to spawn docker compose logs")?;
        child.wait().context("Failed to wait for logs process")?;
    } else {
        let output = cmd.output().context("Failed to run docker compose logs")?;
        print!("{}", String::from_utf8_lossy(&output.stdout));
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
    }

    Ok(())
}
