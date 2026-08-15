//! Skill marketplace: local registry with search, ratings and install stats.
//!
//! The marketplace lives under `<skills>/marketplace/`:
//!
//! ```text
//! marketplace/
//! ├── index.json            # generated index (searchable)
//! ├── packages/<name>/SKILL.md   # distributable skill packages
//! └── ratings/<name>.json   # per-skill rating history
//! ```
//!
//! Package discovery/install works fully offline — the packages directory can
//! be synced via git/shared storage. Every install increments the skill's
//! install counter; ratings are aggregated into an average.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};
use crate::remote::{self, RemoteSyncReport};
use crate::skill::SkillManager;
use crate::storage::{atomic_write, is_safe_id, is_safe_relative_path};

/// A skill available in the marketplace (as indexed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceSkill {
    pub name: String,
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub category: Option<String>,
    /// Whether the package is present locally (index may persist entries
    /// whose package directory was removed).
    pub available: bool,
    /// Number of times this skill has been installed from the marketplace.
    #[serde(default)]
    pub installs: u64,
    /// Average rating (1-5), None when nobody rated yet.
    #[serde(default)]
    pub rating_avg: Option<f64>,
    #[serde(default)]
    pub rating_count: u64,
}

/// One rating record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRating {
    pub rating: u8,
    #[serde(default)]
    pub rater: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Aggregated marketplace statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceStats {
    pub package_count: usize,
    pub total_installs: u64,
    pub rated_count: usize,
    pub top_rated: Vec<MarketplaceSkill>,
}

/// A portable, UTF-8 skill package used by the remote registry protocol.
/// Binary files are intentionally not accepted by this first protocol
/// version; executable plugins require a separate signed package format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteSkillPackage {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub category: Option<String>,
    /// Relative path -> UTF-8 contents. SKILL.md is mandatory.
    pub files: BTreeMap<String, String>,
}

const MAX_REMOTE_PACKAGE_FILES: usize = 1024;
const MAX_REMOTE_FILE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct MarketplaceIndex {
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub skills: Vec<MarketplaceSkill>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RatingsFile {
    #[serde(default)]
    ratings: Vec<SkillRating>,
}

pub struct MarketplaceManager {
    skills_dir: PathBuf,
}

impl MarketplaceManager {
    fn validate_name(name: &str) -> Result<()> {
        if !is_safe_id(name) {
            return Err(AgentHubError::SkillError(format!(
                "Invalid marketplace skill name: {name}"
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

    pub fn marketplace_dir(&self) -> PathBuf {
        self.skills_dir.join("marketplace")
    }

    fn packages_dir(&self) -> PathBuf {
        self.marketplace_dir().join("packages")
    }

    fn ratings_dir(&self) -> PathBuf {
        self.marketplace_dir().join("ratings")
    }

    fn index_path(&self) -> PathBuf {
        self.marketplace_dir().join("index.json")
    }

    // ---- index ------------------------------------------------------------

    fn load_index(&self) -> Result<MarketplaceIndex> {
        let path = self.index_path();
        if !path.exists() {
            return Ok(MarketplaceIndex::default());
        }
        let content = std::fs::read_to_string(&path).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to read marketplace index: {}", e))
        })?;
        serde_json::from_str(&content).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to parse marketplace index: {}", e))
        })
    }

    fn save_index(&self, index: &MarketplaceIndex) -> Result<()> {
        std::fs::create_dir_all(self.marketplace_dir()).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to create marketplace dir: {}", e))
        })?;
        let content = serde_json::to_string_pretty(index).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to serialize marketplace index: {}", e))
        })?;
        std::fs::write(self.index_path(), content).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to write marketplace index: {}", e))
        })?;
        Ok(())
    }

    /// Re-scan the packages directory and regenerate the index, preserving
    /// install counts and ratings for known skills.
    pub fn refresh(&self) -> Result<MarketplaceStats> {
        let mut index = self.load_index().unwrap_or_default();
        let mut seen: Vec<MarketplaceSkill> = Vec::new();
        let packages_dir = self.packages_dir();
        if packages_dir.exists() {
            for entry in std::fs::read_dir(&packages_dir).map_err(|e| {
                AgentHubError::SkillError(format!("Failed to read packages dir: {}", e))
            })? {
                let entry = entry.map_err(|e| {
                    AgentHubError::SkillError(format!("Failed to read entry: {}", e))
                })?;
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let manifest_path = path.join("SKILL.md");
                if !manifest_path.exists() {
                    continue;
                }
                let content = std::fs::read_to_string(&manifest_path).map_err(|e| {
                    AgentHubError::SkillError(format!("Failed to read package manifest: {}", e))
                })?;
                let Ok(manifest) = SkillManager::parse_manifest_pub(&content) else {
                    continue;
                };
                let name = manifest.name.clone();
                if !is_safe_id(&name) {
                    continue;
                }
                // Carry over stats from the previous index.
                let previous = index.skills.iter().find(|s| s.name == name);
                let (installs, rating_avg, rating_count) = match previous {
                    Some(p) => (p.installs, p.rating_avg, p.rating_count),
                    None => {
                        let (avg, count) = self.load_ratings(&name)?;
                        (0, avg, count)
                    }
                };
                seen.push(MarketplaceSkill {
                    name,
                    description: manifest.description.clone(),
                    version: manifest.version.clone(),
                    author: manifest.author.clone(),
                    tags: manifest.tags.clone(),
                    category: manifest.category.clone(),
                    available: true,
                    installs,
                    rating_avg,
                    rating_count,
                });
            }
        }

        // Keep previously-indexed skills that are no longer present, but mark
        // them unavailable so installs can be rejected.
        for previous in &index.skills {
            if !seen.iter().any(|s| s.name == previous.name) {
                let mut entry = previous.clone();
                entry.available = false;
                seen.push(entry);
            }
        }

        seen.sort_by(|a, b| a.name.cmp(&b.name));
        index.skills = seen;
        index.updated_at = Some(Utc::now());
        self.save_index(&index)?;
        self.stats()
    }

    /// Add a skill package (a directory containing SKILL.md) to the marketplace.
    pub fn add_package(&self, name: &str, source_dir: &Path) -> Result<MarketplaceSkill> {
        Self::validate_name(name)?;
        let manifest_path = source_dir.join("SKILL.md");
        if !manifest_path.exists() {
            return Err(AgentHubError::SkillError(format!(
                "No SKILL.md found in {}",
                source_dir.display()
            )));
        }
        let content = std::fs::read_to_string(&manifest_path).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to read package manifest: {}", e))
        })?;
        let manifest = SkillManager::parse_manifest_pub(&content)?;
        if manifest.name != name {
            return Err(AgentHubError::SkillError(format!(
                "Package manifest name '{}' does not match requested name '{}'",
                manifest.name, name
            )));
        }

        let dest = self.packages_dir().join(name);
        if dest.exists() {
            return Err(AgentHubError::SkillError(format!(
                "Marketplace package already exists: {}",
                name
            )));
        }
        std::fs::create_dir_all(&dest).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to create package dir: {}", e))
        })?;
        SkillManager::copy_dir_recursive_pub(source_dir, &dest)?;

        let skill = MarketplaceSkill {
            name: manifest.name.clone(),
            description: manifest.description,
            version: manifest.version,
            author: manifest.author,
            tags: manifest.tags,
            category: manifest.category,
            available: true,
            installs: 0,
            rating_avg: None,
            rating_count: 0,
        };
        self.refresh()?;
        Ok(skill)
    }

    /// Search the index by name/description/tags (case-insensitive substring).
    pub fn search(&self, query: &str) -> Result<Vec<MarketplaceSkill>> {
        let index = self.refresh()?;
        let q = query.to_lowercase();
        Ok(index
            .top_rated
            .iter()
            .filter(|s| {
                s.available
                    && (s.name.to_lowercase().contains(&q)
                        || s.description.to_lowercase().contains(&q)
                        || s.tags.iter().any(|t| t.to_lowercase().contains(&q)))
            })
            .cloned()
            .collect())
    }

    /// Look up one skill in the index.
    pub fn info(&self, name: &str) -> Result<MarketplaceSkill> {
        Self::validate_name(name)?;
        let index = self.load_index()?;
        index
            .skills
            .into_iter()
            .find(|s| s.name == name)
            .ok_or_else(|| {
                AgentHubError::SkillError(format!(
                    "Marketplace skill not found: {} (run `market refresh` first)",
                    name
                ))
            })
    }

    /// Install a marketplace package into the installed skills directory,
    /// bumping the install counter.
    pub fn install(&self, skill_manager: &SkillManager, name: &str) -> Result<()> {
        Self::validate_name(name)?;
        let skill = self.info(name)?;
        if !skill.available {
            return Err(AgentHubError::SkillError(format!(
                "Package '{}' is not available locally (directory missing)",
                name
            )));
        }
        let source = self.packages_dir().join(name);
        skill_manager.install_skill(name, &source)?;

        // Bump install stats.
        let mut index = self.load_index()?;
        if let Some(entry) = index.skills.iter_mut().find(|s| s.name == name) {
            entry.installs += 1;
        }
        self.save_index(&index)?;
        Ok(())
    }

    fn parse_remote(value: Value) -> Result<Vec<RemoteSkillPackage>> {
        let packages = if value.is_array() {
            value
        } else {
            value.get("packages").cloned().ok_or_else(|| {
                AgentHubError::SkillError(
                    "Remote skill registry must be an array or an object with a 'packages' array"
                        .to_string(),
                )
            })
        };
        serde_json::from_value(packages).map_err(|e| {
            AgentHubError::SkillError(format!("Invalid remote skill registry: {e}"))
        })
    }

    fn validate_remote_package(package: &RemoteSkillPackage) -> Result<()> {
        Self::validate_name(&package.name)?;
        if package.files.is_empty() || package.files.len() > MAX_REMOTE_PACKAGE_FILES {
            return Err(AgentHubError::SkillError(format!(
                "Remote package '{}' must contain 1-{} files",
                package.name, MAX_REMOTE_PACKAGE_FILES
            )));
        }
        let manifest = package.files.get("SKILL.md").ok_or_else(|| {
            AgentHubError::SkillError(format!(
                "Remote package '{}' is missing SKILL.md",
                package.name
            ))
        })?;
        if manifest.len() > MAX_REMOTE_FILE_BYTES {
            return Err(AgentHubError::SkillError("Remote SKILL.md is too large".to_string()));
        }
        let parsed = SkillManager::parse_manifest_pub(manifest)?;
        if parsed.name != package.name {
            return Err(AgentHubError::SkillError(format!(
                "Remote manifest name '{}' does not match package name '{}'",
                parsed.name, package.name
            )));
        }
        if !package.version.is_empty() && parsed.version != package.version {
            return Err(AgentHubError::SkillError(format!(
                "Remote package version '{}' does not match manifest version '{}'",
                package.version, parsed.version
            )));
        }
        for (path, content) in &package.files {
            let hidden_component = Path::new(path).components().any(|component| {
                component
                    .as_os_str()
                    .to_string_lossy()
                    .starts_with('.')
            });
            if path.contains('\\') || !is_safe_relative_path(Path::new(path)) || hidden_component {
                return Err(AgentHubError::SkillError(format!(
                    "Unsafe remote package path: {path}"
                )));
            }
            if content.len() > MAX_REMOTE_FILE_BYTES {
                return Err(AgentHubError::SkillError(format!(
                    "Remote package file '{}' is too large",
                    path
                )));
            }
        }
        Ok(())
    }

    fn write_remote_package(&self, package: &RemoteSkillPackage) -> Result<()> {
        Self::validate_remote_package(package)?;
        let destination = self.packages_dir().join(&package.name);
        if destination.exists() {
            std::fs::remove_dir_all(&destination).map_err(|e| {
                AgentHubError::SkillError(format!("Failed to replace remote package: {e}"))
            })?;
        }
        for (relative, content) in &package.files {
            let path = destination.join(relative);
            atomic_write(&path, content).map_err(|e| {
                AgentHubError::SkillError(format!("Failed to write remote package file: {e}"))
            })?;
        }
        Ok(())
    }

    /// Pull UTF-8 skill packages from a remote registry.
    ///
    /// The endpoint may return an array or
    /// `{ "version": 1, "packages": [...] }`. Packages are validated before
    /// any file is written, and path traversal, hidden paths and oversized
    /// files are rejected.
    pub fn pull_remote(
        &self,
        url: &str,
        token: Option<&str>,
        force: bool,
    ) -> Result<RemoteSyncReport> {
        let value = remote::get_json(url, token)
            .map_err(|e| AgentHubError::SkillError(format!("Remote skill pull failed: {e}")))?;
        let packages = Self::parse_remote(value)?;
        // Validate the complete batch before changing local storage.
        for package in &packages {
            Self::validate_remote_package(package)?;
        }
        let old_index = self.load_index().unwrap_or_default();
        let mut report = RemoteSyncReport::default();
        for package in packages {
            let previous = old_index.skills.iter().find(|s| s.name == package.name);
            if previous.is_some_and(|entry| entry.version == package.version) && !force {
                report.skipped += 1;
                continue;
            }
            self.write_remote_package(&package)?;
            if previous.is_some() {
                report.updated += 1;
            } else {
                report.added += 1;
            }
        }
        self.refresh()?;
        Ok(report)
    }

    fn collect_files(root: &Path, current: &Path, files: &mut BTreeMap<String, String>) -> Result<()> {
        for entry in std::fs::read_dir(current).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to read package directory: {e}"))
        })? {
            let path = entry
                .map_err(|e| AgentHubError::SkillError(format!("Failed to read package entry: {e}")))?
                .path();
            if path.is_dir() {
                Self::collect_files(root, &path, files)?;
                continue;
            }
            if !path.is_file() {
                continue;
            }
            let relative = path.strip_prefix(root).map_err(|e| {
                AgentHubError::SkillError(format!("Failed to calculate package path: {e}"))
            })?;
            let relative_string = relative.to_string_lossy().replace('\\', "/");
            let hidden_component = Path::new(&relative_string).components().any(|component| {
                component
                    .as_os_str()
                    .to_string_lossy()
                    .starts_with('.')
            });
            if !is_safe_relative_path(Path::new(&relative_string)) || hidden_component {
                continue;
            }
            let content = std::fs::read_to_string(&path).map_err(|e| {
                AgentHubError::SkillError(format!("Remote protocol only supports UTF-8 files: {e}"))
            })?;
            if content.len() > MAX_REMOTE_FILE_BYTES {
                return Err(AgentHubError::SkillError(format!(
                    "Package file '{}' is too large",
                    relative_string
                )));
            }
            files.insert(relative_string, content);
            if files.len() > MAX_REMOTE_PACKAGE_FILES {
                return Err(AgentHubError::SkillError("Package contains too many files".to_string()));
            }
        }
        Ok(())
    }

    /// Push all local packages to a remote registry.
    pub fn push_remote(&self, url: &str, token: Option<&str>) -> Result<RemoteSyncReport> {
        let mut packages = Vec::new();
        let root = self.packages_dir();
        if root.exists() {
            for entry in std::fs::read_dir(&root).map_err(|e| {
                AgentHubError::SkillError(format!("Failed to read packages dir: {e}"))
            })? {
                let path = entry
                    .map_err(|e| AgentHubError::SkillError(format!("Failed to read package entry: {e}")))?
                    .path();
                if !path.is_dir() {
                    continue;
                }
                let name = path
                    .file_name()
                    .map(|value| value.to_string_lossy().to_string())
                    .unwrap_or_default();
                Self::validate_name(&name)?;
                let mut files = BTreeMap::new();
                Self::collect_files(&path, &path, &mut files)?;
                let manifest = SkillManager::parse_manifest_pub(files.get("SKILL.md").ok_or_else(|| {
                    AgentHubError::SkillError(format!("Package '{}' is missing SKILL.md", name))
                })?)?;
                packages.push(RemoteSkillPackage {
                    name,
                    version: manifest.version,
                    description: manifest.description,
                    author: manifest.author,
                    tags: manifest.tags,
                    category: manifest.category,
                    files,
                });
            }
        }
        let payload = serde_json::json!({ "version": 1, "packages": packages });
        remote::post_json(url, token, &payload)
            .map_err(|e| AgentHubError::SkillError(format!("Remote skill push failed: {e}")))?;
        Ok(RemoteSyncReport {
            uploaded: packages.len(),
            ..RemoteSyncReport::default()
        })
    }

    // ---- ratings ----------------------------------------------------------

    fn ratings_path(&self, name: &str) -> PathBuf {
        self.ratings_dir().join(format!("{}.json", name))
    }

    fn load_ratings(&self, name: &str) -> Result<(Option<f64>, u64)> {
        Self::validate_name(name)?;
        let path = self.ratings_path(name);
        if !path.exists() {
            return Ok((None, 0));
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AgentHubError::SkillError(format!("Failed to read ratings: {}", e)))?;
        let file: RatingsFile = serde_json::from_str(&content)
            .map_err(|e| AgentHubError::SkillError(format!("Failed to parse ratings: {}", e)))?;
        let count = file.ratings.len() as u64;
        if count == 0 {
            return Ok((None, 0));
        }
        let sum: f64 = file.ratings.iter().map(|r| r.rating as f64).sum();
        Ok((Some(sum / count as f64), count))
    }

    /// Rate a marketplace skill (1-5). Updates the index average.
    pub fn rate(&self, name: &str, rating: u8, rater: Option<&str>) -> Result<SkillRating> {
        Self::validate_name(name)?;
        if !(1..=5).contains(&rating) {
            return Err(AgentHubError::SkillError(format!(
                "Rating must be 1-5, got {}",
                rating
            )));
        }
        self.info(name)?;
        let path = self.ratings_path(name);
        std::fs::create_dir_all(self.ratings_dir()).map_err(|e| {
            AgentHubError::SkillError(format!("Failed to create ratings dir: {}", e))
        })?;
        let mut file: RatingsFile = if path.exists() {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| AgentHubError::SkillError(format!("Failed to read ratings: {}", e)))?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            RatingsFile::default()
        };
        let entry = SkillRating {
            rating,
            rater: rater.map(|r| r.to_string()),
            created_at: Utc::now(),
        };
        file.ratings.push(entry.clone());
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&file).map_err(|e| {
                AgentHubError::SkillError(format!("Failed to serialize ratings: {}", e))
            })?,
        )
        .map_err(|e| AgentHubError::SkillError(format!("Failed to write ratings: {}", e)))?;

        let (avg, count) = self.load_ratings(name)?;
        let mut index = self.load_index()?;
        if let Some(entry) = index.skills.iter_mut().find(|s| s.name == name) {
            entry.rating_avg = avg;
            entry.rating_count = count;
        }
        self.save_index(&index)?;
        Ok(entry)
    }

    /// Aggregate statistics across the marketplace.
    pub fn stats(&self) -> Result<MarketplaceStats> {
        let index = self.load_index()?;
        let available: Vec<MarketplaceSkill> = index
            .skills
            .iter()
            .filter(|s| s.available)
            .cloned()
            .collect();
        let total_installs = available.iter().map(|s| s.installs).sum();
        let rated_count = available.iter().filter(|s| s.rating_count > 0).count();
        let mut top_rated = available.clone();
        top_rated.sort_by(|a, b| {
            let ar = a.rating_avg.unwrap_or(0.0);
            let br = b.rating_avg.unwrap_or(0.0);
            br.partial_cmp(&ar)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.installs.cmp(&a.installs))
        });
        Ok(MarketplaceStats {
            package_count: available.len(),
            total_installs,
            rated_count,
            top_rated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use tempfile::TempDir;

    fn registry_server(body: &'static str, method: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 2048];
            let size = stream.read(&mut request).unwrap();
            assert!(String::from_utf8_lossy(&request[..size]).starts_with(method));
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        format!("http://{address}/skills")
    }

    fn write_package(dir: &Path, name: &str, tags: &[&str]) {
        let pkg = dir.join("marketplace").join("packages").join(name);
        std::fs::create_dir_all(&pkg).unwrap();
        let tags_yaml: Vec<String> = tags.iter().map(|t| format!("  - \"{}\"", t)).collect();
        std::fs::write(
            pkg.join("SKILL.md"),
            format!(
                "---\nname: {}\ndescription: \"{} skill package\"\nversion: 1.0.0\nauthor: alice\ntags:\n{}\ncategory: testing\n---\n\n# {}\n",
                name,
                name,
                tags_yaml.join("\n"),
                name
            ),
        )
        .unwrap();
    }

    #[test]
    fn test_refresh_and_search() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("skills");
        write_package(&base, "rust-dev", &["rust", "cargo"]);
        write_package(&base, "elixir-dev", &["elixir", "mix"]);

        let mm = MarketplaceManager::new(base);
        let stats = mm.refresh().unwrap();
        assert_eq!(stats.package_count, 2);

        let results = mm.search("rust").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "rust-dev");

        // Tag search
        let results = mm.search("elixir").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "elixir-dev");

        // No match
        assert!(mm.search("python").unwrap().is_empty());
    }

    #[test]
    fn test_install_increments_stats() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("skills");
        write_package(&base, "rust-dev", &["rust"]);
        let mm = MarketplaceManager::new(base.clone());
        mm.refresh().unwrap();

        let sm = SkillManager::new(base.clone());
        mm.install(&sm, "rust-dev").unwrap();
        // Second install after uninstall bumps the counter again
        sm.uninstall_skill("rust-dev").unwrap();
        mm.install(&sm, "rust-dev").unwrap();

        let info = mm.info("rust-dev").unwrap();
        assert_eq!(info.installs, 2);
        assert!(sm.get_skill("rust-dev").is_ok());

        let stats = mm.stats().unwrap();
        assert_eq!(stats.total_installs, 2);

        // Installing a non-existent package fails
        assert!(mm.install(&sm, "nope").is_err());
    }

    #[test]
    fn test_rating_flow() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("skills");
        write_package(&base, "rust-dev", &["rust"]);
        let mm = MarketplaceManager::new(base.clone());
        mm.refresh().unwrap();

        assert_eq!(mm.rate("rust-dev", 5, Some("alice")).unwrap().rating, 5);
        assert_eq!(mm.rate("rust-dev", 3, Some("bob")).unwrap().rating, 3);

        let info = mm.info("rust-dev").unwrap();
        assert_eq!(info.rating_count, 2);
        assert!((info.rating_avg.unwrap() - 4.0).abs() < 1e-9);

        // Out of range rejected
        assert!(mm.rate("rust-dev", 6, None).is_err());
        assert!(mm.rate("rust-dev", 0, None).is_err());

        // Top rated ranking
        let stats = mm.stats().unwrap();
        assert_eq!(stats.rated_count, 1);
        assert_eq!(stats.top_rated[0].name, "rust-dev");
    }

    #[test]
    fn test_add_package_from_source() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("skills");
        let mm = MarketplaceManager::new(base.clone());

        // A source skill dir
        let src = temp.path().join("src-skill");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("SKILL.md"),
            "---\nname: new-skill\ndescription: \"A fresh skill\"\nversion: 0.5.0\ntags: []\n---\n\n# New\n",
        )
        .unwrap();

        mm.add_package("new-skill", &src).unwrap();
        let info = mm.info("new-skill").unwrap();
        assert_eq!(info.version, "0.5.0");

        // Name mismatch rejected
        let src2 = temp.path().join("src2");
        std::fs::create_dir_all(&src2).unwrap();
        std::fs::write(
            src2.join("SKILL.md"),
            "---\nname: other\ndescription: \"x\"\nversion: 1.0.0\ntags: []\n---\n\n# x\n",
        )
        .unwrap();
        assert!(mm.add_package("mismatch", &src2).is_err());
    }

    #[test]
    fn test_stats_preserved_across_refresh() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("skills");
        write_package(&base, "rust-dev", &["rust"]);
        let mm = MarketplaceManager::new(base.clone());
        mm.refresh().unwrap();
        mm.rate("rust-dev", 4, None).unwrap();

        // Re-refresh must preserve installs/ratings
        let sm = SkillManager::new(base.clone());
        mm.install(&sm, "rust-dev").unwrap();
        let stats = mm.refresh().unwrap();
        assert_eq!(stats.package_count, 1);
        let info = mm.info("rust-dev").unwrap();
        assert_eq!(info.installs, 1);
        assert_eq!(info.rating_count, 1);
    }

    #[test]
    fn test_rejects_unsafe_marketplace_names() {
        let temp = TempDir::new().unwrap();
        let mm = MarketplaceManager::new(temp.path().join("skills"));
        assert!(mm.info("../escape").is_err());
        assert!(mm.rate("../escape", 5, None).is_err());
        assert!(!temp.path().join("escape.json").exists());
    }

    #[test]
    fn test_search_corrupt_index_errors() {
        let temp = TempDir::new().unwrap();
        let mm = MarketplaceManager::new(temp.path().join("skills"));
        std::fs::create_dir_all(mm.marketplace_dir()).unwrap();
        std::fs::write(mm.marketplace_dir().join("index.json"), "{ bad json !!").unwrap();
        // `info` reads the index directly, so a corrupt index must error.
        assert!(mm.info("anything").is_err());
    }

    #[test]
    fn test_remote_package_validation_rejects_traversal_and_mismatch() {
        let valid_manifest = "---\nname: remote-skill\nversion: 1.0.0\n---\n# Skill\n";
        let mut files = BTreeMap::new();
        files.insert("SKILL.md".to_string(), valid_manifest.to_string());
        files.insert("scripts/run.sh".to_string(), "echo ok".to_string());
        let valid = RemoteSkillPackage {
            name: "remote-skill".to_string(),
            version: "1.0.0".to_string(),
            description: String::new(),
            author: None,
            tags: Vec::new(),
            category: None,
            files,
        };
        assert!(MarketplaceManager::validate_remote_package(&valid).is_ok());

        let mut unsafe_package = valid.clone();
        unsafe_package.files.insert("../escape".to_string(), "bad".to_string());
        assert!(MarketplaceManager::validate_remote_package(&unsafe_package).is_err());
        unsafe_package.files.remove("../escape");
        unsafe_package.files.insert(r"..\\escape".to_string(), "bad".to_string());
        assert!(MarketplaceManager::validate_remote_package(&unsafe_package).is_err());

        let mut mismatch = valid;
        mismatch.name = "another-name".to_string();
        assert!(MarketplaceManager::validate_remote_package(&mismatch).is_err());
    }

    #[test]
    fn test_remote_pull_and_push() {
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("skills");
        let manager = MarketplaceManager::new(base.clone());
        let body = r#"{"version":1,"packages":[{"name":"remote-skill","version":"1.0.0","description":"Remote","files":{"SKILL.md":"---\nname: remote-skill\nversion: 1.0.0\n---\n# Remote\n"}}]}"#;
        let report = manager
            .pull_remote(&registry_server(body, "GET"), Some("token"), false)
            .unwrap();
        assert_eq!(report.added, 1);
        assert_eq!(manager.info("remote-skill").unwrap().version, "1.0.0");

        let pushed = manager
            .push_remote(&registry_server("{}", "POST"), None)
            .unwrap();
        assert_eq!(pushed.uploaded, 1);
    }
}
