use std::{
    process::{Command, Stdio},
    sync::LazyLock,
};

use regex::Regex;

use crate::statistics::Occurrences;

static ACTION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Messages with action ([^:]*): ([0-9]*),").unwrap());

#[derive(Clone, Debug, thiserror::Error)]
pub enum RspamdError {
    #[error("subprocess")]
    Subprocess(String),
}

#[derive(Default)]
pub struct MessageActions {
    reject: Occurrences,
    greylist: Occurrences,
    add_header: Occurrences,
    no_action: Occurrences,
}

impl MessageActions {
    pub fn actions(&self) -> Vec<(String, f64)> {
        let Self {
            reject,
            greylist,
            add_header,
            no_action,
        } = self;
        let total = (reject + greylist + add_header + no_action) as f64;
        vec![
            ("Reject".to_string(), (*reject as f64) / total),
            ("Greylist".to_string(), (*greylist as f64) / total),
            ("Add Header".to_string(), (*add_header as f64) / total),
            ("No Action".to_string(), (*no_action as f64) / total),
        ]
    }
}

pub struct RspamdStatistics {
    pub statistics: Vec<String>,
    pub message_actions: MessageActions,
}

fn rspamd_error<E>(e: E) -> RspamdError
where
    E: ToString,
{
    RspamdError::Subprocess(e.to_string())
}

/// Load statistics from rspamd.
pub fn load_rspamd_statistics() -> Result<RspamdStatistics, RspamdError> {
    let rspamd = Command::new("rspamc")
        .arg("stat")
        .stdout(Stdio::piped())
        .spawn()
        .map_err(rspamd_error)?;

    let output = rspamd.wait_with_output().map_err(rspamd_error)?;
    if !output.status.success() {
        return Err(RspamdError::Subprocess(
            String::from_utf8(output.stderr).map_err(rspamd_error)?,
        ));
    }

    let output = String::from_utf8_lossy(&output.stdout);
    let statistics = output
        .split("\n")
        .map(ToString::to_string)
        .collect::<Vec<String>>();

    let mut message_actions = MessageActions::default();
    for line in statistics.as_slice() {
        let captures = ACTION_REGEX.captures(line);
        let Some(capture) = captures else {
            continue;
        };
        let occurrences: usize = capture[2].parse().unwrap();
        match &capture[1] {
            "reject" => message_actions.reject = occurrences,
            "greylist" => message_actions.greylist = occurrences,
            "add header" => message_actions.add_header = occurrences,
            "no action" => message_actions.no_action = occurrences,
            &_ => continue,
        }
    }

    Ok(RspamdStatistics {
        statistics,
        message_actions,
    })
}
