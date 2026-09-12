//! Background update checking and self-update.
//!
//! yaak checks GitHub for a newer release at most once a day. The check runs in
//! a detached background process so it never delays the command the user asked
//! for — the result lands in a small state file and is surfaced on a *later*
//! invocation. When a newer version is known, yaak prints a one-line notice and
//! (on a TTY) offers to update, once per new version. Which update path it
//! takes depends on how yaak was installed: it replaces its own binary only for
//! installs no package manager owns.

use chrono::{DateTime, Duration, Local};
use colored::Colorize;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::Config;

/// GitHub API endpoint for the newest published (non-draft, non-prerelease) release.
const RELEASES_API: &str = "https://api.github.com/repos/hanneshapke/yaak/releases/latest";
/// Human-facing releases page, used when we cannot update automatically.
const RELEASES_PAGE: &str = "https://github.com/hanneshapke/yaak/releases/latest";
/// The one-line installer, used to update installs that no package manager owns.
const INSTALL_SCRIPT: &str = "https://getyaak.ai/install.sh";
/// How long a successful check result stays fresh.
const CHECK_INTERVAL_HOURS: i64 = 24;
/// Floor between attempts, so a check that fails (or is cut short when yaak
/// exits quickly) retries soon without hammering GitHub.
const RETRY_INTERVAL_HOURS: i64 = 1;
/// Network timeout for the version check — it runs in the background, so keep it short.
const CHECK_TIMEOUT_SECS: u64 = 5;
/// Hidden flag yaak passes to itself to run a check in the background.
pub const BACKGROUND_CHECK_FLAG: &str = "--internal-update-check";

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

// ──────────────────────────────────────────────
// Persisted state
// ──────────────────────────────────────────────

/// What we remember between runs about update checking.
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct UpdateState {
    /// When a check was last *started* (recorded up front so a failing check
    /// doesn't re-fire on every invocation).
    pub last_check: Option<DateTime<Local>>,
    /// When a check last came back with an answer.
    pub last_success: Option<DateTime<Local>>,
    /// Newest release tag seen, without the leading `v`.
    pub latest_version: Option<String>,
    /// Version the user was already prompted about and declined.
    pub declined_version: Option<String>,
}

/// Returns the path to the update state file (~/.local/share/yaak/update.json).
pub fn state_path() -> PathBuf {
    if let Some(data_dir) = dirs::data_local_dir() {
        data_dir.join("yaak").join("update.json")
    } else if let Some(home) = dirs::home_dir() {
        home.join(".local")
            .join("share")
            .join("yaak")
            .join("update.json")
    } else {
        PathBuf::from("update.json")
    }
}

fn load_state() -> UpdateState {
    match std::fs::read_to_string(state_path()) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => UpdateState::default(),
    }
}

fn save_state(state: &UpdateState) {
    let path = state_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(&path, json);
    }
}

/// Re-read the state from disk, apply `f`, and write it back. Re-reading keeps
/// the foreground process from clobbering what the background check wrote.
fn update_state(f: impl FnOnce(&mut UpdateState)) {
    let mut state = load_state();
    f(&mut state);
    save_state(&state);
}

/// True when a fresh check is due: once a day after a successful one, and no
/// more often than hourly after an attempt that produced nothing.
pub fn is_due(
    last_check: Option<DateTime<Local>>,
    last_success: Option<DateTime<Local>>,
    now: DateTime<Local>,
) -> bool {
    if let Some(attempted) = last_check {
        // Also covers a clock that jumped backwards: the duration is negative,
        // so no check fires until the clock catches up.
        if now.signed_duration_since(attempted) < Duration::hours(RETRY_INTERVAL_HOURS) {
            return false;
        }
    }
    match last_success {
        Some(succeeded) => {
            now.signed_duration_since(succeeded) >= Duration::hours(CHECK_INTERVAL_HOURS)
        }
        None => true,
    }
}

// ──────────────────────────────────────────────
// Version comparison
// ──────────────────────────────────────────────

/// Parse `1.2.3`, `v1.2.3` or `v1.2` into a comparable triple. Returns `None`
/// for pre-releases (`v1.2.3-rc1`) and anything unparseable — we never nudge a
/// user towards a pre-release.
pub fn parse_version(raw: &str) -> Option<(u64, u64, u64)> {
    let trimmed = raw.trim().trim_start_matches(['v', 'V']);
    if trimmed.is_empty() || trimmed.contains(['-', '+']) {
        return None;
    }
    let mut parts = trimmed.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// True when `latest` is a strictly newer release than `current`.
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

/// Pull the release tag out of a GitHub `releases/latest` response.
pub fn parse_latest_tag(body: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Release {
        tag_name: String,
    }
    let release: Release = serde_json::from_str(body).ok()?;
    let tag = release
        .tag_name
        .trim()
        .trim_start_matches(['v', 'V'])
        .to_string();
    if tag.is_empty() {
        None
    } else {
        Some(tag)
    }
}

// ──────────────────────────────────────────────
// Fetching
// ──────────────────────────────────────────────

/// Ask GitHub for the newest release tag. Returns `None` on any failure —
/// update checking is best-effort and must never interrupt normal use.
pub fn fetch_latest_version() -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(CHECK_TIMEOUT_SECS))
        .build()
        .ok()?;
    let response = client
        .get(RELEASES_API)
        .header("User-Agent", format!("yaak/{}", CURRENT_VERSION))
        .header("Accept", "application/vnd.github+json")
        .send()
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    parse_latest_tag(&response.text().ok()?)
}

// ──────────────────────────────────────────────
// Install method detection
// ──────────────────────────────────────────────

/// How yaak got onto this machine — decides whether it may replace itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMethod {
    /// Installed by the one-line install script (or a manual binary drop).
    Script,
    Homebrew,
    Cargo,
    Nix,
    Scoop,
    /// A system package under /usr — pacman (AUR) or dpkg owns it.
    SystemPackage,
}

/// Guess the install method from the path of the running binary.
pub fn detect_install_method(exe: &Path) -> InstallMethod {
    let path = exe.to_string_lossy().replace('\\', "/").to_lowercase();
    if path.contains("/nix/store/") {
        InstallMethod::Nix
    } else if path.contains("/cellar/")
        || path.contains("/homebrew/")
        || path.contains("/linuxbrew/")
    {
        InstallMethod::Homebrew
    } else if path.contains("/.cargo/bin/") {
        InstallMethod::Cargo
    } else if path.contains("/scoop/apps/") {
        InstallMethod::Scoop
    } else if path.starts_with("/usr/bin/") || path.starts_with("/usr/local/sbin/") {
        InstallMethod::SystemPackage
    } else {
        InstallMethod::Script
    }
}

/// What yaak should do to bring itself up to date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStrategy {
    /// yaak can replace its own binary by re-running the install script.
    SelfInstall,
    /// A package manager owns the binary; `label` names it, `command` upgrades it.
    Managed { label: String, command: String },
    /// No automatic path — point the user at the releases page.
    Manual,
}

/// Map an install method to an update strategy. `has_tool` reports whether a
/// helper binary is on PATH (injected so this stays testable).
pub fn strategy_for(method: InstallMethod, has_tool: impl Fn(&str) -> bool) -> UpdateStrategy {
    match method {
        InstallMethod::Homebrew => UpdateStrategy::Managed {
            label: "Homebrew".into(),
            command: "brew upgrade hanneshapke/yaak/yaak".into(),
        },
        InstallMethod::Cargo => UpdateStrategy::Managed {
            label: "cargo".into(),
            command: "cargo install yaak --force".into(),
        },
        InstallMethod::Nix => UpdateStrategy::Managed {
            label: "Nix".into(),
            command: "nix profile upgrade yaak".into(),
        },
        InstallMethod::Scoop => UpdateStrategy::Managed {
            label: "Scoop".into(),
            command: "scoop update yaak".into(),
        },
        InstallMethod::SystemPackage => {
            for helper in ["paru", "yay"] {
                if has_tool(helper) {
                    return UpdateStrategy::Managed {
                        label: "the AUR".into(),
                        command: format!("{} -S yaak-cli-bin", helper),
                    };
                }
            }
            UpdateStrategy::Manual
        }
        // The install script only supports macOS and Linux.
        InstallMethod::Script if cfg!(windows) => UpdateStrategy::Manual,
        InstallMethod::Script => UpdateStrategy::SelfInstall,
    }
}

fn tool_on_path(name: &str) -> bool {
    let which = if cfg!(windows) { "where" } else { "which" };
    Command::new(which)
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// The strategy for the currently running binary.
pub fn current_strategy() -> UpdateStrategy {
    match std::env::current_exe() {
        Ok(exe) => strategy_for(detect_install_method(&exe), tool_on_path),
        Err(_) => UpdateStrategy::Manual,
    }
}

// ──────────────────────────────────────────────
// Daily check + notification
// ──────────────────────────────────────────────

fn is_interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stderr().is_terminal()
}

/// Values that mean "no" when they show up in an environment variable, so
/// `YAAK_NO_UPDATE_CHECK=0` reads the way a user would expect.
pub fn reads_as_off(value: &str) -> bool {
    matches!(
        value.trim().to_lowercase().as_str(),
        "" | "0" | "false" | "no" | "off"
    )
}

/// True when an environment variable is set to anything but an "off" value.
fn env_flag_set(name: &str) -> bool {
    std::env::var_os(name)
        .map(|value| !reads_as_off(&value.to_string_lossy()))
        .unwrap_or(false)
}

/// Update checks are off in CI, when piped, and when the user opts out via
/// `YAAK_NO_UPDATE_CHECK` or `check_updates = false` in the config file.
fn checks_enabled(config: &Config) -> bool {
    if !config.check_updates {
        return false;
    }
    if env_flag_set("YAAK_NO_UPDATE_CHECK") || env_flag_set("CI") {
        return false;
    }
    std::io::stderr().is_terminal()
}

/// Kick off a version check in a detached child process (`yaak
/// --internal-update-check`). A child rather than a thread, because yaak often
/// exits within milliseconds — a thread would be killed mid-request, while the
/// child outlives us and still records its result. Either way the user never
/// waits on the network: the notice appears on a later run.
fn spawn_background_check() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let _ = Command::new(exe)
        .arg(BACKGROUND_CHECK_FLAG)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// The body of the detached child: fetch the latest release and record it.
pub fn run_background_check() {
    if let Some(latest) = fetch_latest_version() {
        update_state(|state| {
            state.last_success = Some(Local::now());
            state.latest_version = Some(latest);
        });
    }
}

/// Called once per run: refresh the daily check if it's due, then tell the
/// user about a newer version if one is already known.
pub fn maybe_check_and_notify(config: &Config) {
    if !checks_enabled(config) {
        return;
    }

    let state = load_state();
    if is_due(state.last_check, state.last_success, Local::now()) {
        // Record the attempt before making it, so an unreachable network
        // doesn't mean a check on every single invocation.
        update_state(|s| s.last_check = Some(Local::now()));
        spawn_background_check();
    }

    let latest = match state.latest_version {
        Some(ref v) if is_newer(v, CURRENT_VERSION) => v.clone(),
        _ => return,
    };

    eprintln!(
        "{} {}",
        "↑".green().bold(),
        t!(
            "update_available",
            current = CURRENT_VERSION,
            latest = latest
        )
        .bold()
    );

    let already_declined = state.declined_version.as_deref() == Some(latest.as_str());
    if !is_interactive() || already_declined {
        eprintln!("{}", t!("update_hint").dimmed());
        return;
    }

    let accepted = dialoguer::Confirm::new()
        .with_prompt(t!("update_prompt").to_string())
        .default(true)
        .interact()
        .unwrap_or(false);

    if accepted {
        // Update in place and carry on with whatever the user actually asked
        // for — a failed update must not take their command down with it.
        apply_update(&latest);
    } else {
        // Don't ask again for this version — the one-line notice is enough.
        update_state(|s| s.declined_version = Some(latest));
        eprintln!("{}", t!("update_hint").dimmed());
    }
}

// ──────────────────────────────────────────────
// Performing the update
// ──────────────────────────────────────────────

/// Re-running the install script replaces the binary in place — the same path
/// the user took to install yaak.
fn self_install_command() -> String {
    format!("curl -fsSL {} | bash", INSTALL_SCRIPT)
}

fn run_shell_command(command: &str) -> std::io::Result<std::process::ExitStatus> {
    if cfg!(windows) {
        Command::new("cmd").args(["/C", command]).status()
    } else {
        Command::new("bash").args(["-c", command]).status()
    }
}

/// What came of an update attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOutcome {
    /// yaak was upgraded.
    Updated,
    /// The user said no, or the update has to be done by hand.
    NotAttempted,
    /// The upgrade ran and failed.
    Failed,
}

fn report_result(result: std::io::Result<std::process::ExitStatus>, target: &str) -> UpdateOutcome {
    match result {
        Ok(status) if status.success() => {
            eprintln!(
                "{} {}",
                "✓".green().bold(),
                t!("update_succeeded", version = target).bold()
            );
            eprintln!("{}", t!("update_restart_hint").dimmed());
            UpdateOutcome::Updated
        }
        Ok(status) => {
            eprintln!(
                "{} {}",
                t!("error_prefix").red().bold(),
                t!("update_failed_code", code = status.code().unwrap_or(1))
            );
            UpdateOutcome::Failed
        }
        Err(e) => {
            eprintln!(
                "{} {}",
                t!("error_prefix").red().bold(),
                t!("update_failed", error = e)
            );
            UpdateOutcome::Failed
        }
    }
}

/// Apply an update using whichever strategy fits this install.
pub fn apply_update(latest: &str) -> UpdateOutcome {
    match current_strategy() {
        UpdateStrategy::SelfInstall => {
            report_result(run_shell_command(&self_install_command()), latest)
        }
        UpdateStrategy::Managed { label, command } => {
            eprintln!("{}", t!("update_managed_by", manager = label));
            eprintln!("  {}", command.cyan().bold());
            let run_now = is_interactive()
                && dialoguer::Confirm::new()
                    .with_prompt(t!("update_run_now").to_string())
                    .default(true)
                    .interact()
                    .unwrap_or(false);
            if !run_now {
                return UpdateOutcome::NotAttempted;
            }
            report_result(run_shell_command(&command), latest)
        }
        UpdateStrategy::Manual => {
            eprintln!("{}", t!("update_manual", url = RELEASES_PAGE));
            UpdateOutcome::NotAttempted
        }
    }
}

/// `yaak --update`: check on demand and upgrade if a newer release exists.
pub fn run_update() {
    eprintln!(
        "{} {}",
        t!("info_prefix").bold(),
        t!("update_current_version", version = CURRENT_VERSION).dimmed()
    );
    eprintln!("{} {}", t!("info_prefix").bold(), t!("update_checking"));

    let latest = match fetch_latest_version() {
        Some(v) => v,
        None => {
            eprintln!(
                "{} {}",
                t!("error_prefix").red().bold(),
                t!("update_check_failed", url = RELEASES_PAGE)
            );
            std::process::exit(1);
        }
    };

    // Remember what we learned so the daily check doesn't repeat the work.
    update_state(|s| {
        let now = Local::now();
        s.last_check = Some(now);
        s.last_success = Some(now);
        s.latest_version = Some(latest.clone());
    });

    if !is_newer(&latest, CURRENT_VERSION) {
        eprintln!(
            "{} {}",
            "✓".green().bold(),
            t!("update_up_to_date", version = CURRENT_VERSION).bold()
        );
        return;
    }

    eprintln!(
        "{} {}",
        t!("info_prefix").bold(),
        t!(
            "update_updating",
            current = CURRENT_VERSION,
            latest = latest
        )
    );

    if apply_update(&latest) == UpdateOutcome::Failed {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_and_prefixed_versions() {
        assert_eq!(parse_version("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version(" v0.1.5 "), Some((0, 1, 5)));
        assert_eq!(parse_version("v2.0"), Some((2, 0, 0)));
    }

    #[test]
    fn rejects_prereleases_and_junk() {
        assert_eq!(parse_version("v1.2.3-rc1"), None);
        assert_eq!(parse_version("1.2.3+build7"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version("nightly"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.1.6", "0.1.5"));
        assert!(is_newer("v0.2.0", "0.1.9"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.1.5", "0.1.5"));
        assert!(!is_newer("0.1.4", "0.1.5"));
        // A pre-release is never offered as an update.
        assert!(!is_newer("0.2.0-rc1", "0.1.5"));
        assert!(!is_newer("garbage", "0.1.5"));
    }

    #[test]
    fn checks_at_most_once_a_day_after_a_success() {
        let now = Local::now();
        let day_ago = now - Duration::hours(25);
        let hours_ago = |h: i64| Some(now - Duration::hours(h));

        // Never checked before.
        assert!(is_due(None, None, now));
        // Fresh result — nothing to do.
        assert!(!is_due(hours_ago(23), hours_ago(23), now));
        // Yesterday's result — time to look again.
        assert!(is_due(Some(day_ago), Some(day_ago), now));
    }

    #[test]
    fn a_check_that_produced_nothing_retries_hourly() {
        let now = Local::now();
        // Attempted 30 minutes ago, no answer yet: hold off.
        assert!(!is_due(Some(now - Duration::minutes(30)), None, now));
        // Attempted 2 hours ago, still no answer: try again.
        assert!(is_due(Some(now - Duration::hours(2)), None, now));
    }

    #[test]
    fn a_backwards_clock_does_not_cause_a_check_storm() {
        let now = Local::now();
        assert!(!is_due(Some(now + Duration::hours(2)), None, now));
    }

    #[test]
    fn parses_github_release_payload() {
        let body = r#"{"tag_name": "v0.2.0", "name": "v0.2.0", "draft": false}"#;
        assert_eq!(parse_latest_tag(body), Some("0.2.0".into()));
        assert_eq!(parse_latest_tag(r#"{"tag_name": ""}"#), None);
        assert_eq!(parse_latest_tag("not json"), None);
        assert_eq!(parse_latest_tag(r#"{"message": "Not Found"}"#), None);
    }

    #[test]
    fn detects_install_method_from_path() {
        use InstallMethod::*;
        assert_eq!(
            detect_install_method(Path::new("/opt/homebrew/bin/yaak")),
            Homebrew
        );
        assert_eq!(
            detect_install_method(Path::new("/usr/local/Cellar/yaak/0.1.5/bin/yaak")),
            Homebrew
        );
        assert_eq!(
            detect_install_method(Path::new("/home/ana/.cargo/bin/yaak")),
            Cargo
        );
        assert_eq!(
            detect_install_method(Path::new("/nix/store/abc123-yaak-0.1.5/bin/yaak")),
            Nix
        );
        assert_eq!(
            detect_install_method(Path::new(
                "C:\\Users\\ana\\scoop\\apps\\yaak\\current\\yaak.exe"
            )),
            Scoop
        );
        assert_eq!(
            detect_install_method(Path::new("/usr/bin/yaak")),
            SystemPackage
        );
        assert_eq!(
            detect_install_method(Path::new("/home/ana/.local/bin/yaak")),
            Script
        );
    }

    #[test]
    fn env_opt_out_reads_explicit_off_values() {
        for off in ["0", "false", "no", "off", "", "  OFF  "] {
            assert!(reads_as_off(off), "{off:?} should read as off");
        }
        for on in ["1", "true", "yes", "please"] {
            assert!(!reads_as_off(on), "{on:?} should read as on");
        }
    }

    #[test]
    fn self_install_pipes_the_install_script_to_bash() {
        let command = self_install_command();
        assert!(command.contains("https://getyaak.ai/install.sh"));
        assert!(command.starts_with("curl -fsSL"));
        assert!(command.ends_with("| bash"));
    }

    #[test]
    fn maps_package_managers_to_upgrade_commands() {
        let none = |_: &str| false;
        match strategy_for(InstallMethod::Homebrew, none) {
            UpdateStrategy::Managed { command, .. } => assert!(command.starts_with("brew upgrade")),
            other => panic!("expected a managed strategy, got {:?}", other),
        }
        match strategy_for(InstallMethod::Cargo, none) {
            UpdateStrategy::Managed { command, .. } => {
                assert_eq!(command, "cargo install yaak --force")
            }
            other => panic!("expected a managed strategy, got {:?}", other),
        }
    }

    #[test]
    fn system_package_needs_an_aur_helper() {
        assert_eq!(
            strategy_for(InstallMethod::SystemPackage, |_| false),
            UpdateStrategy::Manual
        );
        match strategy_for(InstallMethod::SystemPackage, |tool| tool == "yay") {
            UpdateStrategy::Managed { command, .. } => assert_eq!(command, "yay -S yaak-cli-bin"),
            other => panic!("expected a managed strategy, got {:?}", other),
        }
    }

    #[test]
    fn script_installs_can_update_themselves_on_unix() {
        let strategy = strategy_for(InstallMethod::Script, |_| false);
        if cfg!(windows) {
            assert_eq!(strategy, UpdateStrategy::Manual);
        } else {
            assert_eq!(strategy, UpdateStrategy::SelfInstall);
        }
    }
}
