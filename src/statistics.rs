use std::collections::HashMap;

use chrono::NaiveDate;

macro_rules! impl_into_iterator {
    ($owner:ty, $datum:ty, $field:tt) => {
        impl<'a> IntoIterator for &'a $owner {
            type Item = &'a $datum;
            type IntoIter = <&'a Vec<$datum> as IntoIterator>::IntoIter;

            fn into_iter(self) -> Self::IntoIter {
                (&self.$field).into_iter()
            }
        }
    };
}

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
pub type SpamResultBin = i64;

/// The distribution of the frequency of [SpamResult]s over a given bin size.
pub struct SpamResultsDistribution(Vec<(SpamResultBin, Occurrences)>);

impl_into_iterator!(SpamResultsDistribution, (SpamResultBin, Occurrences), 0);

impl From<&SpamResults> for SpamResultsDistribution {
    fn from(value: &SpamResults) -> Self {
        let mut bins: HashMap<SpamResultBin, Occurrences> = HashMap::new();
        for email in value {
            let bin = email.spam_result as SpamResultBin;
            *bins.entry(bin).or_insert(0) += 1;
        }

        let keys = bins.keys();
        let Some(min) = keys.clone().min() else {
            return SpamResultsDistribution(Vec::new());
        };
        // INVARIANT: If there is a min, there is definitely a max.
        let max = keys.max().unwrap();

        let bins = (*min..*max)
            .map(|bin| (bin, *bins.get(&bin).unwrap_or(&0)))
            .collect();
        SpamResultsDistribution(bins)
    }
}

#[derive(Default)]
struct SpamCount {
    spam: Occurrences,
    ham: Occurrences,
}

fn spam_counts(emails: &SpamResults) -> Vec<(NaiveDate, SpamCount)> {
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

    let delta = (*latest - *earliest).num_days();
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
pub struct SpamRateDistribution(Vec<(NaiveDate, f64)>);

impl_into_iterator!(SpamRateDistribution, (NaiveDate, f64), 0);

impl From<&SpamResults> for SpamRateDistribution {
    fn from(value: &SpamResults) -> Self {
        SpamRateDistribution(
            spam_counts(value)
                .into_iter()
                .map(|(date, count)| {
                    let spam = count.spam as f64;
                    let ham = count.ham as f64;
                    (date, ham / (spam + ham))
                })
                .collect(),
        )
    }
}

/// The distribution total spam received per day.
pub struct TotalSpamDistribution(Vec<(NaiveDate, Occurrences)>);

impl_into_iterator!(TotalSpamDistribution, (NaiveDate, Occurrences), 0);

impl From<&SpamResults> for TotalSpamDistribution {
    fn from(value: &SpamResults) -> Self {
        TotalSpamDistribution(
            spam_counts(value)
                .into_iter()
                .map(|(date, count)| (date, (count.spam + count.ham)))
                .collect(),
        )
    }
}
