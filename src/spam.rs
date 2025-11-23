use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use chrono::{DateTime, Local, NaiveDate};
use email::Mailbox;
use regex::Regex;

use crate::statistics::{SpamEmail, SpamResults};

#[derive(Debug, Copy, Clone, thiserror::Error)]
pub enum EmailError {
    #[error("message is missing spam result header")]
    MissingOrMalformedHeader,
}

fn make_spam_email(message: String, date_received: NaiveDate) -> Result<SpamEmail, anyhow::Error> {
    static SPAMD_RESULT_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"[^\[]*\[(-?[.0-9]*)").unwrap());

    let parsed = email::MimeMessage::parse(message.as_str())?;
    let headers = parsed.headers;
    let spam_result = headers
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

    let is_spam = headers
        .get("X-Spam".to_string())
        .and_then(|header| {
            header
                .get_value::<String>()
                .ok()
                .map(|value| "Yes" == &value)
        })
        .unwrap_or(false);

    let from = headers
        .get("From".to_string())
        .ok_or(EmailError::MissingOrMalformedHeader)?
        .get_value::<String>()?;

    let spam_result: f64 = spam_result.as_str().parse()?;
    Ok(SpamEmail {
        date_received,
        spam_result,
        is_spam,
        from,
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

/// Return a list of the top spam-sending domains
fn top_offending_domains<S, I>(iter: I) -> Vec<(String, usize)>
where
    I: Iterator<Item = S>,
    S: AsRef<SpamEmail>,
{
    let mut counts = HashMap::<String, usize>::new();
    let mut error_count = 0;
    let misclassified_spam = iter.filter(|email| !email.as_ref().is_spam);
    for message in misclassified_spam {
        let message = message.as_ref();
        let Ok(mailbox) = message.from.parse::<Mailbox>() else {
            error_count += 1;
            continue;
        };

        let mut address = mailbox.address.split("@");
        address.next();
        let Some(domain) = address.next() else {
            error_count += 1;
            continue;
        };
        let count = counts.entry(domain.to_string()).or_default();
        *count += 1;
    }

    eprintln!(
        "{} addresses failed to parse while determining the spammiest domains",
        error_count
    );

    let mut counts = counts.into_iter().collect::<Vec<_>>();
    counts.sort_by(|(_, one), (_, two)| two.cmp(one));
    counts
}

pub fn domain_report(spam: impl Iterator<Item = SpamEmail>) -> String {
    let domains = top_offending_domains(spam);
    "<h3>Misclassified Domains</h3>".to_string()
        + "<p>Domains that have sent mail misclassified as ham.</p>"
        + r#"<ul style="list-style-type:none;">"#
        + &domains
            .iter()
            .map(|(domain, count)| format!("<li>{}: {}</li>\n", domain, count))
            .collect::<Vec<_>>()
            .join("\n")
        + "</ul>"
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
        if let Ok(users) = domain?.path().read_dir() {
            for user in users {
                spam.append(&mut list_spam_maildir(user?.path())?);
            }
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
