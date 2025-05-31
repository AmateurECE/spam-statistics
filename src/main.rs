use clap::Parser;
use core::error::Error;
use email::MessageTemplate;
use lettre::{SmtpTransport, Transport};
use plot::{Color, Image, Quantity};
use rspamd::{load_rspamd_statistics, RspamdStatistics};
use spam::load_spam_results;
use statistics::{
    SpamRateDistribution, SpamResults, SpamResultsDistribution, TotalSpamDistribution,
};
use std::{
    ffi::{c_char, CStr},
    io,
    path::Path,
};

mod email;
mod plot;
mod rspamd;
mod spam;
mod statistics;

fn get_hostname() -> Result<String, anyhow::Error> {
    let mut buffer: [u8; 64] = [0; 64];
    let result = unsafe { libc::gethostname(buffer.as_mut_ptr() as *mut c_char, buffer.len()) };
    if 0 != result {
        return Err(io::Error::last_os_error().into());
    }
    let hostname = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
    Ok(hostname.to_str()?.to_owned())
}

fn pie_colors(action: &str) -> Color {
    match action {
        "No Action" => Color::Green,
        "Greylist" => Color::Blue,
        "Add Header" => Color::Orange,
        "Reject" => Color::Red,
        &_ => panic!("Cannot determine pie chart color for an unknown action"),
    }
}

#[allow(dead_code)]
fn spam_statistics<P>(domain: &str, virtual_mailbox_base: P) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
{
    let spam_results = load_spam_results(virtual_mailbox_base)?;
    if spam_results.is_empty() {
        println!("No spam.");
        return Ok(());
    }

    let RspamdStatistics {
        statistics,
        message_actions,
    } = load_rspamd_statistics()?;
    let message_actions = message_actions.actions();
    let actions = message_actions
        .iter()
        .map(|(name, percent)| (name.as_str(), pie_colors(name), *percent))
        .collect::<Vec<_>>();

    let images = [
        // Rspamd action breakdown
        Quantity {
            name: format!("Breakdown of Rspamd Actions for {}", domain),
            domain: "Action".into(),
            range: "Percentage".into(),
            data: actions.as_slice(),
        }
        .make_pie(),
        // Histogram based on X-Spam-Result values
        Quantity {
            name: format!("X-Spam-Result Distribution for {}", domain),
            domain: "Spam Result".into(),
            range: "Occurrences".into(),
            data: <SpamResultsDistribution as From<&SpamResults>>::from(&spam_results),
        }
        .make_histogram(),
        // Histogram of spam classification performance
        Quantity {
            name: format!("Spam Misclassification Rate for {}", domain),
            domain: "Date".into(),
            range: "Percent".into(),
            data: <SpamRateDistribution as From<&SpamResults>>::from(&spam_results),
        }
        .make_histogram(),
        // Histogram of spam received per day
        Quantity {
            name: format!("Daily Received Spam for {}", domain),
            domain: "Date".into(),
            range: "Occurrences".into(),
            data: <TotalSpamDistribution as From<&SpamResults>>::from(&spam_results),
        }
        .make_histogram(),
    ]
    .into_iter()
    .collect::<Result<Vec<Image>, _>>()?;

    let template = MessageTemplate::new(domain.into(), "postmaster".into())?;
    let email = template.make_message(images.into_iter(), statistics.into_iter())?;

    // Create SMTP client for localhost:25
    let mailer = SmtpTransport::unencrypted_localhost();

    // Send the email
    match mailer.send(&email) {
        Ok(_) => println!("Email sent successfully."),
        Err(e) => eprintln!("Failed to send email: {e}"),
    }

    Ok(())
}

#[derive(clap::Parser)]
struct Args {
    /// The virtual mailbox base path
    #[clap(value_parser, short, long)]
    path: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let domain = get_hostname()?;
    spam_statistics(&domain, args.path)
}
