use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::ConfigManager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub name: String,
    pub path: String,
    #[serde(default = "default_project_type")]
    #[serde(rename = "type")]
    pub project_type: String,
    #[serde(default)]
    pub services: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub commands: HashMap<String, String>,
    #[serde(default)]
    pub last_used: Option<String>,
}

fn default_project_type() -> String {
    "unknown".to_string()
}

impl Project {
    pub fn new(name: String, path: String, project_type: String) -> Self {
        Self {
            name,
            path,
            project_type,
            services: Vec::new(),
            env: HashMap::new(),
            commands: HashMap::new(),
            last_used: Some(Utc::now().to_rfc3339()),
        }
    }

    /// Get the expanded absolute path
    pub fn expanded_path(&self) -> PathBuf {
        ConfigManager::expand_path(&self.path)
    }

    /// Check if the project directory exists
    pub fn exists(&self) -> bool {
        self.expanded_path().is_dir()
    }

    /// Update the last_used timestamp to now
    pub fn touch(&mut self) {
        self.last_used = Some(Utc::now().to_rfc3339());
    }

    /// Parse the last_used timestamp
    pub fn last_used_time(&self) -> Option<DateTime<Utc>> {
        self.last_used.as_ref().and_then(|s| s.parse().ok())
    }

    /// Get a human-readable "time ago" string
    pub fn last_used_ago(&self) -> String {
        match self.last_used_time() {
            Some(dt) => {
                let duration = Utc::now().signed_duration_since(dt);
                if duration.num_minutes() < 1 {
                    "just now".to_string()
                } else if duration.num_minutes() < 60 {
                    format!("{} min ago", duration.num_minutes())
                } else if duration.num_hours() < 24 {
                    let h = duration.num_hours();
                    format!("{} hour{} ago", h, if h == 1 { "" } else { "s" })
                } else if duration.num_days() < 7 {
                    let d = duration.num_days();
                    format!("{} day{} ago", d, if d == 1 { "" } else { "s" })
                } else if duration.num_weeks() < 4 {
                    let w = duration.num_weeks();
                    format!("{} week{} ago", w, if w == 1 { "" } else { "s" })
                } else {
                    let m = duration.num_days() / 30;
                    if m < 1 {
                        "1 month ago".to_string()
                    } else {
                        format!("{} month{} ago", m, if m == 1 { "" } else { "s" })
                    }
                }
            }
            None => "never".to_string(),
        }
    }

    /// Has a docker-compose file?
    pub fn has_docker_compose(&self) -> bool {
        let path = self.expanded_path();
        path.join("docker-compose.yml").exists()
            || path.join("docker-compose.yaml").exists()
            || path.join("compose.yml").exists()
            || path.join("compose.yaml").exists()
    }

    /// Has a Python virtual environment?
    pub fn has_venv(&self) -> bool {
        let path = self.expanded_path();
        path.join("venv").exists()
            || path.join(".venv").exists()
            || path.join("env").exists()
    }

    /// Get the venv path if it exists
    pub fn venv_path(&self) -> Option<PathBuf> {
        let path = self.expanded_path();
        for dir in &["venv", ".venv", "env"] {
            let venv = path.join(dir);
            if venv.exists() {
                return Some(venv);
            }
        }
        None
    }

    /// Has a .nvmrc or .node-version file?
    pub fn has_node_version(&self) -> bool {
        let path = self.expanded_path();
        path.join(".nvmrc").exists() || path.join(".node-version").exists()
    }

    /// Detect project type from files in the directory
    pub fn detect_type(path: &Path) -> String {
        if path.join("Cargo.toml").exists() {
            if path.join("src-tauri").exists() {
                return "tauri".to_string();
            }
            return "rust".to_string();
        }
        if path.join("pyproject.toml").exists() || path.join("setup.py").exists() {
            if path.join("requirements.txt").exists() || path.join("pyproject.toml").exists() {
                // Check for FastAPI
                if let Ok(content) = std::fs::read_to_string(path.join("requirements.txt")) {
                    if content.to_lowercase().contains("fastapi") {
                        return "fastapi".to_string();
                    }
                    if content.to_lowercase().contains("django") {
                        return "django".to_string();
                    }
                    if content.to_lowercase().contains("flask") {
                        return "flask".to_string();
                    }
                }
                if let Ok(content) = std::fs::read_to_string(path.join("pyproject.toml")) {
                    if content.to_lowercase().contains("fastapi") {
                        return "fastapi".to_string();
                    }
                    if content.to_lowercase().contains("django") {
                        return "django".to_string();
                    }
                    if content.to_lowercase().contains("flask") {
                        return "flask".to_string();
                    }
                }
            }
            return "python".to_string();
        }
        if path.join("package.json").exists() {
            if let Ok(content) = std::fs::read_to_string(path.join("package.json")) {
                let lower = content.to_lowercase();
                if lower.contains("\"next\"") {
                    return "nextjs".to_string();
                }
                if lower.contains("\"nuxt\"") {
                    return "nuxt".to_string();
                }
                if lower.contains("\"react\"") {
                    if lower.contains("\"vite\"") {
                        return "react-vite".to_string();
                    }
                    return "react".to_string();
                }
                if lower.contains("\"vue\"") {
                    return "vue".to_string();
                }
                if lower.contains("\"svelte\"") {
                    return "svelte".to_string();
                }
                if lower.contains("\"express\"") {
                    return "express".to_string();
                }
            }
            return "node".to_string();
        }
        if path.join("go.mod").exists() {
            return "go".to_string();
        }
        if path.join("pom.xml").exists() || path.join("build.gradle").exists() {
            return "java".to_string();
        }
        "unknown".to_string()
    }

    /// Detect services from docker-compose.yml
    pub fn detect_services(path: &Path) -> Vec<String> {
        let compose_files = [
            "docker-compose.yml",
            "docker-compose.yaml",
            "compose.yml",
            "compose.yaml",
        ];
        for file in &compose_files {
            let compose_path = path.join(file);
            if compose_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&compose_path) {
                    return parse_compose_services(&content);
                }
            }
        }
        Vec::new()
    }

    /// Detect common commands based on project type
    pub fn detect_commands(path: &Path, project_type: &str) -> HashMap<String, String> {
        let mut commands = HashMap::new();
        match project_type {
            "rust" => {
                commands.insert("dev".to_string(), "cargo run".to_string());
                commands.insert("test".to_string(), "cargo test".to_string());
                commands.insert("build".to_string(), "cargo build --release".to_string());
            }
            "fastapi" | "python" => {
                if path.join("manage.py").exists() {
                    commands.insert("dev".to_string(), "python manage.py runserver".to_string());
                    commands.insert("test".to_string(), "python manage.py test".to_string());
                } else {
                    commands.insert("dev".to_string(), "uvicorn app.main:app --reload".to_string());
                    commands.insert("test".to_string(), "pytest".to_string());
                }
            }
            "django" => {
                commands.insert("dev".to_string(), "python manage.py runserver".to_string());
                commands.insert("test".to_string(), "python manage.py test".to_string());
            }
            "nextjs" => {
                commands.insert("dev".to_string(), "npm run dev".to_string());
                commands.insert("build".to_string(), "npm run build".to_string());
                commands.insert("test".to_string(), "npm test".to_string());
            }
            "react-vite" | "react" | "vue" | "svelte" => {
                commands.insert("dev".to_string(), "npm run dev".to_string());
                commands.insert("build".to_string(), "npm run build".to_string());
                commands.insert("test".to_string(), "npm test".to_string());
            }
            "node" | "express" => {
                commands.insert("dev".to_string(), "npm run dev".to_string());
                commands.insert("start".to_string(), "npm start".to_string());
                commands.insert("test".to_string(), "npm test".to_string());
            }
            "tauri" => {
                commands.insert("dev".to_string(), "cargo tauri dev".to_string());
                commands.insert("build".to_string(), "cargo tauri build".to_string());
                commands.insert("test".to_string(), "cargo test".to_string());
            }
            "go" => {
                commands.insert("dev".to_string(), "go run .".to_string());
                commands.insert("test".to_string(), "go test ./...".to_string());
                commands.insert("build".to_string(), "go build -o bin/app .".to_string());
            }
            _ => {}
        }
        commands
    }
}

/// Simple docker-compose service parser
fn parse_compose_services(content: &str) -> Vec<String> {
    let mut services = Vec::new();
    let mut in_services = false;
    let mut service_indent: Option<usize> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        // Detect the "services:" key
        if trimmed == "services:" {
            in_services = true;
            service_indent = None;
            continue;
        }

        if in_services {
            let indent = line.len() - line.trim_start().len();
            if let Some(si) = service_indent {
                if indent <= 0 && !trimmed.is_empty() {
                    // Back to top level
                    break;
                }
                if indent == si && trimmed.ends_with(':') {
                    let name = trimmed.trim_end_matches(':').trim();
                    if !name.is_empty() {
                        services.push(name.to_string());
                    }
                }
            } else if trimmed.ends_with(':') && indent > 0 {
                service_indent = Some(indent);
                let name = trimmed.trim_end_matches(':').trim();
                if !name.is_empty() {
                    services.push(name.to_string());
                }
            }
        }
    }
    services
}
