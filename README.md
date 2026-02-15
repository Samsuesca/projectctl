# projectctl

> Project switcher and development environment manager

![macOS](https://img.shields.io/badge/macOS-Apple_Silicon-blue)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange)
![License](https://img.shields.io/badge/license-MIT-green)

**projectctl** is a command-line tool for rapid project switching, environment management, and development workflow automation across multiple codebases.

---

## Features

- **Instant Project Switching**: Change projects with all context (venv, Docker, editor)
- **Environment Management**: Auto-activate Python venvs, Node versions, etc.
- **Service Orchestration**: Start/stop Docker Compose, databases, dev servers
- **Status Dashboard**: Git status, dependencies, running services at a glance
- **Dependency Management**: Update npm/pip/cargo across all projects
- **Custom Commands**: Per-project dev, test, deploy shortcuts
- **Recent Projects**: Quick access to recently used projects
- **Project Templates**: Bootstrap new projects with templates

---

## Installation

```bash
git clone https://github.com/Samsuesca/projectctl.git
cd projectctl
cargo build --release
cargo install --path .
```

---

## Usage

### List Projects

```bash
# List all registered projects
projectctl list

# Show detailed information
projectctl list --detailed

# Filter by type
projectctl list --type rust

# Show only active projects
projectctl list --active
```

**Output:**
```
Registered Projects:

┌────┬──────────────────────┬──────────┬────────────┬─────────────┐
│ #  │ Name                 │ Type     │ Status     │ Last Used   │
├────┼──────────────────────┼──────────┼────────────┼─────────────┤
│ 1  │ uniformes-system-v2  │ FastAPI  │ ✅ Running │ 2 hours ago │
│ 2  │ ramctl               │ Rust     │ 💤 Idle    │ 1 day ago   │
│ 3  │ portfolio-pos-system │ FastAPI  │ 💤 Idle    │ 3 days ago  │
│ 4  │ wristband            │ FastAPI  │ 💤 Idle    │ 1 week ago  │
│ 5  │ in-flow              │ Tauri    │ 💤 Idle    │ 2 weeks ago │
│ 6  │ EconIntuition        │ Next.js  │ 💤 Idle    │ 1 month ago │
└────┴──────────────────────┴──────────┴────────────┴─────────────┘

Total: 6 projects | Active: 1 | Rust: 1 | FastAPI: 3 | Tauri: 1 | Next.js: 1
```

### Switch Project

```bash
# Switch to project
projectctl switch uniformes-system-v2

# Switch with alias
projectctl switch uniformes

# Switch to most recent
projectctl switch --recent

# Switch and open VSCode
projectctl switch ramctl --code
```

**Output:**
```
Switching to: uniformes-system-v2

📂 Changed directory
   /Users/angelsamuelsuescarios/Documents/03_Proyectos/Codigo/uniformes-system-v2

🐍 Activated Python venv
   venv (Python 3.11.5)

🐳 Starting Docker services...
   ✓ postgres (port 5432)
   ✓ redis (port 6379)

📦 Dependencies up to date
   Last updated: 2 days ago

🌿 Git status
   Branch: main
   Status: clean

💻 Opening VSCode...

✨ Ready to develop!
```

### Project Info

```bash
# Show project details
projectctl info uniformes-system-v2

# Show git status
projectctl info ramctl --git

# Show dependencies
projectctl info ramctl --deps
```

**Output:**
```
Project: uniformes-system-v2
Path: ~/Documents/03_Proyectos/Codigo/uniformes-system-v2
Type: FastAPI + React

Git:
  Branch:        main
  Status:        3 files changed
  Unpushed:      2 commits
  Last commit:   feat: add timezone utils (2 hours ago)

Services:
  ✅ postgres:15     (port 5432, healthy)
  ✅ redis:latest    (port 6379, healthy)
  ✅ backend         (port 8000, healthy)
  ⚠️  frontend       (not running)

Environment:
  Python:        3.11.5 (venv active)
  Node:          v20.11.0
  Database:      postgresql://localhost/uniformes_db

Dependencies:
  Backend:       147 packages (2 outdated)
  Frontend:      234 packages (8 outdated)
  Last update:   2 days ago

Disk Usage:     2.3 GB
```

### Start/Stop Services

```bash
# Start all project services
projectctl start uniformes-system-v2

# Start specific service
projectctl start uniformes --service backend

# Stop all services
projectctl stop uniformes

# Restart services
projectctl restart uniformes
```

**Output:**
```
Starting services for: uniformes-system-v2

🐳 Docker Compose
   ✓ postgres started (port 5432)
   ✓ redis started (port 6379)

🚀 Dev Servers
   ✓ Backend started (port 8000)
     → http://localhost:8000/docs
   ✓ Frontend started (port 5173)
     → http://localhost:5173

✨ All services running!
```

### Logs

```bash
# Tail logs for all services
projectctl logs uniformes

# Follow specific service
projectctl logs uniformes --service backend --follow

# Last N lines
projectctl logs uniformes --lines 100
```

### Dependency Management

```bash
# Update dependencies for one project
projectctl deps update uniformes

# Update all projects
projectctl deps update --all

# Check for outdated packages
projectctl deps check uniformes

# Show dependency summary
projectctl deps summary
```

**Output:**
```
Updating dependencies: uniformes-system-v2

Backend (Python):
  ⬆️  fastapi: 0.109.0 → 0.110.0
  ⬆️  pydantic: 2.5.3 → 2.6.1
  ✓ Installed 2 updates

Frontend (Node):
  ⬆️  react: 18.2.0 → 18.3.0
  ⬆️  react-router-dom: 6.21.0 → 6.22.0
  ⬆️  vite: 5.0.11 → 5.1.0
  ⚠️  @types/node: 20.10.0 → 20.11.0 (breaking)
  ✓ Installed 3 updates (1 requires review)

✨ Dependencies updated!
Run tests to verify: projectctl run uniformes test
```

### Custom Commands

```bash
# Run custom command
projectctl run uniformes dev        # Start dev server
projectctl run uniformes test       # Run tests
projectctl run uniformes deploy     # Deploy (custom script)

# List available commands for project
projectctl run uniformes --list
```

**Output:**
```
Running: uniformes-system-v2 dev

Executing: docker compose up -d && cd backend && uvicorn app.main:app --reload

[Backend] INFO:     Uvicorn running on http://127.0.0.1:8000
[Backend] INFO:     Application startup complete
[Backend] INFO:     Watching for file changes...
```

### Add/Remove Projects

```bash
# Add current directory as project
projectctl add

# Add with custom name
projectctl add --name my-project --path ~/code/my-project

# Add with type
projectctl add --type rust --path ~/code/my-rust-app

# Remove project
projectctl remove uniformes
```

### Recent Projects

```bash
# Show recent projects
projectctl recent

# Limit
projectctl recent --limit 5

# Quick switch to recent
projectctl switch --recent
```

**Output:**
```
Recent Projects:

1. uniformes-system-v2  (2 hours ago)
2. ramctl               (1 day ago)
3. portfolio-pos-system (3 days ago)
4. statsctl             (5 days ago)
5. notectl              (1 week ago)

Switch: projectctl switch <name>
```

### Project Templates

```bash
# Create new project from template
projectctl new my-api --template fastapi

# List templates
projectctl templates

# Add custom template
projectctl templates add rust-cli --path ~/templates/rust-cli
```

**Built-in Templates:**
- `fastapi` - FastAPI + PostgreSQL + Redis
- `react-vite` - React + TypeScript + Tailwind + Vite
- `tauri` - Tauri + React + TypeScript
- `rust-cli` - Rust CLI with clap
- `nextjs` - Next.js 14 App Router

---

## Command Reference

| Command | Description | Options |
|---------|-------------|---------|
| `list` | List projects | `--detailed`, `--type`, `--active` |
| `switch` | Switch to project | `--recent`, `--code` |
| `info` | Project details | `--git`, `--deps` |
| `start` | Start services | `--service` |
| `stop` | Stop services | `--service` |
| `restart` | Restart services | `--service` |
| `logs` | View logs | `--service`, `--follow`, `--lines` |
| `deps` | Manage dependencies | `update`, `check`, `summary` |
| `run` | Run custom command | `--list` |
| `add` | Add project | `--name`, `--path`, `--type` |
| `remove` | Remove project | - |
| `recent` | Recent projects | `--limit` |
| `new` | Create from template | `--template` |
| `templates` | Manage templates | `add`, `list` |

---

## Use Cases

### Daily Workflow

```bash
# Morning: Check active projects
projectctl list --active

# Switch to today's project
projectctl switch uniformes-system-v2

# Services auto-start, venv activates, VSCode opens

# Start developing...
```

### Multi-Project Development

```bash
# Working on uniformes, need to check portfolio
projectctl switch portfolio-pos-system

# Quick fix, switch back
projectctl switch uniformes
```

### Dependency Maintenance

```bash
# Weekly: Update all projects
projectctl deps check --all

# Update one by one
projectctl deps update uniformes
projectctl run uniformes test

projectctl deps update portfolio
projectctl run portfolio test
```

### New Project Setup

```bash
# Create new FastAPI project
projectctl new my-new-api --template fastapi

# Automatically:
# - Creates directory structure
# - Initializes git
# - Creates venv
# - Installs dependencies
# - Generates docker-compose.yml
# - Sets up .env.example

projectctl switch my-new-api
# Ready to develop!
```

---

## Technical Stack

**Language**: Rust 2021 edition

**Dependencies**:
- `clap` - CLI parsing
- `serde` / `serde_json` - Configuration
- `toml` - Config file format
- `git2` - Git operations
- `tokio` - Async runtime
- `colored` - Terminal colors
- `tabled` - Table formatting
- `shellexpand` - Path expansion

---

## Architecture

```
src/
├── main.rs           # CLI entry point
├── config.rs         # Project configuration
├── project.rs        # Project struct and operations
├── services.rs       # Docker/service management
├── deps.rs           # Dependency management
├── git.rs            # Git operations
├── templates.rs      # Project templates
└── display.rs        # Formatted output
```

**Configuration:**
```
~/.projectctl/
├── config.toml       # Global settings
├── projects.toml     # Registered projects
└── templates/        # Custom templates
```

**`projects.toml` example:**
```toml
[[project]]
name = "uniformes-system-v2"
path = "~/Documents/03_Proyectos/Codigo/uniformes-system-v2"
type = "fastapi"
services = ["postgres", "redis", "backend", "frontend"]

[project.env]
python = "3.11"
node = "20"

[project.commands]
dev = "docker compose up -d && cd backend && uvicorn app.main:app --reload"
test = "cd backend && pytest"
deploy = "./scripts/deploy.sh"

[[project]]
name = "ramctl"
path = "~/Documents/03_Proyectos/Codigo/ram_manager_cli"
type = "rust"

[project.commands]
dev = "cargo run"
test = "cargo test"
build = "cargo build --release"
```

---

## Implementation Notes

### Auto-Detection

When adding a project, auto-detect:
- **Type**: Check for `Cargo.toml`, `package.json`, `pyproject.toml`, etc.
- **Services**: Check for `docker-compose.yml`
- **Environment**: Check for `venv/`, `.nvmrc`, `rust-toolchain.toml`

### Service Management

Use `docker compose` CLI for orchestration:
```bash
docker compose -f /path/to/project/docker-compose.yml up -d
```

Track PIDs of dev servers started by `projectctl`.

### Shell Integration

Provide shell functions for seamless switching:

**Zsh/Bash:**
```bash
# ~/.zshrc or ~/.bashrc
pcd() {
  local project_path=$(projectctl info "$1" --path-only)
  if [ -n "$project_path" ]; then
    cd "$project_path"
    # Auto-activate venv, etc.
  fi
}
```

### Environment Activation

- **Python**: Activate venv automatically
- **Node**: Use `nvm use` or `.nvmrc`
- **Rust**: No action needed (cargo handles)

---

## Platform Support

| Platform | Support |
|----------|---------|
| macOS (Apple Silicon) | ✅ Full support |
| macOS (Intel) | ✅ Full support |
| Linux | ✅ Full support |
| Windows | ⚠️ Partial (Docker/WSL) |

---

## Roadmap

- [ ] Integration with tmux/screen (auto-create sessions)
- [ ] Remote projects (SSH integration)
- [ ] Cloud deployment shortcuts (AWS, Vercel, etc.)
- [ ] Timetracking integration (track time per project)
- [ ] Backup/restore project configs
- [ ] AI-assisted command suggestions

---

## License

MIT License

---

## Author

**Angel Samuel Suesca Ríos**
suescapsam@gmail.com

---

## Shell Integration

Add to `~/.zshrc`:

```bash
# Quick project switch
alias p='projectctl switch'
alias pl='projectctl list'
alias pi='projectctl info'
alias pr='projectctl run'

# Recent projects
alias p1='projectctl switch --recent 1'
alias p2='projectctl switch --recent 2'
alias p3='projectctl switch --recent 3'

# Auto-completion (if implemented)
eval "$(projectctl completions zsh)"
```

---

**Perfect for**: Developers juggling multiple projects, anyone tired of manual environment setup, teams with standardized project structures.
