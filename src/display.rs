use colored::Colorize;
use tabled::{
    settings::Style,
    Table, Tabled,
};

use crate::project::Project;
use crate::services;

/// Row in the project list table
#[derive(Tabled)]
struct ProjectRow {
    #[tabled(rename = "#")]
    index: usize,
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Type")]
    project_type: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Last Used")]
    last_used: String,
}

/// Display the project list as a formatted table
pub fn display_project_list(projects: &[Project], detailed: bool) {
    if projects.is_empty() {
        println!("{}", "No projects registered.".yellow());
        println!("Add one with: projectctl add --path /path/to/project");
        return;
    }

    println!("{}\n", "Registered Projects:".bold());

    let rows: Vec<ProjectRow> = projects
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let status = get_project_status(p);
            ProjectRow {
                index: i + 1,
                name: p.name.clone(),
                project_type: capitalize(&p.project_type),
                status,
                last_used: p.last_used_ago(),
            }
        })
        .collect();

    let table = Table::new(&rows)
        .with(Style::modern_rounded())
        .to_string();

    println!("{}", table);

    // Summary line
    let total = projects.len();
    let active = projects
        .iter()
        .filter(|p| is_running(p))
        .count();

    // Count by type
    let mut type_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for p in projects {
        *type_counts
            .entry(capitalize(&p.project_type))
            .or_insert(0) += 1;
    }

    let type_parts: Vec<String> = type_counts
        .iter()
        .map(|(t, c)| format!("{}: {}", t, c))
        .collect();

    println!(
        "\nTotal: {} projects | Active: {} | {}",
        total.to_string().bold(),
        active.to_string().bold(),
        type_parts.join(" | ")
    );

    if detailed {
        println!();
        for p in projects {
            println!("  {} ({})", p.name.cyan().bold(), p.path);
            if !p.commands.is_empty() {
                let cmds: Vec<String> = p.commands.keys().cloned().collect();
                println!("    Commands: {}", cmds.join(", "));
            }
            if !p.services.is_empty() {
                println!("    Services: {}", p.services.join(", "));
            }
        }
    }
}

/// Display recent projects list
pub fn display_recent(projects: &[Project], limit: usize) {
    if projects.is_empty() {
        println!("{}", "No recent projects.".yellow());
        return;
    }

    println!("{}\n", "Recent Projects:".bold());

    let mut sorted: Vec<&Project> = projects.iter().collect();
    sorted.sort_by(|a, b| {
        let ta = a.last_used_time();
        let tb = b.last_used_time();
        tb.cmp(&ta)
    });

    for (i, p) in sorted.iter().take(limit).enumerate() {
        println!(
            "  {}. {}  ({})",
            (i + 1).to_string().bold(),
            p.name.cyan(),
            p.last_used_ago()
        );
    }

    println!("\nSwitch: projectctl switch <name>");
}

/// Get a status string for a project
fn get_project_status(project: &Project) -> String {
    if !project.exists() {
        return format!("{} Missing", "!".yellow());
    }
    if project.has_docker_compose() && is_running(project) {
        return format!("{} Running", "✓".green());
    }
    "Idle".dimmed().to_string()
}

/// Check if any docker compose services are running for the project
fn is_running(project: &Project) -> bool {
    if !project.has_docker_compose() {
        return false;
    }
    let path = project.expanded_path();
    if let Ok(svcs) = services::get_compose_status(&path) {
        return svcs.iter().any(|(_, state, _)| state == "running");
    }
    false
}

fn capitalize(s: &str) -> String {
    if s.is_empty() {
        return s.to_string();
    }
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
