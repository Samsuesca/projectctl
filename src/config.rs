use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::project::Project;

/// Global application configuration
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalConfig {
    #[serde(default)]
    pub editor: String,
    #[serde(default)]
    pub default_shell: String,
    #[serde(default)]
    pub auto_start_services: bool,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            editor: "code".to_string(),
            default_shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string()),
            auto_start_services: false,
        }
    }
}

/// The projects database file
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProjectsFile {
    #[serde(default)]
    pub project: Vec<Project>,
}

/// Main config manager
pub struct ConfigManager {
    config_dir: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Result<Self> {
        let config_dir = Self::config_dir()?;
        Ok(Self { config_dir })
    }

    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        Ok(home.join(".projectctl"))
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.config_dir)
            .context("Failed to create config directory")?;
        fs::create_dir_all(self.config_dir.join("templates"))
            .context("Failed to create templates directory")?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn config_path(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn projects_path(&self) -> PathBuf {
        self.config_dir.join("projects.toml")
    }

    pub fn templates_dir(&self) -> PathBuf {
        self.config_dir.join("templates")
    }

    #[allow(dead_code)]
    pub fn load_global_config(&self) -> Result<GlobalConfig> {
        let path = self.config_path();
        if !path.exists() {
            let config = GlobalConfig::default();
            self.save_global_config(&config)?;
            return Ok(config);
        }
        let content = fs::read_to_string(&path)
            .context("Failed to read config.toml")?;
        let config: GlobalConfig = toml::from_str(&content)
            .context("Failed to parse config.toml")?;
        Ok(config)
    }

    #[allow(dead_code)]
    pub fn save_global_config(&self, config: &GlobalConfig) -> Result<()> {
        self.ensure_dirs()?;
        let content = toml::to_string_pretty(config)
            .context("Failed to serialize config")?;
        fs::write(self.config_path(), content)
            .context("Failed to write config.toml")?;
        Ok(())
    }

    pub fn load_projects(&self) -> Result<Vec<Project>> {
        let path = self.projects_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&path)
            .context("Failed to read projects.toml")?;
        let projects_file: ProjectsFile = toml::from_str(&content)
            .context("Failed to parse projects.toml")?;
        Ok(projects_file.project)
    }

    pub fn save_projects(&self, projects: &[Project]) -> Result<()> {
        self.ensure_dirs()?;
        let projects_file = ProjectsFile {
            project: projects.to_vec(),
        };
        let content = toml::to_string_pretty(&projects_file)
            .context("Failed to serialize projects")?;
        fs::write(self.projects_path(), content)
            .context("Failed to write projects.toml")?;
        Ok(())
    }

    pub fn find_project<'a>(&self, projects: &'a [Project], name: &str) -> Option<&'a Project> {
        let name_lower = name.to_lowercase();
        // Exact match first
        if let Some(p) = projects.iter().find(|p| p.name.to_lowercase() == name_lower) {
            return Some(p);
        }
        // Prefix match (alias-like)
        let matches: Vec<&Project> = projects
            .iter()
            .filter(|p| p.name.to_lowercase().starts_with(&name_lower))
            .collect();
        if matches.len() == 1 {
            return Some(matches[0]);
        }
        // Contains match
        let matches: Vec<&Project> = projects
            .iter()
            .filter(|p| p.name.to_lowercase().contains(&name_lower))
            .collect();
        if matches.len() == 1 {
            return Some(matches[0]);
        }
        None
    }

    #[allow(dead_code)]
    pub fn find_project_mut<'a>(
        &self,
        projects: &'a mut [Project],
        name: &str,
    ) -> Option<&'a mut Project> {
        let name_lower = name.to_lowercase();
        // Exact match first
        let exact = projects
            .iter()
            .position(|p| p.name.to_lowercase() == name_lower);
        if let Some(idx) = exact {
            return Some(&mut projects[idx]);
        }
        // Prefix match
        let prefix_matches: Vec<usize> = projects
            .iter()
            .enumerate()
            .filter(|(_, p)| p.name.to_lowercase().starts_with(&name_lower))
            .map(|(i, _)| i)
            .collect();
        if prefix_matches.len() == 1 {
            return Some(&mut projects[prefix_matches[0]]);
        }
        // Contains match
        let contains_matches: Vec<usize> = projects
            .iter()
            .enumerate()
            .filter(|(_, p)| p.name.to_lowercase().contains(&name_lower))
            .map(|(i, _)| i)
            .collect();
        if contains_matches.len() == 1 {
            return Some(&mut projects[contains_matches[0]]);
        }
        None
    }

    /// Expand a project path (handles ~ and env vars)
    pub fn expand_path(path: &str) -> PathBuf {
        let expanded = shellexpand::tilde(path);
        PathBuf::from(expanded.as_ref())
    }
}
