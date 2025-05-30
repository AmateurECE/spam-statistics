use std::process::{Command, Stdio};

#[derive(Clone, Debug, thiserror::Error)]
enum RspamdError {
    #[error("gnuplot")]
    Subprocess(String),
}

/// Load statistics from rspamd.
pub fn load_rspamd_statistics() -> anyhow::Result<Vec<String>> {
    let rspamd = Command::new("rspamc")
        .arg("stat")
        .stdout(Stdio::piped())
        .spawn()?;

    let output = rspamd.wait_with_output()?;
    if !output.status.success() {
        return Err(RspamdError::Subprocess(String::from_utf8(output.stderr)?).into());
    }

    let output = String::from_utf8_lossy(&output.stdout);
    Ok(output.split("\n").map(ToString::to_string).collect())
}
