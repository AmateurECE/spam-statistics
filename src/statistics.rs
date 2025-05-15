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

pub struct SpamEmail {
    received: NaiveDate,
    spam_result: SpamResult,
    is_spam: bool,
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
        for email in value.into_iter() {
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
            .into_iter()
            .map(|bin| (bin, *bins.get(&bin).unwrap_or(&0)))
            .collect();
        SpamResultsDistribution(bins)
    }
}

fn rate_distribution(is_spam: bool, value: &SpamResults) -> Vec<(NaiveDate, Occurrences)> {
    let classified = value.into_iter().filter(|email| is_spam == email.is_spam);
    let dates_received = classified.clone().map(|email| email.received);
    let Some(earliest) = dates_received.clone().min() else {
        return Vec::new();
    };

    // INVARIANT: There is definitely a max value here, because there was a min value.
    let latest = dates_received.max().unwrap();

    let delta = (latest - earliest).num_days();
    let delta: usize = delta.try_into().expect(&format!(
        "{} seems like the wrong number of emails for this inbox",
        delta
    ));

    earliest
        .iter_days()
        .take(delta)
        .map(|day| {
            let occurrences = classified
                .clone()
                .filter(|email| email.received == day)
                .count();
            (day, occurrences)
        })
        .collect()
}

/// The distribution of spam received per day.
pub struct SpamRateDistribution(Vec<(NaiveDate, Occurrences)>);

impl_into_iterator!(SpamRateDistribution, (NaiveDate, Occurrences), 0);

impl From<&SpamResults> for SpamRateDistribution {
    fn from(value: &SpamResults) -> Self {
        SpamRateDistribution(rate_distribution(true, value))
    }
}

/// The distribution of email erroneously classified as ham received per day.
pub struct MissRateDistribution(Vec<(NaiveDate, Occurrences)>);

impl_into_iterator!(MissRateDistribution, (NaiveDate, Occurrences), 0);

impl From<&SpamResults> for MissRateDistribution {
    fn from(value: &SpamResults) -> Self {
        MissRateDistribution(rate_distribution(false, value))
    }
}
