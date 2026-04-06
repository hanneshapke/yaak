pub fn extract_command(raw: &str) -> String {
    let trimmed = raw.trim();

    // Try to extract from ```bash ... ``` or ``` ... ``` fenced blocks
    if let Some(start) = trimmed.find("```") {
        let after_fence = &trimmed[start + 3..];
        // Skip optional language tag on the same line
        let code_start = after_fence.find('\n').map(|i| i + 1).unwrap_or(0);
        let code_body = &after_fence[code_start..];
        if let Some(end) = code_body.find("```") {
            return code_body[..end].trim().to_string();
        }
    }

    // Otherwise strip any leading "$ " and return as-is
    if let Some(stripped) = trimmed.strip_prefix("$ ") {
        return stripped.to_string();
    }

    trimmed.to_string()
}

/// Destructive command prefixes/patterns that should be blocked.
const DESTRUCTIVE_COMMANDS: &[&str] = &[
    "rm ",
    "rm\t",
    "rmdir ",
    "mkfs",
    "dd ",
    "dd\t",
    "> /dev/",
    "chmod -R 000",
    "chmod 000",
    ":(){ :|:& };:",
    "shred ",
    "wipefs ",
];

/// Returns `Some(keyword)` if the command contains a destructive operation.
pub fn detect_destructive(command: &str) -> Option<&'static str> {
    // Normalise: check each segment separated by && ; | ||
    for segment in command.split(['&', ';', '|']) {
        let trimmed = segment.trim().trim_start_matches('!');
        let trimmed = trimmed.trim();
        for &pattern in DESTRUCTIVE_COMMANDS {
            if trimmed.starts_with(pattern) || trimmed == pattern.trim() {
                return Some(pattern.trim());
            }
        }
        // Also catch "sudo rm ..." etc.
        if let Some(after_sudo) = trimmed.strip_prefix("sudo ") {
            let after_sudo = after_sudo.trim();
            for &pattern in DESTRUCTIVE_COMMANDS {
                if after_sudo.starts_with(pattern) || after_sudo == pattern.trim() {
                    return Some(pattern.trim());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_rm() {
        assert!(detect_destructive("rm -rf /").is_some());
        assert!(detect_destructive("rm file.txt").is_some());
    }

    #[test]
    fn blocks_sudo_rm() {
        assert!(detect_destructive("sudo rm -rf /").is_some());
    }

    #[test]
    fn blocks_rm_in_chain() {
        assert!(detect_destructive("echo hello && rm -rf /tmp").is_some());
        assert!(detect_destructive("ls; rm foo").is_some());
    }

    #[test]
    fn blocks_other_destructive() {
        assert!(detect_destructive("dd if=/dev/zero of=/dev/sda").is_some());
        assert!(detect_destructive("mkfs.ext4 /dev/sda1").is_some());
        assert!(detect_destructive("shred /dev/sda").is_some());
    }

    #[test]
    fn allows_safe_commands() {
        assert!(detect_destructive("ls -la").is_none());
        assert!(detect_destructive("cat file.txt").is_none());
        assert!(detect_destructive("grep -r pattern .").is_none());
        assert!(detect_destructive("echo remove").is_none());
    }

    #[test]
    fn extract_plain_command() {
        assert_eq!(extract_command("ls -la"), "ls -la");
    }

    #[test]
    fn extract_strips_dollar_prefix() {
        assert_eq!(
            extract_command("$ find . -name '*.rs'"),
            "find . -name '*.rs'"
        );
    }

    #[test]
    fn extract_from_fenced_code_block() {
        let raw = "```bash\nfind . -name '*.rs'\n```";
        assert_eq!(extract_command(raw), "find . -name '*.rs'");
    }

    #[test]
    fn extract_from_unlabeled_fence() {
        let raw = "```\necho hello\n```";
        assert_eq!(extract_command(raw), "echo hello");
    }

    #[test]
    fn extract_trims_whitespace() {
        assert_eq!(extract_command("  ls -la  "), "ls -la");
    }
}
