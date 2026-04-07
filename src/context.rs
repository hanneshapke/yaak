use std::env;
use std::fs;
use std::path::Path;

/// Known project markers and their types.
const PROJECT_MARKERS: &[(&str, &str)] = &[
    ("Cargo.toml", "Rust (Cargo)"),
    ("package.json", "Node.js (npm/yarn)"),
    ("go.mod", "Go"),
    ("pyproject.toml", "Python (pyproject)"),
    ("setup.py", "Python (setuptools)"),
    ("requirements.txt", "Python (pip)"),
    ("Pipfile", "Python (pipenv)"),
    ("Gemfile", "Ruby (Bundler)"),
    ("pom.xml", "Java (Maven)"),
    ("build.gradle", "Java/Kotlin (Gradle)"),
    ("CMakeLists.txt", "C/C++ (CMake)"),
    ("Makefile", "Make"),
    ("docker-compose.yml", "Docker Compose"),
    ("docker-compose.yaml", "Docker Compose"),
    ("Dockerfile", "Docker"),
    ("flake.nix", "Nix"),
    ("deno.json", "Deno"),
    ("composer.json", "PHP (Composer)"),
    ("mix.exs", "Elixir (Mix)"),
    (".terraform", "Terraform"),
];

/// Detect project types present in the given directory.
fn detect_project_types(dir: &Path) -> Vec<&'static str> {
    let mut types = Vec::new();
    for &(marker, project_type) in PROJECT_MARKERS {
        if dir.join(marker).exists() {
            types.push(project_type);
        }
    }
    types
}

/// List files/directories in the working directory (non-recursive, top-level only).
/// Returns at most `max_entries` items to avoid overwhelming the prompt.
fn list_directory(dir: &Path, max_entries: usize) -> Vec<String> {
    let mut entries = Vec::new();
    if let Ok(read_dir) = fs::read_dir(dir) {
        for entry in read_dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip hidden files to avoid noise
            if name.starts_with('.') {
                continue;
            }
            let suffix = if entry.path().is_dir() { "/" } else { "" };
            entries.push(format!("{}{}", name, suffix));
        }
    }
    entries.sort();
    entries.truncate(max_entries);
    entries
}

/// Check if the current directory is a git repo and get brief status.
fn git_info(dir: &Path) -> Option<String> {
    if !dir.join(".git").exists() {
        return None;
    }
    let output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(dir)
        .output()
        .ok()?;
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        return Some("Git repo (detached HEAD)".to_string());
    }
    Some(format!("Git repo, branch: {}", branch))
}

/// Build a context string describing the current working directory.
/// Returns a block of text to append to the user message.
pub fn gather_context() -> String {
    let cwd = env::current_dir().unwrap_or_default();
    let mut parts = Vec::new();

    parts.push(format!("Working directory: {}", cwd.display()));

    // Project types
    let project_types = detect_project_types(&cwd);
    if !project_types.is_empty() {
        parts.push(format!("Project type: {}", project_types.join(", ")));
    }

    // Git info
    if let Some(git) = git_info(&cwd) {
        parts.push(git);
    }

    // Directory listing
    let files = list_directory(&cwd, 30);
    if !files.is_empty() {
        parts.push(format!("Files in current directory: {}", files.join(", ")));
    }

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn detect_cargo_project() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        let types = detect_project_types(dir.path());
        assert_eq!(types, vec!["Rust (Cargo)"]);
    }

    #[test]
    fn detect_multiple_project_types() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        fs::write(dir.path().join("Dockerfile"), "").unwrap();
        let types = detect_project_types(dir.path());
        assert!(types.contains(&"Rust (Cargo)"));
        assert!(types.contains(&"Docker"));
    }

    #[test]
    fn detect_no_project_type() {
        let dir = TempDir::new().unwrap();
        let types = detect_project_types(dir.path());
        assert!(types.is_empty());
    }

    #[test]
    fn list_directory_skips_hidden() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".hidden"), "").unwrap();
        fs::write(dir.path().join("visible.txt"), "").unwrap();
        let entries = list_directory(dir.path(), 50);
        assert!(entries.contains(&"visible.txt".to_string()));
        assert!(!entries.iter().any(|e| e.starts_with('.')));
    }

    #[test]
    fn list_directory_marks_dirs() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        fs::write(dir.path().join("file.txt"), "").unwrap();
        let entries = list_directory(dir.path(), 50);
        assert!(entries.contains(&"subdir/".to_string()));
        assert!(entries.contains(&"file.txt".to_string()));
    }

    #[test]
    fn list_directory_respects_limit() {
        let dir = TempDir::new().unwrap();
        for i in 0..10 {
            fs::write(dir.path().join(format!("file{}.txt", i)), "").unwrap();
        }
        let entries = list_directory(dir.path(), 3);
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn gather_context_includes_cwd() {
        let ctx = gather_context();
        assert!(ctx.contains("Working directory:"));
    }
}
