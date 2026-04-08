use colored::Colorize;
use rust_i18n::t;

/// Render a richly formatted explanation of a command in the terminal.
pub fn render_explanation(command: &str, raw: &str) {
    let separator = "─".repeat(60);

    // Header: the command being explained
    eprintln!("{}", separator.dimmed());
    eprintln!("  {} {}", "⟩".cyan().bold(), command.green().bold());
    eprintln!("{}", separator.dimmed());

    let mut in_breakdown = false;
    let mut in_examples = false;

    for line in raw.lines() {
        let trimmed = line.trim();

        if let Some(summary) = trimmed.strip_prefix("SUMMARY:") {
            eprintln!();
            eprintln!("  {} {}", "💡".bold(), summary.trim().bold());
            eprintln!();
            in_breakdown = false;
            in_examples = false;
        } else if trimmed == "BREAKDOWN:" {
            eprintln!("  {}", t!("explain_breakdown").cyan().bold().underline());
            in_breakdown = true;
            in_examples = false;
        } else if trimmed == "EXAMPLES:" {
            eprintln!();
            eprintln!("  {}", t!("explain_examples").cyan().bold().underline());
            in_breakdown = false;
            in_examples = true;
        } else if let Some(caution) = trimmed.strip_prefix("CAUTION:") {
            eprintln!();
            eprintln!(
                "  {} {}",
                format!("⚠  {}", t!("explain_caution")).yellow().bold(),
                caution.trim().yellow()
            );
            in_breakdown = false;
            in_examples = false;
        } else if let Some(part_body) = trimmed.strip_prefix("PART:") {
            if in_breakdown {
                if let Some((token, explanation)) = part_body.split_once('|') {
                    eprintln!(
                        "    {} {:<20} {} {}",
                        "│".dimmed(),
                        token.trim().green().bold(),
                        "→".dimmed(),
                        explanation.trim()
                    );
                } else {
                    eprintln!("    {} {}", "│".dimmed(), part_body.trim());
                }
            }
        } else if let Some(ex_body) = trimmed.strip_prefix("EXAMPLE:") {
            if in_examples {
                if let Some((cmd, desc)) = ex_body.split_once('|') {
                    eprintln!(
                        "    {} {}  {}",
                        "$".dimmed(),
                        cmd.trim().green(),
                        desc.trim().dimmed()
                    );
                } else {
                    eprintln!("    {} {}", "$".dimmed(), ex_body.trim().green());
                }
            }
        } else if !trimmed.is_empty() && !in_breakdown && !in_examples {
            // Fallback for lines that don't match structured format
            eprintln!("  {}", trimmed);
        }
    }

    eprintln!();
    eprintln!("{}", separator.dimmed());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_explanation_does_not_panic() {
        let raw = "\
SUMMARY: Lists all files in the current directory including hidden ones.

BREAKDOWN:
PART: ls | List directory contents
PART: -la | Show long format and include hidden files
PART: /tmp | Target directory

EXAMPLES:
EXAMPLE: ls -lah | Include human-readable file sizes
EXAMPLE: ls -lt | Sort by modification time

CAUTION: None for this command.";

        render_explanation("ls -la /tmp", raw);
    }

    #[test]
    fn render_explanation_handles_minimal_input() {
        render_explanation("echo hello", "SUMMARY: Prints hello to stdout.");
    }
}
