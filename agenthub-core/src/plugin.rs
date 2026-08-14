//! Plugin system: third-party extension entry points.
//!
//! Plugins are directories under `<skills>/plugins/<name>/` containing a
//! `plugin.yaml` manifest. Each plugin can declare hook commands that run on
//! lifecycle events (`on_install`, `on_uninstall`, `on_session_end`,
//! `on_monitor`, `on_backup`). Hooks are executed as shell commands with
//! stdout/stderr captured and a bounded timeout.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};
use crate::storage::is_safe_id;

/// Built-in lifecycle hook event names.
pub const HOOK_INSTALL: &str = "on_install";
pub const HOOK_UNINSTALL: &str = "on_uninstall";
pub const HOOK_SESSION_END: &str = "on_session_end";
pub const HOOK_MONITOR: &str = "on_monitor";
pub const HOOK_BACKUP: &str = "on_backup";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHook {
    /// Event name, e.g. `on_install` or `on_monitor`.
    pub event: String,
    /// Shell command to run when the hook fires.
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Optional description shown in listings.
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub min_agenthub_version: Option<String>,
    /// Main entry script/command relative to the plugin directory.
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub hooks: Vec<PluginHook>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub manifest: PluginManifest,
    pub enabled: bool,
    pub plugin_dir: PathBuf,
    #[serde(default)]
    pub installed_at: Option<DateTime<Utc>>,
}

/// Outcome of running one hook on one plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRunResult {
    pub plugin: String,
    pub event: String,
    pub ok: bool,
    pub output: String,
    pub duration_ms: u64,
}

pub struct PluginManager {
    skills_dir: PathBuf,
}

impl PluginManager {
    fn validate_name(name: &str) -> Result<()> {
        if !is_safe_id(name) {
            return Err(AgentHubError::SkillError(format!(
                "Invalid plugin name: {name}"
            )));
        }
        Ok(())
    }

    pub fn new(skills_dir: PathBuf) -> Self {
        Self { skills_dir }
    }

    pub fn skills_dir(&self) -> &Path {
        &self.skills_dir
    }

    pub fn plugins_dir(&self) -> PathBuf {
        self.skills_dir.join("plugins")
    }

    fn plugin_dir(&self, name: &str) -> PathBuf {
        self.plugins_dir().join(name)
    }

    /// Load a plugin manifest from its directory.
    pub fn load_plugin(&self, name: &str) -> Result<Plugin> {
        Self::validate_name(name)?;
        let dir = self.plugin_dir(name);
        let path = dir.join("plugin.yaml");
        if !path.exists() {
            return Err(AgentHubError::SkillError(format!(
                "Plugin not found: {}",
                name
            )));
        }
        let content = std::fs::read_to_string(&path).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to read plugin manifest: {}", e))
        })?;
        let manifest: PluginManifest = serde_yaml::from_str(&content).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to parse plugin manifest: {}", e))
        })?;
        let enabled = dir.join(".enabled").exists();
        let installed_at = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok().map(DateTime::from));
        Ok(Plugin {
            manifest,
            enabled,
            plugin_dir: dir,
            installed_at,
        })
    }

    pub fn list_plugins(&self) -> Result<Vec<Plugin>> {
        let dir = self.plugins_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut plugins = Vec::new();
        for entry in std::fs::read_dir(&dir)
            .map_err(|e| AgentHubError::SkillError(format!("Failed to read plugins dir: {}", e)))?
        {
            let entry = entry
                .map_err(|e| AgentHubError::SkillError(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            if path.is_dir() && path.join("plugin.yaml").exists() {
                if let Some(name) = path.file_name() {
                    match self.load_plugin(&name.to_string_lossy()) {
                        Ok(plugin) => plugins.push(plugin),
                        Err(e) => {
                            eprintln!("Warning: failed to load plugin {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
        plugins.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
        Ok(plugins)
    }

    /// Register a plugin by copying its directory into the plugins dir.
    pub fn register_plugin(&self, name: &str, source_dir: &Path) -> Result<Plugin> {
        Self::validate_name(name)?;
        let source_manifest = source_dir.join("plugin.yaml");
        if !source_manifest.exists() {
            return Err(AgentHubError::SkillError(format!(
                "No plugin.yaml found in {}",
                source_dir.display()
            )));
        }
        // Validate the manifest before copying.
        let content = std::fs::read_to_string(&source_manifest).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to read plugin manifest: {}", e))
        })?;
        let manifest: PluginManifest = serde_yaml::from_str(&content).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to parse plugin manifest: {}", e))
        })?;
        if manifest.name != name {
            return Err(AgentHubError::SkillError(format!(
                "Plugin manifest name '{}' does not match requested name '{}'",
                manifest.name, name
            )));
        }

        let dest = self.plugin_dir(name);
        if dest.exists() {
            return Err(AgentHubError::SkillError(format!(
                "Plugin already registered: {}",
                name
            )));
        }
        std::fs::create_dir_all(&dest).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to create plugin dir: {}", e))
        })?;
        crate::skill::SkillManager::copy_dir_recursive_pub(source_dir, &dest)?;
        // Enabled by default.
        std::fs::write(dest.join(".enabled"), "")
            .map_err(|e| AgentHubError::SkillError(format!("Failed to enable plugin: {}", e)))?;
        self.load_plugin(name)
    }

    pub fn unregister_plugin(&self, name: &str) -> Result<bool> {
        Self::validate_name(name)?;
        let dir = self.plugin_dir(name);
        if !dir.exists() {
            return Ok(false);
        }
        std::fs::remove_dir_all(&dir).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to unregister plugin: {}", e))
        })?;
        Ok(true)
    }

    pub fn enable_plugin(&self, name: &str) -> Result<()> {
        Self::validate_name(name)?;
        let dir = self.plugin_dir(name);
        if !dir.exists() {
            return Err(AgentHubError::SkillError(format!(
                "Plugin not found: {}",
                name
            )));
        }
        std::fs::write(dir.join(".enabled"), "")
            .map_err(|e| AgentHubError::SkillError(format!("Failed to enable plugin: {}", e)))?;
        Ok(())
    }

    pub fn disable_plugin(&self, name: &str) -> Result<()> {
        Self::validate_name(name)?;
        let dir = self.plugin_dir(name);
        if !dir.exists() {
            return Err(AgentHubError::SkillError(format!(
                "Plugin not found: {}",
                name
            )));
        }
        let enabled = dir.join(".enabled");
        if enabled.exists() {
            std::fs::remove_file(&enabled).map_err(|e| {
                AgentHubError::SkillError(format!("Failed to disable plugin: {}", e))
            })?;
        }
        Ok(())
    }

    /// Run all hooks registered for `event` across enabled plugins, in name
    /// order. Results are always collected; a failed hook does not abort the
    /// rest.
    pub fn run_hook(&self, event: &str) -> Result<Vec<PluginRunResult>> {
        let mut results = Vec::new();
        for plugin in self.list_plugins()? {
            if !plugin.enabled {
                continue;
            }
            for hook in plugin.manifest.hooks.iter().filter(|h| h.event == event) {
                results.push(self.run_one(&plugin, event, hook)?);
            }
        }
        Ok(results)
    }

    fn run_one(&self, plugin: &Plugin, event: &str, hook: &PluginHook) -> Result<PluginRunResult> {
        let start = std::time::Instant::now();
        let (command, args) = if let Some(entry) = &plugin.manifest.entry {
            let entry_path = plugin.plugin_dir.join(entry);
            if entry_path.exists() {
                (entry_path.to_string_lossy().into_owned(), hook.args.clone())
            } else {
                (hook.command.clone(), hook.args.clone())
            }
        } else {
            (hook.command.clone(), hook.args.clone())
        };

        let output = run_command_with_timeout(&command, &args, 30);

        let ok = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let combined = if stderr.is_empty() {
            stdout
        } else if stdout.is_empty() {
            stderr
        } else {
            format!("{}\n{}", stdout, stderr)
        };

        Ok(PluginRunResult {
            plugin: plugin.manifest.name.clone(),
            event: event.to_string(),
            ok,
            output: combined,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

struct CommandOutcome {
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// Run a shell command with a bounded timeout, capturing output.
fn run_command_with_timeout(command: &str, args: &[String], timeout_secs: u64) -> CommandOutcome {
    let mut cmd = std::process::Command::new(if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "sh"
    });
    let full = if args.is_empty() {
        command.to_string()
    } else {
        format!("{} {}", command, args.join(" "))
    };
    cmd.args(if cfg!(target_os = "windows") {
        vec!["/C", &full]
    } else {
        vec!["-c", &full]
    })
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => {
            return CommandOutcome {
                status: std::process::ExitStatus::default(),
                stdout: Vec::new(),
                stderr: format!("Failed to spawn: {}", command).into_bytes(),
            }
        }
    };

    // Poll for completion with a timeout, then kill the process tree.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = read_pipe(&mut child, true);
                let stderr = read_pipe(&mut child, false);
                return CommandOutcome {
                    status,
                    stdout,
                    stderr,
                };
            }
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return CommandOutcome {
                        status: std::process::ExitStatus::default(),
                        stdout: Vec::new(),
                        stderr: format!("Timed out after {}s", timeout_secs).into_bytes(),
                    };
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(_) => {
                return CommandOutcome {
                    status: std::process::ExitStatus::default(),
                    stdout: Vec::new(),
                    stderr: b"Failed to wait for child".to_vec(),
                }
            }
        }
    }
}

fn read_pipe(child: &mut std::process::Child, stdout: bool) -> Vec<u8> {
    use std::io::Read;
    let mut buf = Vec::new();
    let res = if stdout {
        child.stdout.as_mut().map(|p| p.read_to_end(&mut buf))
    } else {
        child.stderr.as_mut().map(|p| p.read_to_end(&mut buf))
    };
    match res {
        Some(Ok(_)) => buf,
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_plugin(base: &Path, name: &str, hooks_yaml: &str) {
        let dir = base.join("plugins").join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("plugin.yaml"),
            format!(
                "name: {}\nversion: 1.0.0\ndescription: \"Test plugin\"\nauthor: alice\nhooks:\n{}\n",
                name, hooks_yaml
            ),
        )
        .unwrap();
        std::fs::write(dir.join(".enabled"), "").unwrap();
    }

    #[test]
    fn test_register_list_unregister() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("skills");
        let pm = PluginManager::new(base.clone());

        // Source dir
        let src = temp.path().join("src-plugin");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("plugin.yaml"),
            "name: my-plugin\nversion: 0.1.0\ndescription: \"x\"\nhooks: []\n",
        )
        .unwrap();

        let plugin = pm.register_plugin("my-plugin", &src).unwrap();
        assert_eq!(plugin.manifest.name, "my-plugin");
        assert!(plugin.enabled);

        let list = pm.list_plugins().unwrap();
        assert_eq!(list.len(), 1);

        assert!(pm.unregister_plugin("my-plugin").unwrap());
        assert!(pm.list_plugins().unwrap().is_empty());
        assert!(!pm.unregister_plugin("my-plugin").unwrap());
    }

    #[test]
    fn test_register_rejects_name_mismatch() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("src-plugin");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("plugin.yaml"),
            "name: other\nversion: 0.1.0\ndescription: \"x\"\nhooks: []\n",
        )
        .unwrap();
        let pm = PluginManager::new(temp.path().join("skills"));
        assert!(pm.register_plugin("mismatch", &src).is_err());
    }

    #[test]
    fn test_run_hook_executes_commands() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("skills");
        let pm = PluginManager::new(base.clone());
        write_plugin(
            &base,
            "p1",
            "- event: on_install\n  command: \"echo plugin-one\"\n  args: []\n  description: \"greet\"\n",
        );
        write_plugin(
            &base,
            "p2",
            "- event: on_install\n  command: \"echo plugin-two\"\n  args: []\n  description: \"greet\"\n- event: on_monitor\n  command: \"echo monitored\"\n  args: []\n  description: \"m\"\n",
        );

        let results = pm.run_hook(HOOK_INSTALL).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.ok));
        assert!(results
            .iter()
            .any(|r| r.plugin == "p1" && r.output.contains("plugin-one")));
        assert!(results
            .iter()
            .any(|r| r.plugin == "p2" && r.output.contains("plugin-two")));

        // on_monitor only fires for p2
        let monitored = pm.run_hook(HOOK_MONITOR).unwrap();
        assert_eq!(monitored.len(), 1);
        assert_eq!(monitored[0].plugin, "p2");

        // Unknown event -> no results, no error
        assert!(pm.run_hook("on_nothing").unwrap().is_empty());
    }

    #[test]
    fn test_disabled_plugins_skip_hooks() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("skills");
        let pm = PluginManager::new(base.clone());
        write_plugin(
            &base,
            "p1",
            "- event: on_install\n  command: \"echo hi\"\n  args: []\n",
        );

        assert_eq!(pm.run_hook(HOOK_INSTALL).unwrap().len(), 1);
        pm.disable_plugin("p1").unwrap();
        assert!(pm.run_hook(HOOK_INSTALL).unwrap().is_empty());
        pm.enable_plugin("p1").unwrap();
        assert_eq!(pm.run_hook(HOOK_INSTALL).unwrap().len(), 1);
    }

    #[test]
    fn test_failed_command_reported_but_continues() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("skills");
        let pm = PluginManager::new(base.clone());
        write_plugin(
            &base,
            "p1",
            "- event: on_install\n  command: \"exit 3\"\n  args: []\n",
        );
        write_plugin(
            &base,
            "p2",
            "- event: on_install\n  command: \"echo still-ran\"\n  args: []\n",
        );

        let results = pm.run_hook(HOOK_INSTALL).unwrap();
        assert_eq!(results.len(), 2);
        assert!(!results[0].ok);
        assert!(results[1].ok);
    }

    #[test]
    fn test_rejects_unsafe_plugin_names() {
        let temp = TempDir::new().unwrap();
        let pm = PluginManager::new(temp.path().join("skills"));
        assert!(pm.load_plugin("../escape").is_err());
        assert!(pm.unregister_plugin("../escape").is_err());
    }

    #[test]
    fn test_load_plugin_error_paths() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("skills");
        let pm = PluginManager::new(base.clone());

        // Missing manifest.
        assert!(pm.load_plugin("ghost").is_err());

        // Corrupt manifest file.
        let dir = pm.plugin_dir("broken");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.yaml"), "name: \"unterminated").unwrap();
        assert!(pm.load_plugin("broken").is_err());
        // Listing skips corrupt plugins, not fails.
        assert!(pm.list_plugins().unwrap().is_empty());
    }

    #[test]
    fn test_register_plugin_error_paths() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("skills");
        let pm = PluginManager::new(base.clone());

        // Source without plugin.yaml.
        let empty_src = temp.path().join("empty");
        std::fs::create_dir_all(&empty_src).unwrap();
        assert!(pm.register_plugin("p", &empty_src).is_err());

        // Corrupt source manifest.
        let bad_src = temp.path().join("bad");
        std::fs::create_dir_all(&bad_src).unwrap();
        std::fs::write(bad_src.join("plugin.yaml"), "::::").unwrap();
        assert!(pm.register_plugin("p", &bad_src).is_err());
    }

    #[test]
    fn test_enable_disable_missing_plugin_errors() {
        let temp = TempDir::new().unwrap();
        let pm = PluginManager::new(temp.path().join("skills"));
        assert!(pm.enable_plugin("nope").is_err());
        assert!(pm.disable_plugin("nope").is_err());
    }

    #[test]
    fn test_run_hook_with_entry_script_and_failing_spawn() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("skills");
        let pm = PluginManager::new(base.clone());

        // Hook with an entry script that exists -> entry path wins.
        let dir = pm.plugin_dir("with-entry");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("plugin.yaml"),
            "name: with-entry\nversion: 0.1.0\ndescription: \"x\"\nentry: run.sh\nhooks:\n- event: on_install\n  command: \"echo fallback\"\n  args: []\n",
        )
        .unwrap();
        std::fs::write(dir.join("run.sh"), "#!/bin/sh\necho from-entry\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dir.join("run.sh"), std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }
        std::fs::write(dir.join(".enabled"), "").unwrap();

        let results = pm.run_hook(HOOK_INSTALL).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].ok);
        assert!(results[0].output.contains("from-entry"), "{:?}", results[0]);

        // Hook whose entry is missing -> falls back to the plain command.
        write_plugin(
            &base,
            "no-entry",
            "- event: on_install\n  command: \"echo plain-command\"\n  args: []\n",
        );
        let results = pm.run_hook(HOOK_INSTALL).unwrap();
        let no_entry = results
            .iter()
            .find(|r| r.plugin == "no-entry")
            .expect("no-entry result");
        assert!(no_entry.output.contains("plain-command"));

        // Hook with stderr-only output merges stderr into the message.
        write_plugin(
            &base,
            "stderr-only",
            "- event: on_install\n  command: \"echo oops >&2\"\n  args: []\n",
        );
        let results = pm.run_hook(HOOK_INSTALL).unwrap();
        let stderr_only = results
            .iter()
            .find(|r| r.plugin == "stderr-only")
            .expect("stderr-only result");
        assert!(stderr_only.output.contains("oops"));

        // A command that cannot spawn reports a failure, not a panic.
        write_plugin(
            &base,
            "spawn-fail",
            "- event: on_install\n  command: \"definitely-not-a-real-binary-zzz\"\n  args: []\n",
        );
        let results = pm.run_hook(HOOK_INSTALL).unwrap();
        let fail = results
            .iter()
            .find(|r| r.plugin == "spawn-fail")
            .expect("spawn-fail result");
        assert!(!fail.ok);
    }
}
