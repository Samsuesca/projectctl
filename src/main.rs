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
    author = "Angel Samuel Suesca Rios <suescapsam@gmail.com>",
    after_help = "\
Common workflows:
  Register project:     projectctl add --path ~/code/myapp
  Switch context:       projectctl switch myapp
  View all projects:    projectctl list --detailed
  Start services:       projectctl start myapp
  View recent:          projectctl recent
  Create new project:   projectctl new myapp --template react-vite
  Shell completions:    projectctl completions zsh >> ~/.zshrc"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all registered projects
    #[command(long_about = "\
List all registered projects in a formatted table.

Shows project name, type, status, and last-used time. Use filters to narrow
results by type or running state. The --detailed flag adds paths, commands,
and services for each project.

Examples:
  projectctl list                        # Show all projects
  projectctl list --detailed             # Show with paths and commands
  projectctl list -t fastapi             # Filter by type
  projectctl list --active               # Only projects with running services
  projectctl list -t react --detailed    # Combine filters")]
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
    #[command(long_about = "\
Switch to a project context.

Displays the project directory, activates any detected Python virtualenv,
shows git branch status, and optionally opens the project in VSCode.
Supports fuzzy name matching (partial, prefix, or substring).

Examples:
  projectctl switch myapp                # Switch by name
  projectctl switch my                   # Partial name match
  projectctl switch --recent             # Switch to last used project
  projectctl switch myapp --code         # Switch and open in VSCode
  projectctl switch uniforme -c          # Fuzzy match + VSCode")]
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
    #[command(long_about = "\
Show detailed information about a specific project.

Displays project metadata, git status, running services, environment
configuration, detected dependency managers, and custom commands. Use
--path-only for shell scripting integration.

Examples:
  projectctl info myapp                  # Full project overview
  projectctl info myapp --git            # Git info only
  projectctl info myapp --deps           # Dependency info only
  projectctl info myapp --path-only      # Print path (for scripts)
  cd $(projectctl info myapp --path-only)  # Shell integration")]
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
    #[command(long_about = "\
Start Docker Compose services for a project.

Runs 'docker compose up -d' in the project directory. Optionally start
only a specific service by name. The project must have a docker-compose.yml
or compose.yml file.

Examples:
  projectctl start myapp                 # Start all services
  projectctl start myapp -s backend      # Start only backend service
  projectctl start myapp -s postgres     # Start only the database
  projectctl start uniforme --service redis  # Start Redis for a project")]
    Start {
        /// Project name
        name: String,
        /// Start only a specific service
        #[arg(short, long)]
        service: Option<String>,
    },

    /// Stop project services
    #[command(long_about = "\
Stop Docker Compose services for a project.

Runs 'docker compose stop' (or 'docker compose stop <service>') in the
project directory. Does not remove containers or volumes.

Examples:
  projectctl stop myapp                  # Stop all services
  projectctl stop myapp -s backend       # Stop only backend
  projectctl stop myapp --service redis  # Stop a specific service")]
    Stop {
        /// Project name
        name: String,
        /// Stop only a specific service
        #[arg(short, long)]
        service: Option<String>,
    },

    /// Restart project services
    #[command(long_about = "\
Restart Docker Compose services for a project.

Runs 'docker compose restart' in the project directory. Useful after
configuration changes or when a service becomes unresponsive.

Examples:
  projectctl restart myapp               # Restart all services
  projectctl restart myapp -s backend    # Restart only backend
  projectctl restart myapp --service api # Restart a specific service")]
    Restart {
        /// Project name
        name: String,
        /// Restart only a specific service
        #[arg(short, long)]
        service: Option<String>,
    },

    /// View service logs
    #[command(long_about = "\
View Docker Compose service logs for a project.

Displays logs from running containers. Use --follow to stream logs in
real time (like 'tail -f'). Control the number of historical lines shown
with --lines.

Examples:
  projectctl logs myapp                  # Last 50 lines, all services
  projectctl logs myapp -f               # Follow logs in real time
  projectctl logs myapp -s backend -f    # Follow only backend logs
  projectctl logs myapp -l 200           # Show last 200 lines
  projectctl logs myapp -s api -l 100 -f  # Follow API with 100-line history")]
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
    #[command(long_about = "\
Manage project dependencies across your registered projects.

Supports updating, checking for outdated packages, and viewing a summary
of dependency managers across all projects. Works with npm, pip, cargo,
and other detected package managers.

Examples:
  projectctl deps update myapp           # Update deps for one project
  projectctl deps update --all           # Update deps for all projects
  projectctl deps check myapp            # Check for outdated packages
  projectctl deps check --all            # Check all projects
  projectctl deps summary                # Overview of all dependency managers")]
    Deps {
        #[command(subcommand)]
        action: DepsAction,
    },

    /// Run a custom project command
    #[command(long_about = "\
Run a custom command defined in the project configuration.

Commands are defined per-project in ~/.projectctl/projects.toml under
[project.commands]. Use --list to see available commands for a project.
The command is executed in the project's root directory.

Examples:
  projectctl run myapp dev               # Run the 'dev' command
  projectctl run myapp test              # Run the 'test' command
  projectctl run myapp build             # Run the 'build' command
  projectctl run myapp --list            # List available commands
  projectctl run myapp                   # Also lists commands (no args)")]
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
    #[command(long_about = "\
Register an existing project directory with projectctl.

Auto-detects the project type (fastapi, react, tauri, rust, etc.),
available services (from docker-compose), and common commands. If --path
is omitted, the current directory is used. The project name defaults to
the directory name.

Examples:
  projectctl add --path ~/code/myapp     # Register with auto-detect
  projectctl add                         # Register current directory
  projectctl add --name api --path ~/code/backend  # Custom name
  projectctl add -p ~/code/app -t react  # Explicit type
  projectctl add -n myproject -p . -t fastapi  # All options")]
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
    #[command(long_about = "\
Remove a project from the projectctl registry.

This only unregisters the project from projectctl's tracking. It does NOT
delete the project directory or any files on disk. The project can be
re-added later with 'projectctl add'.

Examples:
  projectctl remove myapp                # Remove by exact name
  projectctl remove old-project          # Remove an unused project
  projectctl remove test-api             # Clean up test projects")]
    Remove {
        /// Project name
        name: String,
    },

    /// Show recently used projects
    #[command(long_about = "\
Show recently used projects sorted by last access time.

Displays projects ordered by when they were last switched to. Useful for
quickly finding the project you were working on. The --limit flag controls
how many entries to show.

Examples:
  projectctl recent                      # Show last 10 projects
  projectctl recent -l 5                 # Show last 5 projects
  projectctl recent --limit 20           # Show last 20 projects")]
    Recent {
        /// Maximum number of projects to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Create a new project from a template
    #[command(long_about = "\
Scaffold a new project from a built-in or custom template.

Creates a new directory with the project structure, configuration files,
and boilerplate code from the chosen template. The project is automatically
registered with projectctl after creation.

Examples:
  projectctl new myapp --template react-vite       # React + Vite project
  projectctl new api --template fastapi             # FastAPI project
  projectctl new desktop --template tauri           # Tauri desktop app
  projectctl new myapp -t react-vite -d ~/projects  # Custom target dir
  projectctl new cli -t rust                        # Rust CLI project")]
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
    #[command(long_about = "\
List and manage project templates.

View built-in templates or add custom templates from local directories.
Custom templates are stored in the projectctl configuration directory
and can be used with 'projectctl new'.

Examples:
  projectctl templates                   # List all available templates
  projectctl templates list              # Same as above
  projectctl templates add mytemplate --path ~/templates/react-custom
  projectctl templates add fastapi-full -p ~/templates/fastapi")]
    Templates {
        #[command(subcommand)]
        action: Option<TemplatesAction>,
    },

    /// Generate shell completions
    #[command(long_about = "\
Generate shell completion scripts for projectctl.

Outputs completion script to stdout. Redirect to the appropriate file for
your shell to enable tab-completion of commands, project names, and flags.

Examples:
  projectctl completions zsh >> ~/.zshrc           # Zsh completions
  projectctl completions bash >> ~/.bashrc         # Bash completions
  projectctl completions fish > ~/.config/fish/completions/projectctl.fish
  source <(projectctl completions zsh)             # Load for current session")]
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

fn main() -> Result<()> {
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
