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
        todo!()
    }
}

/// The distribution of spam received per day.
pub struct SpamRateDistribution(Vec<(NaiveDate, Occurrences)>);

impl_into_iterator!(SpamRateDistribution, (NaiveDate, Occurrences), 0);

impl From<&SpamResults> for SpamRateDistribution {
    fn from(value: &SpamResults) -> Self {
        let classified_as_spam = value.into_iter().filter(|email| email.is_spam);
        let dates_received = classified_as_spam.clone().map(|email| email.received);
        let Some(earliest) = dates_received.clone().min() else {
            return SpamRateDistribution(Vec::new());
        };

        // INVARIANT: There is definitely a max value here, because there was a min value.
        let latest = dates_received.max().unwrap();

        let delta = (latest - earliest).num_days();
        let delta: usize = delta.try_into().expect(&format!(
            "{} seems like the wrong number of emails for this inbox",
            delta
        ));
        let distribution = earliest
            .iter_days()
            .take(delta)
            .map(|day| {
                let occurrences = classified_as_spam
                    .clone()
                    .filter(|email| email.received == day)
                    .count();
                (day, occurrences)
            })
            .collect();

        SpamRateDistribution(distribution)
    }
}

/// The distribution of email erroneously classified as ham received per day.
pub struct MissRateDistribution(Vec<(NaiveDate, Occurrences)>);

impl_into_iterator!(MissRateDistribution, (NaiveDate, Occurrences), 0);

impl From<&SpamResults> for MissRateDistribution {
    fn from(value: &SpamResults) -> Self {
        todo!()
    }
}
