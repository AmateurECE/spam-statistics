// Actions to determine spam statistics for a single user, user@domain.com:
// 1. Read config file
// 2. ls /var/spool/vmail/
// 3. ls /var/spool/vmail/domain.com
// 4. stat /var/spool/vmail/domain.com/user/.Spam
// 5. stat /var/spool/vmail/domain.com/user/.Spam/{cur,new}
// 6. ls /var/spool/vmail/domain.com/user/.Spam/{cur,new}
// 7. cat /var/spool/vmail/domain.com/user/.Spam/{cur,new}/*
// 8. Format into SVG image
// 9. Send email

// See maildir(5)

use lettre::address::AddressError;
use lettre::message::{Mailbox, Message, MultiPart, SinglePart, header};
use lettre::{SmtpTransport, Transport};
use mime::IMAGE_SVG;
use std::error::Error;

struct MessageTemplate {
    domain: String,
    recipient: Mailbox,
    sender: Mailbox,
}

impl MessageTemplate {
    pub fn new(domain: String, recipient_username: String) -> Result<Self, AddressError> {
        Ok(Self {
            recipient: format!("{}@{}", recipient_username, &domain).parse()?,
            sender: format!("spam-stats@{}", &domain).parse()?,
            domain,
        })
    }

    fn make_message(self, image: String) -> Result<Message, lettre::error::Error> {
        let cid = "image1"; // Content-ID for embedding

        let html_body = format!(
            r#"
        <html>
        <body>
            <p>Here are the spam statistics for {}.</p>
            <img src="cid:{}" alt="SVG image" />
        </body>
        </html>
        "#,
            self.domain, cid
        );

        Message::builder()
            .from(self.sender)
            .to(self.recipient)
            .subject("Spam Statistics")
            .multipart(
                MultiPart::related() // "multipart/related" to embed inline image
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_HTML)
                            .body(html_body),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::parse(&IMAGE_SVG.to_string()).unwrap())
                            .header(header::ContentDisposition::inline())
                            .header(header::ContentId::from(format!("<{}>", cid)))
                            .body(image),
                    ),
            )
    }
}

#[allow(dead_code)]
fn spam_statistics() -> Result<(), Box<dyn Error>> {
    // Your SVG image as a string (inline image)
    let svg_image = r#"
    <svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">
        <circle cx="100" cy="100" r="80" stroke="green" stroke-width="4" fill="yellow" />
    </svg>
    "#;

    let template = MessageTemplate::new("ethantwardy.com".into(), "et".into())?;
    let email = template.make_message(svg_image.into())?;

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
