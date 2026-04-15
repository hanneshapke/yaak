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

/// Privilege escalation prefixes to detect.
const ESCALATION_COMMANDS: &[&str] = &["sudo", "doas", "pkexec"];

/// sudo flags that consume the next token as a value argument.
const SUDO_FLAGS_WITH_VALUE: &[&str] = &[
    "-u",
    "--user",
    "-g",
    "--group",
    "-C",
    "--close-from",
    "-D",
    "--chdir",
    "-R",
    "--chroot",
    "-T",
    "--command-timeout",
    "-h",
    "--host",
    "-p",
    "--prompt",
    "-r",
    "--role",
    "-t",
    "--type",
];

/// Strips privilege-escalation prefixes (sudo, doas, pkexec) and their flags
/// to reveal the underlying command. Handles patterns like:
///   sudo -E rm -rf /
///   sudo -u root env VAR=val dd ...
///   doas rm -rf /
fn strip_escalation_prefix(s: &str) -> &str {
    let s = s.trim();
    for &prefix in ESCALATION_COMMANDS {
        if let Some(rest) = s.strip_prefix(prefix) {
            // Must be followed by whitespace (not e.g. "sudoku")
            if !rest.starts_with(|c: char| c.is_whitespace()) {
                continue;
            }
            let mut remaining = rest.trim_start();
            // Skip flags and their value arguments
            while remaining.starts_with('-') {
                let token_end = remaining
                    .find(char::is_whitespace)
                    .unwrap_or(remaining.len());
                let flag = &remaining[..token_end];
                // Advance past this flag token
                if token_end >= remaining.len() {
                    return remaining; // flag with no following command
                }
                remaining = remaining[token_end..].trim_start();
                // If this flag takes a value, skip the next token too
                if SUDO_FLAGS_WITH_VALUE.contains(&flag) && !remaining.is_empty() {
                    if let Some(pos) = remaining.find(char::is_whitespace) {
                        remaining = remaining[pos..].trim_start();
                    } else {
                        return ""; // value was the last token, no command left
                    }
                }
            }
            // Skip `env` wrapper if present
            if let Some(after_env) = remaining.strip_prefix("env") {
                if after_env.starts_with(|c: char| c.is_whitespace()) {
                    remaining = after_env.trim_start();
                    // Skip VAR=val pairs
                    while let Some(eq_pos) = remaining.find('=') {
                        // Make sure the '=' is part of a VAR=val token (no space before '=')
                        let before_eq = &remaining[..eq_pos];
                        if before_eq.contains(char::is_whitespace) {
                            break;
                        }
                        if let Some(pos) = remaining.find(char::is_whitespace) {
                            remaining = remaining[pos..].trim_start();
                        } else {
                            break;
                        }
                    }
                }
            }
            return remaining;
        }
    }
    s
}

/// Returns `Some(keyword)` if the command contains a destructive operation.
pub fn detect_destructive(command: &str) -> Option<&'static str> {
    // Normalise: check each segment separated by && ; | ||
    for segment in command.split(['&', ';', '|']) {
        let trimmed = segment.trim().trim_start_matches('!');
        let trimmed = trimmed.trim();
        // Strip any privilege escalation prefix to get the underlying command
        let effective = strip_escalation_prefix(trimmed);
        for &pattern in DESTRUCTIVE_COMMANDS {
            if effective.starts_with(pattern) || effective == pattern.trim() {
                return Some(pattern.trim());
            }
        }
    }
    None
}

/// Returns `true` if the command contains a privilege-escalation prefix
/// (sudo, doas, pkexec) in any segment.
pub fn requires_privilege_escalation(command: &str) -> bool {
    for segment in command.split(['&', ';', '|']) {
        let trimmed = segment.trim().trim_start_matches('!').trim();
        for &prefix in ESCALATION_COMMANDS {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                if rest.is_empty() || rest.starts_with(char::is_whitespace) {
                    return true;
                }
            }
        }
    }
    false
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
    fn blocks_sudo_with_flags() {
        assert!(detect_destructive("sudo -E rm -rf /").is_some());
        assert!(detect_destructive("sudo -u root rm -rf /").is_some());
        assert!(detect_destructive("sudo --preserve-env rm file").is_some());
    }

    #[test]
    fn blocks_sudo_env_wrapper() {
        assert!(detect_destructive("sudo env PATH=/usr/bin dd if=/dev/zero of=/dev/sda").is_some());
        assert!(detect_destructive("sudo -E env VAR=val rm -rf /tmp").is_some());
    }

    #[test]
    fn blocks_doas_and_pkexec() {
        assert!(detect_destructive("doas rm -rf /").is_some());
        assert!(detect_destructive("pkexec rm -rf /").is_some());
        assert!(detect_destructive("doas dd if=/dev/zero of=/dev/sda").is_some());
    }

    #[test]
    fn blocks_rm_in_chain() {
        assert!(detect_destructive("echo hello && rm -rf /tmp").is_some());
        assert!(detect_destructive("ls; rm foo").is_some());
        assert!(detect_destructive("ls; sudo rm foo").is_some());
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
        // "sudoku" should not match "sudo"
        assert!(detect_destructive("sudoku").is_none());
    }

    #[test]
    fn detects_privilege_escalation() {
        assert!(requires_privilege_escalation("sudo apt install foo"));
        assert!(requires_privilege_escalation(
            "doas systemctl restart nginx"
        ));
        assert!(requires_privilege_escalation("pkexec visudo"));
        assert!(requires_privilege_escalation(
            "echo hello && sudo service restart"
        ));
    }

    #[test]
    fn no_privilege_escalation_for_normal_commands() {
        assert!(!requires_privilege_escalation("ls -la"));
        assert!(!requires_privilege_escalation("apt list --installed"));
        assert!(!requires_privilege_escalation("sudoku")); // not sudo
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
