//! Adapter around the Legendary CLI.
//!
//! This is the only module allowed to spawn the Legendary process. Everything
//! else in the app talks to the typed API exposed here, which keeps command
//! line flags, environment and output parsing in one place.
//!
//! Legendary's own configuration is pinned to Wisp's data folder through
//! `LEGENDARY_CONFIG_PATH`, so a Legendary or Heroic installation the user
//! already has is never read or written.

pub mod json;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::platform::hide_console;

/// Platform Wisp asks Legendary about. Epic titles are Windows builds; on
/// Linux they run through a compatibility layer (chapter 2), so the asset
/// platform stays "Windows" there too.
pub const GAME_PLATFORM: &str = "Windows";

/// What `legendary status --json` reports. It works logged out, which makes
/// it the cheapest way to ask "is there an account attached?".
#[derive(Debug, Clone, Deserialize)]
pub struct Status {
    pub account: String,
    #[serde(default)]
    pub games_available: u32,
    #[serde(default)]
    pub games_installed: u32,
    #[serde(default)]
    pub egl_sync_enabled: bool,
}

impl Status {
    /// Legendary prints this placeholder instead of an account name.
    const LOGGED_OUT: &'static str = "<not logged in>";

    pub fn logged_in(&self) -> bool {
        self.account != Self::LOGGED_OUT && !self.account.trim().is_empty()
    }

    pub fn account_name(&self) -> Option<&str> {
        self.logged_in().then_some(self.account.as_str())
    }
}

/// Result of running Legendary once.
#[derive(Debug, Clone)]
pub struct Run {
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    pub fn succeeded(&self) -> bool {
        self.status == Some(0)
    }

    /// stdout if the process succeeded, otherwise an error carrying whichever
    /// stream Legendary actually wrote its complaint to.
    pub fn into_stdout(self) -> Result<String> {
        if self.succeeded() {
            return Ok(self.stdout);
        }
        let detail = if self.stderr.trim().is_empty() {
            self.stdout.trim().to_owned()
        } else {
            self.stderr.trim().to_owned()
        };
        Err(Error::Legendary(if detail.is_empty() {
            format!("legendary exited with status {:?}", self.status)
        } else {
            detail
        }))
    }
}

#[derive(Debug, Clone)]
pub struct Legendary {
    executable: PathBuf,
    config_dir: PathBuf,
}

impl Legendary {
    pub fn new(executable: PathBuf, config_dir: PathBuf) -> Self {
        Self {
            executable,
            config_dir,
        }
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn is_installed(&self) -> bool {
        self.executable.is_file()
    }

    /// Builds a command with Wisp's isolated configuration already applied.
    /// Callers that need to stream output (the download queue) build on this.
    pub fn command<I, S>(&self, args: I) -> tokio::process::Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = tokio::process::Command::new(&self.executable);
        command
            .args(args)
            .env("LEGENDARY_CONFIG_PATH", &self.config_dir)
            // Legendary colours its output when it thinks a terminal is
            // attached; the escape codes would only confuse our parsers.
            .env("NO_COLOR", "1")
            .kill_on_drop(true);
        hide_console(&mut command);
        command
    }

    /// Runs Legendary to completion and collects both output streams.
    pub async fn run<I, S>(&self, args: I) -> Result<Run>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        if !self.is_installed() {
            return Err(Error::Runtime(format!(
                "Legendary is not installed at {}",
                self.executable.display()
            )));
        }

        let output = self.command(args).output().await.map_err(|e| {
            Error::Runtime(format!("could not run {}: {e}", self.executable.display()))
        })?;

        Ok(Run {
            status: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    /// Runs a command that prints JSON and parses the result.
    async fn run_json<I, S>(&self, args: I) -> Result<Value>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let stdout = self.run(args).await?.into_stdout()?;
        serde_json::from_str(&stdout)
            .map_err(|e| Error::Legendary(format!("could not read Legendary's JSON output: {e}")))
    }

    /// Account and library counts. Works whether or not anyone is signed in.
    pub async fn status(&self) -> Result<Status> {
        let value = self.run_json(["status", "--json"]).await?;
        serde_json::from_value(value)
            .map_err(|e| Error::Legendary(format!("unexpected status output: {e}")))
    }

    /// Everything the account owns, with each entry's DLC nested under `dlcs`.
    pub async fn list(&self, force_refresh: bool) -> Result<Vec<Value>> {
        let mut args = vec!["list", "--json", "--platform", GAME_PLATFORM];
        if force_refresh {
            args.push("--force-refresh");
        }
        as_array(self.run_json(args).await?, "list")
    }

    /// Games (and DLC) installed on this machine.
    pub async fn list_installed(&self) -> Result<Vec<Value>> {
        as_array(
            self.run_json(["list-installed", "--json"]).await?,
            "list-installed",
        )
    }

    /// Detailed information for a single title.
    pub async fn info(&self, app_name: &str) -> Result<Value> {
        self.run_json(["info", app_name, "--json"]).await
    }

    /// Completes a login with the authorization code from Epic's redirect.
    pub async fn auth_with_code(&self, code: &str) -> Result<()> {
        let run = self.run(["auth", "--code", code]).await?;
        if run.succeeded() {
            return Ok(());
        }
        run.into_stdout().map(|_| ())
    }

    /// Removes the stored credentials.
    pub async fn logout(&self) -> Result<()> {
        let run = self.run(["auth", "--delete"]).await?;
        if run.succeeded() {
            return Ok(());
        }
        run.into_stdout().map(|_| ())
    }

    /// The version Legendary reports for itself.
    pub async fn version(&self) -> Result<String> {
        let run = self.run(["--version"]).await?;
        if !run.succeeded() {
            return run.into_stdout().map(|_| String::new());
        }
        // Legendary prints this on stdout, but fall back to stderr so a future
        // change of stream does not break the startup check.
        parse_version(&run.stdout)
            .or_else(|| parse_version(&run.stderr))
            .ok_or_else(|| {
                Error::Legendary(format!(
                    "could not read a version out of: {}",
                    run.stdout.trim()
                ))
            })
    }
}

fn as_array(value: Value, command: &str) -> Result<Vec<Value>> {
    match value {
        Value::Array(items) => Ok(items),
        other => Err(Error::Legendary(format!(
            "expected a list from `legendary {command}`, got {}",
            kind_of(&other)
        ))),
    }
}

fn kind_of(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "a list",
        Value::Object(_) => "an object",
    }
}

/// Pulls `0.21.1` out of `legendary version "0.21.1", codename "Lowlife"`.
pub fn parse_version(output: &str) -> Option<String> {
    let line = output
        .lines()
        .find(|line| line.contains("legendary version"))?;
    let rest = line.split_once("legendary version")?.1.trim_start();
    let quoted = rest.strip_prefix('"')?;
    let (version, _) = quoted.split_once('"')?;
    let version = version.trim();
    (!version.is_empty()).then(|| version.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_version_legendary_actually_prints() {
        let output = "legendary version \"0.21.1\", codename \"Lowlife\"\n";
        assert_eq!(parse_version(output).as_deref(), Some("0.21.1"));
    }

    #[test]
    fn ignores_leading_noise() {
        let output = "some warning on the first line\nlegendary version \"0.20.34\", codename \"Direct Intervention\"\n";
        assert_eq!(parse_version(output).as_deref(), Some("0.20.34"));
    }

    #[test]
    fn rejects_output_without_a_version() {
        assert_eq!(parse_version(""), None);
        assert_eq!(parse_version("command not found"), None);
        assert_eq!(parse_version("legendary version \"\""), None);
    }

    #[test]
    fn run_reports_the_stream_that_carries_the_error() {
        let failed = Run {
            status: Some(1),
            stdout: String::new(),
            stderr: "not logged in".into(),
        };
        let err = failed.into_stdout().unwrap_err();
        assert_eq!(err.kind(), "legendary");
        assert!(err.to_string().contains("not logged in"));
    }

    #[test]
    fn status_recognises_the_logged_out_placeholder() {
        let logged_out: Status =
            serde_json::from_str(r#"{"account":"<not logged in>","games_available":0}"#).unwrap();
        assert!(!logged_out.logged_in());
        assert_eq!(logged_out.account_name(), None);

        let logged_in: Status =
            serde_json::from_str(r#"{"account":"someone@example.com","games_available":42}"#)
                .unwrap();
        assert!(logged_in.logged_in());
        assert_eq!(logged_in.account_name(), Some("someone@example.com"));
        assert_eq!(logged_in.games_available, 42);
    }

    #[test]
    fn a_non_list_response_is_reported_clearly() {
        let err = as_array(serde_json::json!({"error": "nope"}), "list").unwrap_err();
        assert!(err.to_string().contains("expected a list"));
        assert!(err.to_string().contains("an object"));
    }

    #[test]
    fn run_falls_back_to_stdout_when_stderr_is_empty() {
        let failed = Run {
            status: Some(2),
            stdout: "bad argument".into(),
            stderr: "   ".into(),
        };
        let err = failed.into_stdout().unwrap_err();
        assert!(err.to_string().contains("bad argument"));
    }
}
