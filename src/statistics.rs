use core::hash;
use std::{collections::HashMap, vec};

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
    pub from: String,
}

impl AsRef<SpamEmail> for SpamEmail {
    fn as_ref(&self) -> &SpamEmail {
        self
    }
}

/// A series of data points that correlate a [SpamResult] assigned to an email with the date that
/// the email was received.
pub type SpamResults = Vec<SpamEmail>;

/// Spam results are sorted into integer-sized bins for calculating the distribution.
pub type SpamResultBin = i32;

/// The [SpamResult]s of the emails over integer-sized bins.
pub fn quantize_spam_results<'a, I, S>(
    iter: I,
) -> impl Iterator<Item = SpamResultBin> + Clone + use<'a, I, S>
where
    I: Iterator<Item = S> + Clone,
    S: AsRef<SpamEmail>,
{
    iter.map(|email| email.as_ref().spam_result as SpamResultBin)
}

#[derive(Clone, Default)]
struct SpamCount {
    spam: Occurrences,
    ham: Occurrences,
}

fn spam_counts<I, S>(emails: I) -> impl Iterator<Item = (NaiveDate, SpamCount)> + Clone
where
    I: Iterator<Item = S> + Clone,
    S: AsRef<SpamEmail>,
{
    let mut counts = HashMap::new();
    for email in emails {
        let email = email.as_ref();
        let count: &mut SpamCount = counts.entry(email.date_received).or_default();
        if email.is_spam {
            count.spam += 1;
        } else {
            count.ham += 1;
        }
    }

    let mut counts = counts.into_iter().collect::<Vec<_>>();
    counts.sort_by(|(one, _), (two, _)| one.cmp(two));
    counts.into_iter()
}

/// The percentage of correctly classified spam received on each day.
pub fn misclassification_rate<I, S>(iter: I) -> impl Iterator<Item = (NaiveDate, f64)> + Clone
where
    I: Iterator<Item = S> + Clone,
    S: AsRef<SpamEmail> + Clone,
{
    spam_counts(iter).map(|(date, count)| {
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

//
// WeeklyBins
//

/// INVARIANT: The vector must be sorted.
#[derive(Clone)]
pub struct WeeklyBinIter<S>(Vec<S>);
impl<S> Iterator for WeeklyBinIter<S>
where
    S: AsRef<SpamEmail>,
{
    type Item = SpamEmail;

    fn next(&mut self) -> Option<Self::Item> {
        let mut email = self.0.pop()?.as_ref().clone();
        email.date_received = previous_sunday(&email.date_received);
        Some(email)
    }
}

impl<S> WeeklyBinIter<S>
where
    S: AsRef<SpamEmail> + Clone,
{
    pub fn take_weeks(self, num: u64) -> impl Iterator<Item = SpamEmail> + Clone + use<S> {
        const DAYS_PER_WEEK: u64 = 7;
        let now = Local::now().date_naive();
        let earliest_date = previous_sunday(&now)
            .checked_sub_days(Days::new((num - 1) * DAYS_PER_WEEK))
            .unwrap();
        self.into_iter()
            .take_while(move |e| e.date_received > earliest_date)
    }
}

pub trait WeeklyBins<S> {
    fn weekly_bins(self) -> WeeklyBinIter<S>;
}

impl<I, S> WeeklyBins<S> for I
where
    I: Iterator<Item = S>,
    S: AsRef<SpamEmail>,
{
    fn weekly_bins(self) -> WeeklyBinIter<S> {
        let mut email = self.collect::<Vec<_>>();
        email.sort_by(|a, b| a.as_ref().date_received.cmp(&b.as_ref().date_received));
        WeeklyBinIter(email)
    }
}

//
// IntoBins
//

pub trait IntoBins {
    type Item;
    fn into_bins(self) -> vec::IntoIter<Self::Item>;
}

impl<I, X> IntoBins for I
where
    I: Iterator<Item = X>,
    X: Ord + Eq + hash::Hash,
{
    type Item = (X, usize);
    fn into_bins(self) -> vec::IntoIter<Self::Item> {
        let mut counts = HashMap::new();
        for item in self {
            let entry = counts.entry(item).or_default();
            *entry += 1;
        }

        let mut counts = counts.into_iter().collect::<Vec<_>>();
        counts.sort_by(|(one, _), (two, _)| one.cmp(two));
        counts.into_iter()
    }
}
