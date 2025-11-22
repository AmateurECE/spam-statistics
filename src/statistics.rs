use std::collections::HashMap;

use chrono::{Datelike, Days, Local, NaiveDate};

/// A [SpamResult] is the value assigned to an email by Rspamd that summarizes its spam or ham
/// -like attributes.
pub type SpamResult = f64;

/// The number of occurrences of an event.
pub type Occurrences = usize;

#[derive(Clone, Debug)]
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

// TODO: functions in here should return iterators.

/// The [SpamResult]s of the emails over integer-sized bins.
pub fn quantize_spam_results<'a, I>(
    iter: I,
) -> impl Iterator<Item = SpamResultBin> + Clone + use<'a, I>
where
    I: Iterator<Item = &'a SpamEmail> + Clone,
{
    iter.map(|email| email.spam_result as SpamResultBin)
}

#[derive(Clone, Default)]
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
pub fn misclassification_rate<'a, I>(iter: I) -> impl Iterator<Item = (NaiveDate, f64)> + Clone
where
    I: Iterator<Item = &'a SpamEmail> + Clone,
{
    spam_counts(iter).into_iter().map(|(date, count)| {
        let spam = count.spam as f64;
        let ham = count.ham as f64;
        (date, ham / (spam + ham))
    })
}

pub fn last_n_days(data: &[SpamEmail], n_days: Days) -> Option<&[SpamEmail]> {
    let today = Local::now().date_naive();
    let earliest_date = today.checked_sub_days(n_days).unwrap();

    if data.is_empty() {
        return None;
    }

    if data[0].date_received > earliest_date {
        Some(data)
    } else if data.last().unwrap().date_received < earliest_date {
        None
    } else {
        let i = data.partition_point(|email| email.date_received < earliest_date);
        Some(&data[i..])
    }
}

/// Get the date of the previous Sunday given a date.
fn previous_sunday(date: &NaiveDate) -> NaiveDate {
    let current_weekday = Datelike::weekday(date) as u64;
    date.checked_sub_days(Days::new(current_weekday)).unwrap()
}

/// INVARIANT: The vector must be sorted.
#[derive(Clone)]
pub struct WeeklyBins<'a>(Vec<&'a SpamEmail>);
impl Iterator for WeeklyBins<'_> {
    type Item = SpamEmail;

    fn next(&mut self) -> Option<Self::Item> {
        let mut email = self.0.pop().cloned()?;
        email.date_received = previous_sunday(&email.date_received);
        Some(email)
    }
}

impl<'a> WeeklyBins<'a> {
    pub fn take_weeks(self, num: u64) -> impl Iterator<Item = SpamEmail> + Clone + use<'a> {
        const DAYS_PER_WEEK: u64 = 7;
        let now = Local::now().date_naive();
        let earliest_date = previous_sunday(&now)
            .checked_sub_days(Days::new((num - 1) * DAYS_PER_WEEK))
            .unwrap();
        self.into_iter()
            .take_while(move |e| e.date_received > earliest_date)
    }
}

pub fn weekly_bins<'a, I>(iter: I) -> WeeklyBins<'a>
where
    I: Iterator<Item = &'a SpamEmail>,
{
    let mut email = iter.collect::<Vec<&'a SpamEmail>>();
    email.sort_by(|a, b| a.date_received.cmp(&b.date_received));
    WeeklyBins(email)
}
