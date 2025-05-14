// Actions to determine spam statistics for a single user, user@domain.com:
// 1. Read config file
// 2. ls /var/spool/vmail/
// 3. ls /var/spool/vmail/domain.com
// 4. stat /var/spool/vmail/domain.com/user/.Spam
// 5. stat /var/spool/vmail/domain.com/user/.Spam/{cur,new}
// 6. ls /var/spool/vmail/domain.com/user/.Spam/{cur,new}
// 7. cat /var/spool/vmail/domain.com/user/.Spam/{cur,new}/*
// 8. Send email

// See maildir(5)

use core::error::Error;
use email::MessageTemplate;
use lettre::{SmtpTransport, Transport};
use plot::{Image, Quantity};
use statistics::{
    MissRateDistribution, SpamRateDistribution, SpamResults, SpamResultsDistribution,
};

mod email;
mod plot;
mod statistics;

#[allow(dead_code)]
fn spam_statistics() -> Result<(), Box<dyn Error>> {
    let spam_results: SpamResults = Vec::new();
    let domain = "ethantwardy.com";
    let images = [
        // 1. Histogram based on X-Spam-Result values
        Quantity {
            name: format!("X-Spam-Result Distribution for {}", domain),
            domain: "Spam Result".into(),
            range: "Occurrences".into(),
            data: <SpamResultsDistribution as From<&SpamResults>>::from(&spam_results),
        }
        .make_histogram(),
        // 2. Histogram of Spam received per day
        Quantity {
            name: format!("Daily Received Spam for {}", domain),
            domain: "Date".into(),
            range: "Occurrences".into(),
            data: <SpamRateDistribution as From<&SpamResults>>::from(&spam_results),
        }
        .make_histogram(),
        // 3. Histogram of Spam received not marked "X-Spam":"Yes" per day
        Quantity {
            name: format!("Daily Misclassified Spam for {}", domain),
            domain: "Date".into(),
            range: "Occurrences".into(),
            data: <MissRateDistribution as From<&SpamResults>>::from(&spam_results),
        }
        .make_histogram(),
    ]
    .into_iter()
    .collect::<Result<Vec<Image>, _>>()?;

    let template = MessageTemplate::new(domain.into(), "et".into())?;
    let email = template.make_message(images.into_iter())?;

    // Create SMTP client for localhost:25
    let mailer = SmtpTransport::unencrypted_localhost();

    // Send the email
    match mailer.send(&email) {
        Ok(_) => println!("Email sent successfully."),
        Err(e) => eprintln!("Failed to send email: {e}"),
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}
