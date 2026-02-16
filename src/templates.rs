use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::ConfigManager;

/// Built-in template definition
pub struct BuiltinTemplate {
    pub name: &'static str,
    pub description: &'static str,
}

pub const BUILTIN_TEMPLATES: &[BuiltinTemplate] = &[
    BuiltinTemplate {
        name: "fastapi",
        description: "FastAPI + PostgreSQL + Redis",
    },
    BuiltinTemplate {
        name: "react-vite",
        description: "React + TypeScript + Tailwind + Vite",
    },
    BuiltinTemplate {
        name: "tauri",
        description: "Tauri + React + TypeScript",
    },
    BuiltinTemplate {
        name: "rust-cli",
        description: "Rust CLI with clap",
    },
    BuiltinTemplate {
        name: "nextjs",
        description: "Next.js App Router",
    },
];

/// List all available templates (built-in + custom)
pub fn list_templates(config: &ConfigManager) -> Result<()> {
    println!("{}\n", "Available Templates".bold().underline());

    println!("  {}", "Built-in:".bold());
    for tmpl in BUILTIN_TEMPLATES {
        println!("    {} - {}", tmpl.name.cyan(), tmpl.description);
    }

    let templates_dir = config.templates_dir();
    if templates_dir.exists() {
        let mut custom = Vec::new();
        for entry in fs::read_dir(&templates_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                custom.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        if !custom.is_empty() {
            println!("\n  {}", "Custom:".bold());
            for name in &custom {
                println!("    {} ({})", name.cyan(), templates_dir.join(name).display());
            }
        }
    }

    Ok(())
}

/// Add a custom template from a directory
pub fn add_template(config: &ConfigManager, name: &str, source_path: &str) -> Result<()> {
    let source = ConfigManager::expand_path(source_path);
    if !source.is_dir() {
        bail!("Source path is not a directory: {}", source_path);
    }

    let dest = config.templates_dir().join(name);
    if dest.exists() {
        bail!("Template '{}' already exists. Remove it first.", name);
    }

    config.ensure_dirs()?;
    copy_dir_recursive(&source, &dest)?;

    println!(
        "{} Template '{}' added from {}",
        "✓".green(),
        name.cyan(),
        source_path
    );
    Ok(())
}

/// Create a new project from a template
pub fn create_from_template(
    name: &str,
    template: &str,
    target_dir: Option<&str>,
) -> Result<PathBuf> {
    let target = match target_dir {
        Some(dir) => ConfigManager::expand_path(dir).join(name),
        None => std::env::current_dir()?.join(name),
    };

    if target.exists() {
        bail!("Directory already exists: {}", target.display());
    }

    println!(
        "Creating project '{}' from template '{}'\n",
        name.cyan().bold(),
        template.cyan()
    );

    // Check custom templates first
    let config = ConfigManager::new()?;
    let custom_template = config.templates_dir().join(template);
    if custom_template.is_dir() {
        copy_dir_recursive(&custom_template, &target)?;
        println!("  {} Copied custom template files", "✓".green());
    } else {
        create_builtin_template(template, &target, name)?;
    }

    // Initialize git
    init_git(&target)?;

    println!(
        "\n{} Project '{}' created at {}",
        "✓".green().bold(),
        name,
        target.display()
    );
    println!(
        "\nNext steps:\n  cd {}\n  projectctl add\n",
        target.display()
    );

    Ok(target)
}

fn create_builtin_template(template: &str, target: &Path, name: &str) -> Result<()> {
    fs::create_dir_all(target)?;

    match template {
        "fastapi" => create_fastapi(target, name)?,
        "react-vite" => create_react_vite(target, name)?,
        "rust-cli" => create_rust_cli(target, name)?,
        "nextjs" => create_nextjs(target, name)?,
        "tauri" => create_tauri(target, name)?,
        _ => bail!(
            "Unknown template '{}'. Use 'projectctl templates' to list available.",
            template
        ),
    }

    Ok(())
}

// --- FastAPI Template ---

fn create_fastapi(target: &Path, name: &str) -> Result<()> {
    for dir in &["app", "app/api", "app/models", "app/schemas", "tests"] {
        fs::create_dir_all(target.join(dir))?;
    }

    fs::write(target.join("app/__init__.py"), "")?;
    fs::write(
        target.join("app/main.py"),
        format!(
            r#"from fastapi import FastAPI

app = FastAPI(title="{name}", version="0.1.0")


@app.get("/")
async def root():
    return {{"message": "Welcome to {name}"}}


@app.get("/health")
async def health():
    return {{"status": "healthy"}}
"#
        ),
    )?;
    fs::write(target.join("app/api/__init__.py"), "")?;
    fs::write(target.join("app/models/__init__.py"), "")?;
    fs::write(target.join("app/schemas/__init__.py"), "")?;

    fs::write(
        target.join("requirements.txt"),
        "fastapi>=0.110.0\nuvicorn[standard]>=0.27.0\npydantic>=2.6.0\nsqlalchemy>=2.0.0\nalembic>=1.13.0\npytest>=8.0.0\nhttpx>=0.27.0\n",
    )?;

    fs::write(
        target.join("docker-compose.yml"),
        format!(
            r#"services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_DB: {name}_db
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"

volumes:
  pgdata:
"#
        ),
    )?;

    fs::write(
        target.join(".env.example"),
        format!("DATABASE_URL=postgresql://postgres:postgres@localhost:5432/{name}_db\nREDIS_URL=redis://localhost:6379\nSECRET_KEY=changeme\n"),
    )?;

    fs::write(
        target.join(".gitignore"),
        "venv/\n.venv/\n__pycache__/\n*.pyc\n.env\n*.egg-info/\ndist/\nbuild/\n.pytest_cache/\n",
    )?;

    fs::write(target.join("tests/__init__.py"), "")?;
    fs::write(
        target.join("tests/test_main.py"),
        r#"from fastapi.testclient import TestClient
from app.main import app

client = TestClient(app)


def test_root():
    response = client.get("/")
    assert response.status_code == 200


def test_health():
    response = client.get("/health")
    assert response.status_code == 200
    assert response.json()["status"] == "healthy"
"#,
    )?;

    println!("  {} Created FastAPI project structure", "✓".green());
    println!("  {} Created docker-compose.yml (PostgreSQL + Redis)", "✓".green());
    println!("  {} Created requirements.txt", "✓".green());
    Ok(())
}

// --- React + Vite Template ---

fn create_react_vite(target: &Path, name: &str) -> Result<()> {
    fs::create_dir_all(target.join("src"))?;
    fs::create_dir_all(target.join("public"))?;

    fs::write(
        target.join("package.json"),
        format!(
            r#"{{
  "name": "{name}",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "test": "vitest"
  }},
  "dependencies": {{
    "react": "^18.3.0",
    "react-dom": "^18.3.0"
  }},
  "devDependencies": {{
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.2.0",
    "typescript": "^5.4.0",
    "vite": "^5.2.0",
    "vitest": "^1.4.0",
    "tailwindcss": "^3.4.0",
    "autoprefixer": "^10.4.0",
    "postcss": "^8.4.0"
  }}
}}
"#
        ),
    )?;

    fs::write(
        target.join("src/App.tsx"),
        format!(
            r#"function App() {{
  return (
    <div className="min-h-screen flex items-center justify-center">
      <h1 className="text-4xl font-bold">Welcome to {name}</h1>
    </div>
  )
}}

export default App
"#
        ),
    )?;

    fs::write(
        target.join("src/main.tsx"),
        r#"import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
"#,
    )?;

    fs::write(
        target.join("index.html"),
        format!(
            r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{name}</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
"#
        ),
    )?;

    fs::write(target.join(".gitignore"), "node_modules/\ndist/\n.env\n")?;

    println!("  {} Created React + Vite + TypeScript project", "✓".green());
    Ok(())
}

// --- Rust CLI Template ---

fn create_rust_cli(target: &Path, name: &str) -> Result<()> {
    fs::create_dir_all(target.join("src"))?;

    fs::write(
        target.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = {{ version = "4", features = ["derive"] }}
anyhow = "1"
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
colored = "2"
"#
        ),
    )?;

    fs::write(
        target.join("src/main.rs"),
        format!(
            r#"use clap::Parser;

#[derive(Parser)]
#[command(name = "{name}", about = "A CLI tool", version)]
struct Cli {{
    #[command(subcommand)]
    command: Commands,
}}

#[derive(clap::Subcommand)]
enum Commands {{
    /// Say hello
    Hello {{
        #[arg(short, long, default_value = "World")]
        name: String,
    }},
}}

fn main() -> anyhow::Result<()> {{
    let cli = Cli::parse();

    match cli.command {{
        Commands::Hello {{ name }} => {{
            println!("Hello, {{}}!", name);
        }}
    }}

    Ok(())
}}
"#
        ),
    )?;

    fs::write(target.join(".gitignore"), "target/\n")?;

    println!("  {} Created Rust CLI project with clap", "✓".green());
    Ok(())
}

// --- Next.js Template ---

fn create_nextjs(target: &Path, name: &str) -> Result<()> {
    fs::create_dir_all(target.join("app"))?;
    fs::create_dir_all(target.join("public"))?;

    fs::write(
        target.join("package.json"),
        format!(
            r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "private": true,
  "scripts": {{
    "dev": "next dev",
    "build": "next build",
    "start": "next start",
    "lint": "next lint"
  }},
  "dependencies": {{
    "next": "^14.1.0",
    "react": "^18.3.0",
    "react-dom": "^18.3.0"
  }},
  "devDependencies": {{
    "@types/node": "^20.11.0",
    "@types/react": "^18.3.0",
    "typescript": "^5.4.0"
  }}
}}
"#
        ),
    )?;

    fs::write(
        target.join("app/layout.tsx"),
        format!(
            r#"export const metadata = {{
  title: '{name}',
  description: 'Created with projectctl',
}}

export default function RootLayout({{
  children,
}}: {{
  children: React.ReactNode
}}) {{
  return (
    <html lang="en">
      <body>{{children}}</body>
    </html>
  )
}}
"#
        ),
    )?;

    fs::write(
        target.join("app/page.tsx"),
        format!(
            r#"export default function Home() {{
  return (
    <main>
      <h1>Welcome to {name}</h1>
    </main>
  )
}}
"#
        ),
    )?;

    fs::write(target.join(".gitignore"), "node_modules/\n.next/\nout/\n.env\n")?;

    println!("  {} Created Next.js App Router project", "✓".green());
    Ok(())
}

// --- Tauri Template ---

fn create_tauri(target: &Path, name: &str) -> Result<()> {
    fs::create_dir_all(target.join("src-tauri/src"))?;
    fs::create_dir_all(target.join("src"))?;

    fs::write(
        target.join("package.json"),
        format!(
            r#"{{
  "name": "{name}",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "dev": "vite",
    "build": "tsc && vite build",
    "tauri": "tauri"
  }},
  "dependencies": {{
    "react": "^18.3.0",
    "react-dom": "^18.3.0",
    "@tauri-apps/api": "^1.5.0"
  }},
  "devDependencies": {{
    "@types/react": "^18.3.0",
    "@vitejs/plugin-react": "^4.2.0",
    "typescript": "^5.4.0",
    "vite": "^5.2.0",
    "@tauri-apps/cli": "^1.5.0"
  }}
}}
"#
        ),
    )?;

    fs::write(
        target.join("src-tauri/Cargo.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = {{ version = "1", features = [] }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
"#
        ),
    )?;

    fs::write(
        target.join("src-tauri/src/main.rs"),
        r#"#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
"#,
    )?;

    fs::write(
        target.join("src/App.tsx"),
        format!(
            r#"function App() {{
  return (
    <div>
      <h1>Welcome to {name}</h1>
    </div>
  )
}}

export default App
"#
        ),
    )?;

    fs::write(target.join(".gitignore"), "node_modules/\ntarget/\ndist/\n.env\n")?;

    println!("  {} Created Tauri + React project", "✓".green());
    Ok(())
}

fn init_git(target: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["init"])
        .current_dir(target)
        .output()
        .context("Failed to initialize git")?;

    if output.status.success() {
        println!("  {} Initialized git repository", "✓".green());
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ft.is_dir() {
            if entry.file_name() == ".git" {
                continue;
            }
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
