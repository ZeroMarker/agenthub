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
use std::path::{Path, PathBuf};

use crate::error::{AgentHubError, Result};
use crate::skill::SkillManager;

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

    // ---- ratings ----------------------------------------------------------

    fn ratings_path(&self, name: &str) -> PathBuf {
        self.ratings_dir().join(format!("{}.json", name))
    }

    fn load_ratings(&self, name: &str) -> Result<(Option<f64>, u64)> {
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
    use tempfile::TempDir;

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
}
