use crate::agent::{PackageManager, Platform};
use crate::error::{AgentHubError, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

/// Output from a command execution.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub duration_ms: u64,
}

/// Platform-aware command builder for package management operations.
///
/// Centralizes command generation and execution so that `Agent`, `Installer`,
/// `StatusDetector`, and `DiagnosticManager` all share the same logic.
pub struct CommandBuilder {
    pub platform: Platform,
}

impl CommandBuilder {
    pub fn new(platform: Platform) -> Self {
        Self { platform }
    }

    /// Generate the install command string for a given package manager and package.
    pub fn install_command(&self, manager: &PackageManager, package: &str) -> Option<String> {
        match manager {
            PackageManager::Npm => Some(format!("npm install -g {}", package)),
            PackageManager::Pip => Some(format!("pip install {}", package)),
            PackageManager::Winget => Some(format!("winget install {}", package)),
            PackageManager::BrewCask => Some(format!("brew install --cask {}", package)),
            PackageManager::Manual => None,
        }
    }

    /// Generate the uninstall command string for a given package manager and package.
    pub fn uninstall_command(&self, manager: &PackageManager, package: &str) -> Option<String> {
        match manager {
            PackageManager::Npm => Some(format!("npm uninstall -g {}", package)),
            PackageManager::Pip => Some(format!("pip uninstall -y {}", package)),
            PackageManager::Winget => Some(format!("winget uninstall {}", package)),
            PackageManager::BrewCask => Some(format!("brew uninstall --cask {}", package)),
            PackageManager::Manual => None,
        }
    }

    /// Execute a command with an optional timeout.
    ///
    /// On Windows, wraps in `cmd /C`. On Unix, uses direct execution with
    /// proper argument splitting via `shell_words`.
    ///
    /// When `timeout` is `Some`, the process is killed and `timed_out` is set
    /// to `true` in the result if it exceeds the duration.
    pub fn run_command(&self, command: &str, timeout: Option<Duration>) -> Result<CommandOutput> {
        self.run_command_cancellable(command, timeout, None)
    }

    /// Execute a command with an optional timeout and cancellation flag.
    ///
    /// Same semantics as [`run_command`](Self::run_command), but also polls the
    /// given `cancel` flag while the child process runs. When the flag becomes
    /// `true`, the process tree is killed and the returned output carries
    /// `success: false` with `stderr` set to "Operation cancelled by user".
    ///
    /// The child process is ALWAYS killed on timeout/cancel (the wait happens
    /// in a thread while this function retains the PID, so `kill` actually
    /// terminates the process tree).
    pub fn run_command_cancellable(
        &self,
        command: &str,
        timeout: Option<Duration>,
        cancel: Option<Arc<AtomicBool>>,
    ) -> Result<CommandOutput> {
        let start = std::time::Instant::now();
        let (program, args): (String, Vec<String>) = if self.platform == Platform::Windows {
            ("cmd".into(), vec!["/C".into(), command.into()])
        } else {
            let parts = shell_words::split(command).map_err(|e| {
                AgentHubError::InstallerError(format!("Failed to parse command: {}", e))
            })?;
            if parts.is_empty() {
                return Err(AgentHubError::InstallerError("Empty command".to_string()));
            }
            (parts[0].clone(), parts[1..].to_vec())
        };

        let child = std::process::Command::new(&program)
            .args(args.iter().map(|s| s.as_str()))
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                AgentHubError::InstallerError(format!("Failed to spawn process: {}", e))
            })?;
        let pid = child.id();
        let deadline = timeout.map(|dur| start + dur);

        // Capture output in a thread while this function keeps the PID so the
        // process can be killed on timeout/cancel.
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let output = child.wait_with_output();
            let _ = tx.send(output);
        });

        let mut cancelled = false;
        let mut timed_out = false;
        loop {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(Ok(output)) => {
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    return Ok(CommandOutput {
                        success: output.status.success() && !cancelled,
                        exit_code: if cancelled {
                            None
                        } else {
                            output.status.code()
                        },
                        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                        stderr: if cancelled {
                            "Operation cancelled by user".to_string()
                        } else {
                            stderr
                        },
                        timed_out,
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
                Ok(Err(e)) => {
                    return Err(AgentHubError::InstallerError(format!(
                        "Process wait failed: {}",
                        e
                    )))
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if !cancelled && !timed_out {
                        if let Some(flag) = &cancel {
                            if flag.load(Ordering::Relaxed) {
                                cancelled = true;
                                Self::kill_process(pid);
                            }
                        }
                        if cancelled {
                            // keep waiting for the killed child to report
                        } else if let Some(d) = deadline {
                            if std::time::Instant::now() >= d {
                                timed_out = true;
                                Self::kill_process(pid);
                            }
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(AgentHubError::InstallerError(
                        "Process channel disconnected".to_string(),
                    ));
                }
            }
        }
    }

    /// Kill a process tree by PID (taskkill /T on Windows, kill -TERM elsewhere).
    fn kill_process(pid: u32) {
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .status();
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = std::process::Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status();
        }
    }
}

/// Trait for executing commands, enabling mocking in tests.
pub trait CommandRunner: Send + Sync {
    fn run_command(&self, command: &str, timeout: Option<Duration>) -> Result<CommandOutput>;

    /// Execute a command with a cancellation flag. Default implementation
    /// ignores the flag (used by mocks); real runners override it.
    fn run_command_cancellable(
        &self,
        command: &str,
        timeout: Option<Duration>,
        _cancel: Option<Arc<AtomicBool>>,
    ) -> Result<CommandOutput> {
        self.run_command(command, timeout)
    }

    /// Build a platform-aware command string for installation.
    fn install_command(&self, manager: &PackageManager, package: &str) -> Option<String>;

    /// Build a platform-aware command string for uninstallation.
    fn uninstall_command(&self, manager: &PackageManager, package: &str) -> Option<String>;
}

/// Real command runner that executes system commands.
#[derive(Debug, Clone)]
pub struct RealCommandRunner {
    pub platform: Platform,
}

impl RealCommandRunner {
    pub fn new(platform: Platform) -> Self {
        Self { platform }
    }
}

impl CommandRunner for RealCommandRunner {
    fn run_command(&self, command: &str, timeout: Option<Duration>) -> Result<CommandOutput> {
        let builder = CommandBuilder::new(self.platform);
        builder.run_command(command, timeout)
    }

    fn run_command_cancellable(
        &self,
        command: &str,
        timeout: Option<Duration>,
        cancel: Option<Arc<AtomicBool>>,
    ) -> Result<CommandOutput> {
        let builder = CommandBuilder::new(self.platform);
        builder.run_command_cancellable(command, timeout, cancel)
    }

    fn install_command(&self, manager: &PackageManager, package: &str) -> Option<String> {
        let builder = CommandBuilder::new(self.platform);
        builder.install_command(manager, package)
    }

    fn uninstall_command(&self, manager: &PackageManager, package: &str) -> Option<String> {
        let builder = CommandBuilder::new(self.platform);
        builder.uninstall_command(manager, package)
    }
}

/// Mock command runner for testing. Returns canned responses.
#[derive(Debug, Clone)]
pub struct MockCommandRunner {
    pub responses: Vec<MockResponse>,
}

impl MockCommandRunner {
    pub fn new() -> Self {
        Self {
            responses: Vec::new(),
        }
    }

    pub fn with_response(mut self, response: MockResponse) -> Self {
        self.responses.push(response);
        self
    }

    /// Create a mock runner that always succeeds.
    pub fn success() -> Self {
        Self {
            responses: vec![MockResponse::success()],
        }
    }

    /// Create a mock runner that always fails.
    pub fn failure() -> Self {
        Self {
            responses: vec![MockResponse::failure()],
        }
    }

    /// Create a mock runner that always times out.
    pub fn timeout() -> Self {
        Self {
            responses: vec![MockResponse::timeout()],
        }
    }
}

/// A canned response for the mock command runner.
#[derive(Debug, Clone)]
pub struct MockResponse {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

impl MockResponse {
    pub fn new(success: bool, exit_code: Option<i32>, stdout: &str, stderr: &str) -> Self {
        Self {
            success,
            exit_code,
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            timed_out: false,
        }
    }

    pub fn success() -> Self {
        Self::new(true, Some(0), "installed successfully", "")
    }

    pub fn failure() -> Self {
        Self::new(false, Some(1), "", "Installation failed")
    }

    pub fn timeout() -> Self {
        Self {
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: "Process timed out".to_string(),
            timed_out: true,
        }
    }
}

impl CommandRunner for MockCommandRunner {
    fn run_command(&self, _command: &str, _timeout: Option<Duration>) -> Result<CommandOutput> {
        if self.responses.is_empty() {
            return Ok(CommandOutput {
                success: true,
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                timed_out: false,
                duration_ms: 0,
            });
        }

        let resp = &self.responses[0];
        Ok(CommandOutput {
            success: resp.success,
            exit_code: resp.exit_code,
            stdout: resp.stdout.clone(),
            stderr: resp.stderr.clone(),
            timed_out: resp.timed_out,
            duration_ms: 0,
        })
    }

    fn install_command(&self, manager: &PackageManager, package: &str) -> Option<String> {
        match manager {
            PackageManager::Npm => Some(format!("npm install -g {}", package)),
            PackageManager::Pip => Some(format!("pip install {}", package)),
            PackageManager::Winget => Some(format!("winget install {}", package)),
            PackageManager::BrewCask => Some(format!("brew install --cask {}", package)),
            PackageManager::Manual => None,
        }
    }

    fn uninstall_command(&self, manager: &PackageManager, package: &str) -> Option<String> {
        match manager {
            PackageManager::Npm => Some(format!("npm uninstall -g {}", package)),
            PackageManager::Pip => Some(format!("pip uninstall -y {}", package)),
            PackageManager::Winget => Some(format!("winget uninstall {}", package)),
            PackageManager::BrewCask => Some(format!("brew uninstall --cask {}", package)),
            PackageManager::Manual => None,
        }
    }
}

impl Default for MockCommandRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_command_npm() {
        let builder = CommandBuilder::new(Platform::Windows);
        assert_eq!(
            builder.install_command(&PackageManager::Npm, "@test/pkg"),
            Some("npm install -g @test/pkg".to_string())
        );
    }

    #[test]
    fn test_install_command_pip() {
        let builder = CommandBuilder::new(Platform::Windows);
        assert_eq!(
            builder.install_command(&PackageManager::Pip, "test-pkg"),
            Some("pip install test-pkg".to_string())
        );
    }

    #[test]
    fn test_install_command_winget() {
        let builder = CommandBuilder::new(Platform::Windows);
        assert_eq!(
            builder.install_command(&PackageManager::Winget, "Test.Package"),
            Some("winget install Test.Package".to_string())
        );
    }

    #[test]
    fn test_install_command_brew() {
        let builder = CommandBuilder::new(Platform::Windows);
        assert_eq!(
            builder.install_command(&PackageManager::BrewCask, "test-pkg"),
            Some("brew install --cask test-pkg".to_string())
        );
    }

    #[test]
    fn test_install_command_manual() {
        let builder = CommandBuilder::new(Platform::Windows);
        assert_eq!(builder.install_command(&PackageManager::Manual, "x"), None);
    }

    #[test]
    fn test_uninstall_command_npm() {
        let builder = CommandBuilder::new(Platform::Windows);
        assert_eq!(
            builder.uninstall_command(&PackageManager::Npm, "@test/pkg"),
            Some("npm uninstall -g @test/pkg".to_string())
        );
    }

    #[test]
    fn test_uninstall_command_pip() {
        let builder = CommandBuilder::new(Platform::Windows);
        assert_eq!(
            builder.uninstall_command(&PackageManager::Pip, "test-pkg"),
            Some("pip uninstall -y test-pkg".to_string())
        );
    }

    #[test]
    fn test_uninstall_command_winget() {
        let builder = CommandBuilder::new(Platform::Windows);
        assert_eq!(
            builder.uninstall_command(&PackageManager::Winget, "Test.Package"),
            Some("winget uninstall Test.Package".to_string())
        );
    }

    #[test]
    fn test_uninstall_command_brew() {
        let builder = CommandBuilder::new(Platform::Windows);
        assert_eq!(
            builder.uninstall_command(&PackageManager::BrewCask, "test-pkg"),
            Some("brew uninstall --cask test-pkg".to_string())
        );
    }

    #[test]
    fn test_real_runner_command_generation() {
        let real = RealCommandRunner::new(Platform::Windows);
        assert_eq!(
            real.install_command(&PackageManager::Npm, "@test/pkg"),
            Some("npm install -g @test/pkg".to_string())
        );
    }

    #[test]
    fn test_run_command_cancellable_kills_process() {
        #[cfg(target_os = "windows")]
        let builder = CommandBuilder::new(Platform::Windows);
        #[cfg(not(target_os = "windows"))]
        let builder = CommandBuilder::new(Platform::Linux);
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_for_kill = cancel.clone();

        // Long-running command: ping 30 times on Windows, sleep 30 on Unix.
        #[cfg(target_os = "windows")]
        let cmd = "ping -n 30 127.0.0.1";
        #[cfg(not(target_os = "windows"))]
        let cmd = "sleep 30";

        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            cancel_for_kill.store(true, Ordering::Relaxed);
        });

        let result = builder
            .run_command_cancellable(cmd, Some(Duration::from_secs(10)), Some(cancel))
            .unwrap();

        assert!(!result.success);
        assert!(result.stderr.contains("cancelled"));
        assert!(result.duration_ms < 10_000);
    }

    #[test]
    fn test_run_command_timeout_kills_process() {
        #[cfg(target_os = "windows")]
        let builder = CommandBuilder::new(Platform::Windows);
        #[cfg(not(target_os = "windows"))]
        let builder = CommandBuilder::new(Platform::Linux);

        #[cfg(target_os = "windows")]
        let cmd = "ping -n 30 127.0.0.1";
        #[cfg(not(target_os = "windows"))]
        let cmd = "sleep 30";

        let result = builder
            .run_command(cmd, Some(Duration::from_millis(300)))
            .unwrap();

        assert!(!result.success);
        assert!(result.timed_out);
        assert!(result.duration_ms < 10_000);
    }

    #[test]
    fn test_mock_runner_success() {
        let runner = MockCommandRunner::success();
        let result = runner.run_command("npm install -g test", None).unwrap();
        assert!(result.success);
        assert_eq!(result.exit_code, Some(0));
        assert!(!result.timed_out);
    }

    #[test]
    fn test_mock_runner_failure() {
        let runner = MockCommandRunner::failure();
        let result = runner.run_command("npm install -g test", None).unwrap();
        assert!(!result.success);
        assert_eq!(result.exit_code, Some(1));
        assert!(result.stderr.contains("Installation failed"));
    }

    #[test]
    fn test_mock_runner_timeout() {
        let runner = MockCommandRunner::timeout();
        let result = runner.run_command("npm install -g test", None).unwrap();
        assert!(!result.success);
        assert!(result.timed_out);
    }

    #[test]
    fn test_mock_runner_custom_response() {
        let runner = MockCommandRunner::new().with_response(MockResponse::new(
            true,
            Some(0),
            "custom output",
            "",
        ));
        let result = runner.run_command("npm list", None).unwrap();
        assert!(result.success);
        assert_eq!(result.stdout, "custom output");
    }

    #[test]
    fn test_mock_runner_empty_responses() {
        let runner = MockCommandRunner::new();
        let result = runner.run_command("echo hello", None).unwrap();
        assert!(result.success);
    }
}
