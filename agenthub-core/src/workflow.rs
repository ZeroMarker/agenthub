use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};
use crate::skill::SkillManager;
use crate::storage::is_safe_id;

/// One step in a workflow: run a skill (optionally with arguments).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Skill name to invoke.
    pub skill: String,
    /// Optional arguments passed to the skill (name -> value).
    #[serde(default)]
    pub args: HashMap<String, String>,
    /// When true, a failing/absent skill does not fail the whole workflow.
    #[serde(default)]
    pub optional: bool,
}

/// A named, ordered sequence of skill steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Outcome of running one step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepResult {
    pub skill: String,
    pub ok: bool,
    /// Reason for failure / summary.
    pub message: String,
    /// True when the step was skipped because it is optional.
    #[serde(default)]
    pub skipped: bool,
}

/// Outcome of running a whole workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunReport {
    pub workflow_id: String,
    pub executed_at: DateTime<Utc>,
    pub ok: bool,
    pub steps: Vec<WorkflowStepResult>,
}

impl WorkflowRunReport {
    pub fn succeeded(&self) -> usize {
        self.steps.iter().filter(|s| s.ok).count()
    }
}

/// Manages skill workflows (CRUD + dry-run execution).
///
/// A workflow is a lightweight orchestration layer on top of skills: it lets
/// users define "do A, then B, then C" pipelines that are validated against the
/// installed skill set (existence, enabled state, dependency commands and
/// version compatibility) before execution. Execution is a dry-run plan for
/// now — the actual skill invocation is delegated to the host environment.
pub struct WorkflowManager {
    workflows_dir: PathBuf,
}

impl WorkflowManager {
    fn validate_id(id: &str) -> Result<()> {
        if !is_safe_id(id) {
            return Err(AgentHubError::SkillError(format!(
                "Invalid workflow id: {id}"
            )));
        }
        Ok(())
    }

    pub fn new(skills_dir: PathBuf) -> Self {
        Self {
            workflows_dir: skills_dir.join("workflows"),
        }
    }

    pub fn workflows_dir(&self) -> &Path {
        &self.workflows_dir
    }

    fn workflow_path(&self, id: &str) -> PathBuf {
        self.workflows_dir.join(format!("{}.yaml", id))
    }

    pub fn list_workflows(&self) -> Result<Vec<Workflow>> {
        let mut workflows = Vec::new();
        if !self.workflows_dir.exists() {
            return Ok(workflows);
        }
        for entry in std::fs::read_dir(&self.workflows_dir).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to read workflows dir: {}", e))
        })? {
            let entry = entry.map_err(|e| {
                AgentHubError::SkillError(format!("Failed to read workflow entry: {}", e))
            })?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
            {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(wf) = serde_yaml::from_str::<Workflow>(&content) {
                        workflows.push(wf);
                    }
                }
            }
        }
        workflows.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(workflows)
    }

    pub fn get_workflow(&self, id: &str) -> Result<Workflow> {
        Self::validate_id(id)?;
        let path = self.workflow_path(id);
        if !path.exists() {
            return Err(AgentHubError::SkillError(format!(
                "Workflow not found: {}",
                id
            )));
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AgentHubError::SkillError(format!("Failed to read workflow: {}", e)))?;
        serde_yaml::from_str(&content)
            .map_err(|e| AgentHubError::SkillError(format!("Failed to parse workflow: {}", e)))
    }

    pub fn create_workflow(
        &self,
        id: &str,
        name: &str,
        description: &str,
        steps: Vec<WorkflowStep>,
    ) -> Result<Workflow> {
        Self::validate_id(id)?;
        if steps.is_empty() {
            return Err(AgentHubError::SkillError(
                "Workflow must contain at least one step".to_string(),
            ));
        }
        let now = Utc::now();
        let workflow = Workflow {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            steps,
            created_at: now,
            updated_at: now,
        };
        self.save_workflow(&workflow)?;
        Ok(workflow)
    }

    pub fn save_workflow(&self, workflow: &Workflow) -> Result<()> {
        Self::validate_id(&workflow.id)?;
        std::fs::create_dir_all(&self.workflows_dir).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to create workflows dir: {}", e))
        })?;
        let content = serde_yaml::to_string(workflow).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to serialize workflow: {}", e))
        })?;
        std::fs::write(self.workflow_path(&workflow.id), content)
            .map_err(|e| AgentHubError::SkillError(format!("Failed to write workflow: {}", e)))?;
        Ok(())
    }

    pub fn delete_workflow(&self, id: &str) -> Result<bool> {
        Self::validate_id(id)?;
        let path = self.workflow_path(id);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                AgentHubError::SkillError(format!("Failed to delete workflow: {}", e))
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Validate a workflow against the installed skills and produce a run plan.
    /// Optional steps that are missing/failing do not fail the workflow.
    pub fn run_workflow(
        &self,
        skill_manager: &SkillManager,
        id: &str,
    ) -> Result<WorkflowRunReport> {
        let workflow = self.get_workflow(id)?;
        let mut steps = Vec::with_capacity(workflow.steps.len());

        for step in &workflow.steps {
            let result = self.evaluate_step(skill_manager, step);
            let skipped = !result.ok && step.optional;
            steps.push(WorkflowStepResult {
                ok: result.ok || skipped,
                skipped,
                ..result
            });
        }

        let ok = steps.iter().all(|s| s.ok);
        Ok(WorkflowRunReport {
            workflow_id: workflow.id.clone(),
            executed_at: Utc::now(),
            ok,
            steps,
        })
    }

    fn evaluate_step(
        &self,
        skill_manager: &SkillManager,
        step: &WorkflowStep,
    ) -> WorkflowStepResult {
        let skill = match skill_manager.get_skill(&step.skill) {
            Ok(skill) => skill,
            Err(_) => {
                return WorkflowStepResult {
                    skill: step.skill.clone(),
                    ok: false,
                    message: "skill not installed".to_string(),
                    skipped: false,
                }
            }
        };

        if !skill.enabled {
            return WorkflowStepResult {
                skill: step.skill.clone(),
                ok: false,
                message: "skill is disabled".to_string(),
                skipped: false,
            };
        }

        let compat = skill_manager.check_compatibility(&step.skill);
        if let Ok(compat) = compat {
            if !compat.compatible {
                return WorkflowStepResult {
                    skill: step.skill.clone(),
                    ok: false,
                    message: format!(
                        "requires AgentHub >= {}",
                        compat.requires_agenthub.unwrap_or_default()
                    ),
                    skipped: false,
                };
            }
        }

        let deps = skill_manager.check_dependencies(&step.skill);
        match deps {
            Ok(deps) => {
                let missing: Vec<String> = deps
                    .iter()
                    .filter(|(_, present)| !present)
                    .map(|(cmd, _)| cmd.clone())
                    .collect();
                if !missing.is_empty() {
                    return WorkflowStepResult {
                        skill: step.skill.clone(),
                        ok: false,
                        message: format!("missing dependency commands: {}", missing.join(", ")),
                        skipped: false,
                    };
                }
            }
            Err(e) => {
                return WorkflowStepResult {
                    skill: step.skill.clone(),
                    ok: false,
                    message: format!("dependency check failed: {}", e),
                    skipped: false,
                }
            }
        }

        let arg_note = if step.args.is_empty() {
            String::new()
        } else {
            format!(" with {} arg(s)", step.args.len())
        };
        WorkflowStepResult {
            skill: step.skill.clone(),
            ok: true,
            message: format!("ready to run{}", arg_note),
            skipped: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_managers() -> (WorkflowManager, SkillManager, tempfile::TempDir) {
        let temp = tempfile::tempdir().unwrap();
        let skill_manager = SkillManager::new(temp.path().join("skills"));
        let workflow_manager = WorkflowManager::new(temp.path().join("skills"));
        (workflow_manager, skill_manager, temp)
    }

    #[test]
    fn test_create_list_get_delete() {
        let (manager, _skills, _temp) = create_managers();
        let mut args = HashMap::new();
        args.insert("language".to_string(), "rust".to_string());

        manager
            .create_workflow(
                "release",
                "Release pipeline",
                "Run checks then build",
                vec![
                    WorkflowStep {
                        skill: "rust-dev".to_string(),
                        args: HashMap::new(),
                        optional: false,
                    },
                    WorkflowStep {
                        skill: "release-build".to_string(),
                        args,
                        optional: true,
                    },
                ],
            )
            .unwrap();

        let workflows = manager.list_workflows().unwrap();
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].id, "release");
        assert_eq!(workflows[0].steps.len(), 2);
        assert_eq!(workflows[0].steps[1].args["language"], "rust");

        let loaded = manager.get_workflow("release").unwrap();
        assert_eq!(loaded.name, "Release pipeline");

        assert!(manager.delete_workflow("release").unwrap());
        assert!(!manager.delete_workflow("release").unwrap());
        assert!(manager.get_workflow("release").is_err());
    }

    #[test]
    fn test_create_validation() {
        let (manager, _skills, _temp) = create_managers();
        assert!(manager
            .create_workflow("", "Empty", "desc", vec![])
            .is_err());
        assert!(manager
            .create_workflow("no-steps", "No steps", "desc", vec![])
            .is_err());
    }

    #[test]
    fn test_run_workflow_missing_skills() {
        let (manager, skills, _temp) = create_managers();
        // Install a real skill
        let skill_dir = tempfile::tempdir().unwrap();
        std::fs::write(
            skill_dir.path().join("SKILL.md"),
            "---\nname: rust-dev\ndescription: Rust workflow\nversion: 1.0.0\n---\n\n# Rust\n",
        )
        .unwrap();
        skills.install_skill("rust-dev", skill_dir.path()).unwrap();

        manager
            .create_workflow(
                "ci",
                "CI",
                "Checks",
                vec![
                    WorkflowStep {
                        skill: "rust-dev".to_string(),
                        args: HashMap::new(),
                        optional: false,
                    },
                    WorkflowStep {
                        skill: "not-installed".to_string(),
                        args: HashMap::new(),
                        optional: false,
                    },
                ],
            )
            .unwrap();

        let report = manager.run_workflow(&skills, "ci").unwrap();
        assert!(!report.ok);
        assert_eq!(report.steps.len(), 2);
        assert!(report.steps[0].ok);
        assert!(!report.steps[1].ok);
        assert_eq!(report.steps[1].message, "skill not installed");
        assert_eq!(report.succeeded(), 1);
    }

    #[test]
    fn test_run_workflow_optional_step_skipped() {
        let (manager, skills, _temp) = create_managers();
        manager
            .create_workflow(
                "opt",
                "Optional",
                "Optional missing step",
                vec![WorkflowStep {
                    skill: "missing".to_string(),
                    args: HashMap::new(),
                    optional: true,
                }],
            )
            .unwrap();

        let report = manager.run_workflow(&skills, "opt").unwrap();
        assert!(report.ok);
        assert_eq!(report.steps.len(), 1);
        assert!(report.steps[0].skipped);
    }

    #[test]
    fn test_workflow_persists_across_reload() {
        let temp = tempfile::tempdir().unwrap();
        {
            let manager = WorkflowManager::new(temp.path().join("skills"));
            manager
                .create_workflow(
                    "wf",
                    "Wf",
                    "desc",
                    vec![WorkflowStep {
                        skill: "a".to_string(),
                        args: HashMap::new(),
                        optional: false,
                    }],
                )
                .unwrap();
        }
        let manager = WorkflowManager::new(temp.path().join("skills"));
        assert_eq!(manager.list_workflows().unwrap().len(), 1);
    }
}
