mod config;
mod deps;
mod display;
mod git;
mod project;
mod services;
mod templates;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::process::Command;

use config::ConfigManager;
use project::Project;

#[derive(Parser)]
#[command(
    name = "projectctl",
    about = "Project switcher and development environment manager",
    version,
    author = "Angel Samuel Suesca Rios <suescapsam@gmail.com>"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all registered projects
    List {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
        /// Filter by project type
        #[arg(short = 't', long = "type")]
        project_type: Option<String>,
        /// Show only active projects (with running services)
        #[arg(short, long)]
        active: bool,
    },

    /// Switch to a project
    Switch {
        /// Project name (or partial match)
        name: Option<String>,
        /// Switch to the most recent project
        #[arg(short, long)]
        recent: bool,
        /// Also open in VSCode
        #[arg(short, long)]
        code: bool,
    },

    /// Show project details
    Info {
        /// Project name
        name: String,
        /// Show git info
        #[arg(short, long)]
        git: bool,
        /// Show dependency info
        #[arg(short, long)]
        deps: bool,
        /// Output only the project path (for shell integration)
        #[arg(long)]
        path_only: bool,
    },

    /// Start project services (Docker Compose)
    Start {
        /// Project name
        name: String,
        /// Start only a specific service
        #[arg(short, long)]
        service: Option<String>,
    },

    /// Stop project services
    Stop {
        /// Project name
        name: String,
        /// Stop only a specific service
        #[arg(short, long)]
        service: Option<String>,
    },

    /// Restart project services
    Restart {
        /// Project name
        name: String,
        /// Restart only a specific service
        #[arg(short, long)]
        service: Option<String>,
    },

    /// View service logs
    Logs {
        /// Project name
        name: String,
        /// Show logs for a specific service
        #[arg(short, long)]
        service: Option<String>,
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
        /// Number of lines to show
        #[arg(short, long, default_value = "50")]
        lines: usize,
    },

    /// Dependency management
    Deps {
        #[command(subcommand)]
        action: DepsAction,
    },

    /// Run a custom project command
    Run {
        /// Project name
        name: String,
        /// Command to run (e.g., dev, test, build)
        command: Option<String>,
        /// List available commands
        #[arg(short, long)]
        list: bool,
    },

    /// Add a project
    Add {
        /// Custom project name
        #[arg(short, long)]
        name: Option<String>,
        /// Path to the project directory
        #[arg(short, long)]
        path: Option<String>,
        /// Project type (auto-detected if not given)
        #[arg(short = 't', long = "type")]
        project_type: Option<String>,
    },

    /// Remove a project from the registry
    Remove {
        /// Project name
        name: String,
    },

    /// Show recently used projects
    Recent {
        /// Maximum number of projects to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Create a new project from a template
    New {
        /// Name for the new project
        name: String,
        /// Template to use
        #[arg(short, long)]
        template: String,
        /// Target directory (defaults to current directory)
        #[arg(short, long)]
        dir: Option<String>,
    },

    /// Manage project templates
    Templates {
        #[command(subcommand)]
        action: Option<TemplatesAction>,
    },

    /// Generate shell completions
    Completions {
        /// Shell type
        shell: String,
    },
}

#[derive(Subcommand)]
enum DepsAction {
    /// Update project dependencies
    Update {
        /// Project name (omit for --all)
        name: Option<String>,
        /// Update all projects
        #[arg(short, long)]
        all: bool,
    },
    /// Check for outdated dependencies
    Check {
        /// Project name (omit for --all)
        name: Option<String>,
        /// Check all projects
        #[arg(short, long)]
        all: bool,
    },
    /// Show dependency summary
    Summary,
}

#[derive(Subcommand)]
enum TemplatesAction {
    /// Add a custom template
    Add {
        /// Template name
        name: String,
        /// Path to template directory
        #[arg(short, long)]
        path: String,
    },
    /// List available templates
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = ConfigManager::new()?;
    config.ensure_dirs()?;

    match cli.command {
        Commands::List {
            detailed,
            project_type,
            active,
        } => cmd_list(&config, detailed, project_type, active)?,

        Commands::Switch {
            name,
            recent,
            code,
        } => cmd_switch(&config, name, recent, code)?,

        Commands::Info {
            name,
            git,
            deps,
            path_only,
        } => cmd_info(&config, &name, git, deps, path_only)?,

        Commands::Start { name, service } => cmd_start(&config, &name, service.as_deref())?,
        Commands::Stop { name, service } => cmd_stop(&config, &name, service.as_deref())?,
        Commands::Restart { name, service } => cmd_restart(&config, &name, service.as_deref())?,

        Commands::Logs {
            name,
            service,
            follow,
            lines,
        } => cmd_logs(&config, &name, service.as_deref(), follow, lines)?,

        Commands::Deps { action } => cmd_deps(&config, action)?,

        Commands::Run {
            name,
            command,
            list,
        } => cmd_run(&config, &name, command.as_deref(), list)?,

        Commands::Add {
            name,
            path,
            project_type,
        } => cmd_add(&config, name, path, project_type)?,

        Commands::Remove { name } => cmd_remove(&config, &name)?,

        Commands::Recent { limit } => cmd_recent(&config, limit)?,

        Commands::New {
            name,
            template,
            dir,
        } => cmd_new(&name, &template, dir.as_deref())?,

        Commands::Templates { action } => cmd_templates(&config, action)?,

        Commands::Completions { shell } => cmd_completions(&shell)?,
    }

    Ok(())
}

// =========================================================================
// Command implementations
// =========================================================================

fn cmd_list(
    config: &ConfigManager,
    detailed: bool,
    project_type: Option<String>,
    active: bool,
) -> Result<()> {
    let projects = config.load_projects()?;

    let filtered: Vec<Project> = projects
        .into_iter()
        .filter(|p| {
            if let Some(ref pt) = project_type {
                if !p.project_type.to_lowercase().contains(&pt.to_lowercase()) {
                    return false;
                }
            }
            if active && !p.has_docker_compose() {
                return false;
            }
            true
        })
        .collect();

    display::display_project_list(&filtered, detailed);
    Ok(())
}

fn cmd_switch(
    config: &ConfigManager,
    name: Option<String>,
    recent: bool,
    code: bool,
) -> Result<()> {
    let mut projects = config.load_projects()?;

    let project = if recent {
        // Find most recently used
        let mut sorted: Vec<(usize, _)> = projects
            .iter()
            .enumerate()
            .collect();
        sorted.sort_by(|a, b| {
            let ta = a.1.last_used_time();
            let tb = b.1.last_used_time();
            tb.cmp(&ta)
        });
        match sorted.first() {
            Some((idx, _)) => *idx,
            None => bail!("No projects registered."),
        }
    } else {
        let query = name.as_deref().unwrap_or_else(|| {
            eprintln!("{}", "Error: provide a project name or use --recent".red());
            std::process::exit(1);
        });
        match projects.iter().position(|p| {
            let q = query.to_lowercase();
            p.name.to_lowercase() == q
                || p.name.to_lowercase().starts_with(&q)
                || p.name.to_lowercase().contains(&q)
        }) {
            Some(idx) => idx,
            None => bail!("Project '{}' not found. Use 'projectctl list' to see registered projects.", query),
        }
    };

    let proj = &projects[project];
    let project_path = proj.expanded_path();

    if !project_path.exists() {
        bail!(
            "Project directory does not exist: {}",
            project_path.display()
        );
    }

    println!("Switching to: {}\n", proj.name.cyan().bold());

    // Show directory
    println!(
        "{} Changed directory\n   {}\n",
        "📂".to_string(),
        project_path.display().to_string().dimmed()
    );

    // Check for Python venv
    if let Some(venv) = proj.venv_path() {
        let python_version = get_python_version(&venv);
        println!(
            "{} Activated Python venv\n   {} ({})\n",
            "🐍".to_string(),
            venv.file_name().unwrap_or_default().to_string_lossy(),
            python_version
        );
    }

    // Check node version
    if proj.has_node_version() {
        println!("{} Node.js version detected\n", "📦".to_string());
    }

    // Git status
    if project_path.join(".git").exists() {
        if let Ok(git_info) = git::GitInfo::from_path(&project_path) {
            println!("{} Git status", "🌿".to_string());
            println!("   Branch: {}", git_info.branch.cyan());
            println!("   Status: {}\n", git_info.status_string());
        }
    }

    // Open in VSCode if requested
    if code {
        println!("{} Opening VSCode...\n", "💻".to_string());
        Command::new("code")
            .arg(&project_path)
            .spawn()
            .ok();
    }

    // Update last_used
    projects[project].touch();
    config.save_projects(&projects)?;

    // Print the cd command for shell integration
    println!("{} Ready to develop!", "✨".to_string());
    println!(
        "\n{}",
        format!("# Run this or use the shell function:\ncd {}", project_path.display()).dimmed()
    );

    Ok(())
}

fn cmd_info(
    config: &ConfigManager,
    name: &str,
    show_git: bool,
    show_deps: bool,
    path_only: bool,
) -> Result<()> {
    let projects = config.load_projects()?;
    let project = config
        .find_project(&projects, name)
        .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", name))?;

    if path_only {
        println!("{}", project.expanded_path().display());
        return Ok(());
    }

    let project_path = project.expanded_path();

    println!("{} {}", "Project:".bold(), project.name.cyan().bold());
    println!("{} {}", "Path:".bold(), project.path);
    println!("{} {}", "Type:".bold(), project.project_type);
    println!();

    // Git info
    if (show_git || (!show_git && !show_deps)) && project_path.join(".git").exists() {
        println!("{}:", "Git".bold());
        match git::GitInfo::from_path(&project_path) {
            Ok(info) => info.display(),
            Err(e) => println!("  Could not read git info: {}", e),
        }
        println!();
    }

    // Services info
    if !show_git && !show_deps {
        if project.has_docker_compose() {
            println!("{}:", "Services".bold());
            match services::get_compose_status(&project_path) {
                Ok(svcs) if !svcs.is_empty() => {
                    for (svc_name, state, ports) in &svcs {
                        let icon = if state == "running" {
                            "✓".green().to_string()
                        } else {
                            "✗".red().to_string()
                        };
                        let port_info = if ports.is_empty() {
                            String::new()
                        } else {
                            format!("  ({})", ports)
                        };
                        println!("  {} {} {}{}", icon, svc_name, state, port_info);
                    }
                }
                Ok(_) => println!("  No running services"),
                Err(_) => println!("  Could not query docker compose"),
            }
            println!();
        }

        // Environment
        println!("{}:", "Environment".bold());
        if project.has_venv() {
            let venv = project.venv_path().unwrap();
            let version = get_python_version(&venv);
            println!("  Python:  {} (venv)", version);
        }
        if project.has_node_version() {
            println!("  Node:    (version file detected)");
        }
        if !project.env.is_empty() {
            for (k, v) in &project.env {
                println!("  {}:  {}", k, v);
            }
        }
        println!();
    }

    // Deps info
    if show_deps || (!show_git && !show_deps) {
        if project_path.exists() {
            let managers = deps::detect_managers(&project_path);
            if !managers.is_empty() {
                println!("{}:", "Dependencies".bold());
                println!("  Managers: {}", managers.join(", "));
                println!();
            }
        }
    }

    // Custom commands
    if !project.commands.is_empty() && !show_git && !show_deps {
        println!("{}:", "Commands".bold());
        for (cmd_name, cmd_val) in &project.commands {
            println!("  {} = {}", cmd_name.cyan(), cmd_val.dimmed());
        }
        println!();
    }

    Ok(())
}

fn cmd_start(config: &ConfigManager, name: &str, service: Option<&str>) -> Result<()> {
    let projects = config.load_projects()?;
    let project = config
        .find_project(&projects, name)
        .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", name))?;
    services::start_services(project, service)
}

fn cmd_stop(config: &ConfigManager, name: &str, service: Option<&str>) -> Result<()> {
    let projects = config.load_projects()?;
    let project = config
        .find_project(&projects, name)
        .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", name))?;
    services::stop_services(project, service)
}

fn cmd_restart(config: &ConfigManager, name: &str, service: Option<&str>) -> Result<()> {
    let projects = config.load_projects()?;
    let project = config
        .find_project(&projects, name)
        .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", name))?;
    services::restart_services(project, service)
}

fn cmd_logs(
    config: &ConfigManager,
    name: &str,
    service: Option<&str>,
    follow: bool,
    lines: usize,
) -> Result<()> {
    let projects = config.load_projects()?;
    let project = config
        .find_project(&projects, name)
        .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", name))?;
    services::show_logs(project, service, follow, lines)
}

fn cmd_deps(config: &ConfigManager, action: DepsAction) -> Result<()> {
    let projects = config.load_projects()?;

    match action {
        DepsAction::Update { name, all } => {
            if all {
                for project in &projects {
                    deps::update_deps(project)?;
                    println!();
                }
            } else {
                let query = name.as_deref().unwrap_or_else(|| {
                    eprintln!(
                        "{}",
                        "Provide a project name or use --all".red()
                    );
                    std::process::exit(1);
                });
                let project = config
                    .find_project(&projects, query)
                    .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", query))?;
                deps::update_deps(project)?;
            }
        }
        DepsAction::Check { name, all } => {
            if all {
                for project in &projects {
                    deps::check_outdated(project)?;
                    println!();
                }
            } else {
                let query = name.as_deref().unwrap_or_else(|| {
                    eprintln!(
                        "{}",
                        "Provide a project name or use --all".red()
                    );
                    std::process::exit(1);
                });
                let project = config
                    .find_project(&projects, query)
                    .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", query))?;
                deps::check_outdated(project)?;
            }
        }
        DepsAction::Summary => {
            deps::show_summary(&projects)?;
        }
    }

    Ok(())
}

fn cmd_run(config: &ConfigManager, name: &str, command: Option<&str>, list: bool) -> Result<()> {
    let projects = config.load_projects()?;
    let project = config
        .find_project(&projects, name)
        .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", name))?;

    if list || command.is_none() {
        if project.commands.is_empty() {
            println!("{}", "No custom commands defined for this project.".yellow());
            println!("Add them in ~/.projectctl/projects.toml under [project.commands]");
            return Ok(());
        }
        println!(
            "Available commands for {}:\n",
            project.name.cyan().bold()
        );
        for (cmd_name, cmd_val) in &project.commands {
            println!("  {} → {}", cmd_name.bold(), cmd_val.dimmed());
        }
        return Ok(());
    }

    let cmd_name = command.unwrap();
    let cmd_value = project.commands.get(cmd_name).ok_or_else(|| {
        anyhow::anyhow!(
            "Command '{}' not found for project '{}'. Use --list to see available commands.",
            cmd_name,
            project.name
        )
    })?;

    let project_path = project.expanded_path();
    println!(
        "Running: {} {}\n",
        project.name.cyan().bold(),
        cmd_name.bold()
    );
    println!(
        "Executing: {}\n",
        cmd_value.dimmed()
    );

    let status = Command::new("sh")
        .args(["-c", cmd_value])
        .current_dir(&project_path)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .stdin(std::process::Stdio::inherit())
        .status()?;

    if !status.success() {
        bail!("Command exited with status: {}", status);
    }

    Ok(())
}

fn cmd_add(
    config: &ConfigManager,
    name: Option<String>,
    path: Option<String>,
    project_type: Option<String>,
) -> Result<()> {
    let project_path = match path {
        Some(p) => ConfigManager::expand_path(&p),
        None => std::env::current_dir()?,
    };

    if !project_path.is_dir() {
        bail!("Path is not a directory: {}", project_path.display());
    }

    let project_name = name.unwrap_or_else(|| {
        project_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    let detected_type = project_type.unwrap_or_else(|| Project::detect_type(&project_path));
    let detected_services = Project::detect_services(&project_path);
    let detected_commands = Project::detect_commands(&project_path, &detected_type);

    let mut projects = config.load_projects()?;

    // Check if already registered
    if projects.iter().any(|p| p.name == project_name) {
        bail!(
            "Project '{}' already registered. Remove it first to re-add.",
            project_name
        );
    }

    let mut project = Project::new(
        project_name.clone(),
        project_path.to_string_lossy().to_string(),
        detected_type.clone(),
    );
    project.services = detected_services;
    project.commands = detected_commands;

    println!("{} Project added!\n", "✓".green().bold());
    println!("  Name:     {}", project.name.cyan());
    println!("  Path:     {}", project.path);
    println!("  Type:     {}", detected_type);
    if !project.services.is_empty() {
        println!("  Services: {}", project.services.join(", "));
    }
    if !project.commands.is_empty() {
        let cmds: Vec<String> = project.commands.keys().cloned().collect();
        println!("  Commands: {}", cmds.join(", "));
    }

    projects.push(project);
    config.save_projects(&projects)?;

    Ok(())
}

fn cmd_remove(config: &ConfigManager, name: &str) -> Result<()> {
    let mut projects = config.load_projects()?;
    let original_len = projects.len();

    projects.retain(|p| {
        let q = name.to_lowercase();
        p.name.to_lowercase() != q
    });

    if projects.len() == original_len {
        bail!("Project '{}' not found.", name);
    }

    config.save_projects(&projects)?;
    println!(
        "{} Project '{}' removed.",
        "✓".green(),
        name.cyan()
    );

    Ok(())
}

fn cmd_recent(config: &ConfigManager, limit: usize) -> Result<()> {
    let projects = config.load_projects()?;
    display::display_recent(&projects, limit);
    Ok(())
}

fn cmd_new(name: &str, template: &str, dir: Option<&str>) -> Result<()> {
    templates::create_from_template(name, template, dir)?;
    Ok(())
}

fn cmd_templates(config: &ConfigManager, action: Option<TemplatesAction>) -> Result<()> {
    match action {
        Some(TemplatesAction::Add { name, path }) => {
            templates::add_template(config, &name, &path)?;
        }
        Some(TemplatesAction::List) | None => {
            templates::list_templates(config)?;
        }
    }
    Ok(())
}

fn cmd_completions(shell: &str) -> Result<()> {
    use clap::CommandFactory;
    let mut cmd = Cli::command();

    match shell {
        "bash" => {
            clap_complete::generate(
                clap_complete::Shell::Bash,
                &mut cmd,
                "projectctl",
                &mut std::io::stdout(),
            );
        }
        "zsh" => {
            clap_complete::generate(
                clap_complete::Shell::Zsh,
                &mut cmd,
                "projectctl",
                &mut std::io::stdout(),
            );
        }
        "fish" => {
            clap_complete::generate(
                clap_complete::Shell::Fish,
                &mut cmd,
                "projectctl",
                &mut std::io::stdout(),
            );
        }
        _ => {
            bail!("Unsupported shell: {}. Supported: bash, zsh, fish", shell);
        }
    }
    Ok(())
}

/// Helper: get python version from a venv
fn get_python_version(venv_path: &std::path::Path) -> String {
    let python = venv_path.join("bin").join("python");
    if python.exists() {
        if let Ok(output) = Command::new(&python).arg("--version").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.trim().to_string();
        }
    }
    "Python (unknown version)".to_string()
}
