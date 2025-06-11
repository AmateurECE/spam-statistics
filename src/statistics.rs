use std::collections::HashMap;

use chrono::NaiveDate;

/// A [SpamResult] is the value assigned to an email by Rspamd that summarizes its spam or ham
/// -like attributes.
pub type SpamResult = f64;

/// The number of occurrences of an event.
pub type Occurrences = usize;

#[derive(Debug)]
pub struct SpamEmail {
    pub date_received: NaiveDate,
    pub spam_result: SpamResult,
    pub is_spam: bool,
}

/// A series of data points that correlate a [SpamResult] assigned to an email with the date that
/// the email was received.
pub type SpamResults = Vec<SpamEmail>;

/// Spam results are sorted into integer-sized bins for calculating the distribution.
pub type SpamResultBin = i32;

/// The [SpamResult]s of the emails over integer-sized bins.
pub fn quantize_spam_results<'a, I>(iter: I) -> Vec<SpamResultBin>
where
    I: Iterator<Item = &'a SpamEmail>,
{
    iter.map(|email| email.spam_result as SpamResultBin)
        .collect::<Vec<_>>()
}

/// Return a list of the date received for each spam email.
pub fn dates_received<'a, I>(iter: I) -> Vec<NaiveDate>
where
    I: Iterator<Item = &'a SpamEmail>,
{
    iter.map(|email| email.date_received).collect::<Vec<_>>()
}

#[derive(Default)]
struct SpamCount {
    spam: Occurrences,
    ham: Occurrences,
}

fn spam_counts<'a, I>(emails: I) -> Vec<(NaiveDate, SpamCount)>
where
    I: Iterator<Item = &'a SpamEmail> + Clone,
{
    let mut counts = HashMap::new();
    for email in emails {
        let count: &mut SpamCount = counts.entry(email.date_received).or_default();
        if email.is_spam {
            count.spam += 1;
        } else {
            count.ham += 1;
        }
    }

    let dates_received = counts.keys();
    let Some(earliest) = dates_received.clone().min() else {
        return Vec::new();
    };

    // INVARIANT: There is definitely a max value here, because there was a min value.
    let latest = dates_received.max().unwrap();

    let delta = (*latest - *earliest).num_days() + 1;
    let delta: usize = delta.try_into().unwrap_or_else(|_| {
        panic!(
            "{} seems like the wrong number of emails for this inbox",
            delta
        )
    });

    earliest
        .iter_days()
        .take(delta)
        .map(|day| {
            let count = counts.remove(&day).unwrap_or(SpamCount::default());
            (day, count)
        })
        .collect()
}

/// The percentage of correctly classified spam received on each day.
pub fn misclassification_rate<'a, I>(iter: I) -> Vec<(NaiveDate, f64)>
where
    I: Iterator<Item = &'a SpamEmail> + Clone,
{
    spam_counts(iter)
        .into_iter()
        .map(|(date, count)| {
            let spam = count.spam as f64;
            let ham = count.ham as f64;
            (date, ham / (spam + ham))
        })
        .collect()
}
