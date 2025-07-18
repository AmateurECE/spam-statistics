use clap::Parser;
use core::error::Error;
use email::MessageTemplate;
use lettre::{SmtpTransport, Transport};
use plot::{Color, Image, PieSlice, Quantity};
use rspamd::{load_rspamd_statistics, MessageActions, RspamdStatistics};
use spam::{load_spam_maildir, load_spam_virtual_mailbox_base};
use statistics::{dates_received, misclassification_rate, quantize_spam_results};
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

fn action_breakdown(
    MessageActions {
        no_action,
        greylist,
        add_header,
        reject,
    }: MessageActions,
) -> Vec<PieSlice> {
    let total: f64 = (no_action + greylist + add_header + reject) as f64;
    let make_label = |label, occurrences| {
        format!(
            "{} ({}, {:.1}%)",
            label,
            occurrences,
            ((occurrences as f64) / total) * 100.0
        )
    };
    vec![
        PieSlice {
            label: make_label("No Action", no_action),
            color: Color::Green,
            ratio: (no_action as f64) / total,
        },
        PieSlice {
            label: make_label("Greylist", greylist),
            color: Color::Blue,
            ratio: (greylist as f64) / total,
        },
        PieSlice {
            label: make_label("Mark as Spam", add_header),
            color: Color::Orange,
            ratio: (add_header as f64) / total,
        },
        PieSlice {
            label: make_label("Reject", reject),
            color: Color::Red,
            ratio: (reject as f64) / total,
        },
    ]
}

#[allow(dead_code)]
fn spam_statistics<P, Q>(
    domain: &str,
    virtual_mailbox_base: P,
    maildirs: &[Q],
) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let RspamdStatistics {
        statistics,
        message_actions,
    } = load_rspamd_statistics()?;
    let message_actions = action_breakdown(message_actions);

    // Rspamd action breakdown
    let rspamd_image = Quantity {
        name: format!("Breakdown of Rspamd Actions for {}", domain),
        domain: "Action".into(),
        range: "Percentage".into(),
        data: message_actions.as_slice(),
    }
    .make_pie();

    let mut spam_results = load_spam_virtual_mailbox_base(virtual_mailbox_base)?;
    for maildir in maildirs {
        if let Ok(results) = load_spam_maildir(maildir) {
            spam_results.extend(results);
        }
    }

    let spam_scores = spam_results
        .iter()
        .map(|email| (email.date_received, email.spam_result))
        .collect::<Vec<_>>();

    let images = if !spam_results.is_empty() {
        vec![
            // Histogram based on X-Spam-Result values
            Quantity {
                name: format!("X-Spam-Result Distribution for {}", domain),
                domain: "Spam Result".into(),
                range: "Occurrences".into(),
                data: quantize_spam_results(spam_results.iter()).as_slice(),
            }
            .make_histogram(),
            // Histogram of spam classification performance
            Quantity {
                name: format!("Spam Misclassification Rate for {}", domain),
                domain: "Date".into(),
                range: "Percent".into(),
                data: misclassification_rate(spam_results.iter()).as_slice(),
            }
            .make_linechart(),
            // Distribution of daily spam results
            Quantity {
                name: format!("Daily Spam Results for {}", domain),
                domain: "Date".into(),
                range: "X-Spam-Result".into(),
                data: spam_scores.as_slice(),
            }
            .make_boxplot(),
            // Histogram of spam received per day
            Quantity {
                name: format!("Daily Received Spam for {}", domain),
                domain: "Date".into(),
                range: "Occurrences".into(),
                data: dates_received(spam_results.iter()).as_slice(),
            }
            .make_histogram(),
        ]
    } else {
        println!("No spam.");
        vec![]
    };

    let template = MessageTemplate::new(domain.into(), "postmaster".into())?;
    let email = template.make_message(
        [rspamd_image].into_iter().chain(images.into_iter()),
        statistics.into_iter(),
    )?;

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

    /// Additional Maildir paths to parse through
    #[clap(value_parser, short, long)]
    maildirs: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let domain = get_hostname()?;
    spam_statistics(&domain, args.path, &args.maildirs)
}
