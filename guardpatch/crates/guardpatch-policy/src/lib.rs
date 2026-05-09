use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;
use anyhow::Context;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GuardMode {
    Editable,
    ProposalOnly,
    ReviewRequired,
    Protected,
    Frozen,
    AppendOnly,
    Generated,
    HumanOnly,
    Deprecated,
}

impl Default for GuardMode {
    fn default() -> Self {
        GuardMode::Editable
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProjectConfig {
    pub name: String,
    #[serde(default)]
    pub mode: GuardMode,
    #[serde(default)]
    pub locs_required_for_new_files: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PathRule {
    pub pattern: String,
    pub mode: GuardMode,
}

/// Per-agent authority profile. Agents are identified by name (e.g. "frontend_agent").
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AgentProfile {
    pub name: String,
    /// Glob patterns this agent is allowed to edit.
    #[serde(default)]
    pub allow: Vec<String>,
    /// Glob patterns this agent is explicitly denied from editing.
    #[serde(default)]
    pub deny: Vec<String>,
    /// Optional default mode override for this agent.
    #[serde(default)]
    pub default_mode: Option<GuardMode>,
    /// If true, this agent can only propose patches, never apply them directly.
    #[serde(default)]
    pub proposal_only: bool,
}

/// Promotion evidence requirements.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PromotionRequirements {
    #[serde(default)]
    pub user_approval: bool,
    #[serde(default)]
    pub tests_passed: bool,
    #[serde(default)]
    pub typecheck_passed: bool,
    #[serde(default)]
    pub no_changes_for_commits: Option<usize>,
    #[serde(default)]
    pub docs_synced: bool,
    #[serde(default)]
    pub release_tagged: bool,
}

/// Promotion config from .guardpatch.yml.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PromotionConfig {
    #[serde(default)]
    pub to_stable: Option<PromotionRequirements>,
    #[serde(default)]
    pub to_protected: Option<PromotionRequirements>,
    #[serde(default)]
    pub to_frozen: Option<PromotionRequirements>,
}

/// Unlock policy.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct UnlockPolicy {
    #[serde(default)]
    pub require_reason: bool,
    #[serde(default)]
    pub require_impact_report: bool,
    #[serde(default)]
    pub auto_relock_after_merge: bool,
}

/// Patch size/shape limits.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PatchLimits {
    pub max_files_changed: Option<usize>,
    pub max_lines_changed: Option<usize>,
    #[serde(default)]
    pub dependency_changes_require_approval: bool,
    #[serde(default)]
    pub reject_format_only_large_diff: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    pub project: ProjectConfig,
    #[serde(default)]
    pub paths: Vec<PathRule>,
    #[serde(default)]
    pub lock_first_lines: Option<usize>,
    #[serde(default)]
    pub lock_sections: Vec<String>,
    #[serde(default)]
    pub editable_sections: Vec<String>,
    #[serde(default)]
    pub lock_symbols: Vec<String>,
    #[serde(default)]
    pub lock_signatures: Vec<String>,
    #[serde(default)]
    pub lock_exports: bool,
    #[serde(default)]
    pub lock_dependencies: bool,
    #[serde(default)]
    pub detect_test_weakening: bool,
    /// Per-agent authority profiles.
    #[serde(default)]
    pub agents: Vec<AgentProfile>,
    #[serde(default)]
    pub patch_limits: PatchLimits,
    #[serde(default)]
    pub promotion: PromotionConfig,
    #[serde(default)]
    pub unlock_policy: UnlockPolicy,
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;
        let config: Config = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse config YAML: {:?}", path.as_ref()))?;
        Ok(config)
    }

    pub fn find_and_load() -> anyhow::Result<Option<Self>> {
        let current_dir = std::env::current_dir()?;
        let config_path = current_dir.join(".guardpatch.yml");
        if config_path.exists() {
            Ok(Some(Self::load_from_file(config_path)?))
        } else {
            Ok(None)
        }
    }

    pub fn resolve_path_mode<P: AsRef<Path>>(&self, path: P) -> GuardMode {
        let path = path.as_ref();

        for rule in self.paths.iter().rev() {
            if let Ok(pattern) = glob::Pattern::new(&rule.pattern) {
                if pattern.matches_path(path) {
                    return rule.mode.clone();
                }
            }
        }

        self.project.mode.clone()
    }

    /// Resolve the effective mode for a specific agent against a path.
    /// Returns None if the agent has no specific rules for this path.
    pub fn resolve_agent_mode<P: AsRef<Path>>(&self, agent_name: &str, path: P) -> Option<GuardMode> {
        let path = path.as_ref();
        let profile = self.agents.iter().find(|a| a.name == agent_name)?;

        // Deny rules take precedence
        for deny_pattern in &profile.deny {
            if let Ok(pattern) = glob::Pattern::new(deny_pattern) {
                if pattern.matches_path(path) {
                    return Some(GuardMode::Protected);
                }
            }
        }

        // Check allow rules
        for allow_pattern in &profile.allow {
            if let Ok(pattern) = glob::Pattern::new(allow_pattern) {
                if pattern.matches_path(path) {
                    if profile.proposal_only {
                        return Some(GuardMode::ProposalOnly);
                    }
                    return Some(profile.default_mode.clone().unwrap_or(GuardMode::Editable));
                }
            }
        }

        // Agent has no specific rule — fall back to protected (deny by default for unknown agents)
        Some(GuardMode::Protected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_resolution() {
        let config = Config {
            project: ProjectConfig {
                name: "test".to_string(),
                mode: GuardMode::Editable,
                locs_required_for_new_files: false,
            },
            paths: vec![
                PathRule {
                    pattern: "src/**".to_string(),
                    mode: GuardMode::Protected,
                },
                PathRule {
                    pattern: "src/main.rs".to_string(),
                    mode: GuardMode::Editable,
                },
            ],
            ..Default::default()
        };

        assert_eq!(config.resolve_path_mode("README.md"), GuardMode::Editable);
        assert_eq!(config.resolve_path_mode("src/lib.rs"), GuardMode::Protected);
        assert_eq!(config.resolve_path_mode("src/main.rs"), GuardMode::Editable);
    }

    #[test]
    fn test_agent_mode_deny_overrides_allow() {
        let config = Config {
            project: ProjectConfig {
                name: "test".to_string(),
                mode: GuardMode::Editable,
                locs_required_for_new_files: false,
            },
            agents: vec![AgentProfile {
                name: "frontend_agent".to_string(),
                allow: vec!["src/**".to_string()],
                deny: vec!["src/auth/**".to_string()],
                default_mode: None,
                proposal_only: false,
            }],
            ..Default::default()
        };

        assert_eq!(
            config.resolve_agent_mode("frontend_agent", "src/auth/login.ts"),
            Some(GuardMode::Protected)
        );
        assert_eq!(
            config.resolve_agent_mode("frontend_agent", "src/ui/button.ts"),
            Some(GuardMode::Editable)
        );
    }
}
