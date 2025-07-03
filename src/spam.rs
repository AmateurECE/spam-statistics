use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use chrono::{DateTime, Local, NaiveDate};
use regex::Regex;

use crate::statistics::{SpamEmail, SpamResults};

// TODO: Replace with thiserror
#[derive(Debug, Copy, Clone, thiserror::Error)]
pub enum EmailError {
    #[error("message is missing spam result header")]
    MissingOrMalformedHeader,
}

fn make_spam_email(message: String, date_received: NaiveDate) -> Result<SpamEmail, anyhow::Error> {
    static SPAMD_RESULT_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"[^\[]*\[(-?[.0-9]*)").unwrap());

    let parsed = email::MimeMessage::parse(message.as_str())?;
    let spam_result = parsed
        .headers
        .get("X-Spamd-Result".to_string())
        .ok_or(EmailError::MissingOrMalformedHeader)?
        .get_value::<String>()?;

    let parse_error = EmailError::MissingOrMalformedHeader;
    let spam_result = if SPAMD_RESULT_REGEX.is_match(&spam_result) {
        SPAMD_RESULT_REGEX
            .captures_iter(&spam_result)
            .next()
            .ok_or(parse_error)?
            // Skip zeroeth capture, because that's the whole string
            .get(1)
            .ok_or(parse_error)
    } else {
        Err(parse_error)
    }?;

    let is_spam = parsed
        .headers
        .get("X-Spam".to_string())
        .and_then(|header| {
            header
                .get_value::<String>()
                .ok()
                .map(|value| "Yes" == &value)
        })
        .unwrap_or(false);

    let spam_result: f64 = spam_result.as_str().parse()?;
    Ok(SpamEmail {
        date_received,
        spam_result,
        is_spam,
    })
}

fn load_spam<P>(path: P) -> anyhow::Result<SpamEmail>
where
    P: AsRef<Path>,
{
    let mut file = File::open(&path)?;

    // See maildir(5)
    let date_received: DateTime<Local> = file.metadata()?.modified()?.into();

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    make_spam_email(contents, date_received.date_naive())
}

fn list_spam_maildir<P>(path: P) -> anyhow::Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
{
    let mut spam: Vec<PathBuf> = Vec::new();
    let spam_folder = path.as_ref().join(".Spam");

    // See maildir(5)
    let read = spam_folder.join("cur");
    if read.is_dir() {
        let mut emails = read
            .read_dir()?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .collect::<Vec<PathBuf>>();
        spam.append(&mut emails);
    }

    let unread = spam_folder.join("new");
    if unread.is_dir() {
        let mut emails = unread
            .read_dir()?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .collect::<Vec<PathBuf>>();
        spam.append(&mut emails);
    }

    Ok(spam)
}

pub fn load_spam_maildir<P>(path: P) -> anyhow::Result<SpamResults>
where
    P: AsRef<Path>,
{
    Ok(list_spam_maildir(path)?
        .into_iter()
        .filter_map(|email| load_spam(email).ok())
        .collect::<SpamResults>())
}

fn list_spam_virtual_mailbox_base<P>(path: P) -> Result<Vec<PathBuf>, anyhow::Error>
where
    P: AsRef<Path>,
{
    let mut spam = Vec::new();
    let domains = path.as_ref().read_dir()?;
    for domain in domains {
        let users = domain?.path().read_dir()?;
        for user in users {
            spam.append(&mut list_spam_maildir(user?.path())?);
        }
    }

    Ok(spam)
}

pub fn load_spam_virtual_mailbox_base<P>(path: P) -> Result<SpamResults, anyhow::Error>
where
    P: AsRef<Path>,
{
    let spam = list_spam_virtual_mailbox_base(path)?;
    let mut spam_results = Vec::new();
    for path in spam {
        match load_spam(path) {
            Ok(spam_email) => spam_results.push(spam_email),
            Err(error) => eprintln!("{}", error),
        }
    }

    Ok(spam_results)
}
