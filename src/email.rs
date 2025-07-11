use lettre::{
    address::AddressError,
    message::{header, Mailbox, MultiPart, SinglePart},
    Message,
};

use crate::plot::Image;

pub struct MessageTemplate {
    pub domain: String,
    pub recipient: Mailbox,
    pub sender: Mailbox,
}

impl MessageTemplate {
    pub fn new(domain: String, recipient_username: String) -> Result<Self, AddressError> {
        Ok(Self {
            recipient: format!("{}@{}", recipient_username, &domain).parse()?,
            sender: format!("spam-stats@{}", &domain).parse()?,
            domain,
        })
    }

    pub fn make_message<I, T>(self, images: I, text: T) -> Result<Message, lettre::error::Error>
    where
        I: Iterator<Item = Image>,
        T: Iterator<Item = String>,
    {
        let mut html_image_content = String::new();
        let mut parts = Vec::<SinglePart>::new();
        for (i, image) in images.enumerate() {
            let cid = format!("image{}", i);
            html_image_content += &format!(r#"<img src="cid:{}" alt="{}" />"#, cid, image.alt);
            let singlepart = SinglePart::builder()
                .header(header::ContentType::parse(mime::IMAGE_PNG.as_ref()).unwrap())
                .header(header::ContentDisposition::inline())
                .header(header::ContentId::from(format!("<{}>", cid)))
                .body(image.png);
            parts.push(singlepart);
        }

        let text_content = r#"<ul style="list-style-type:none;">"#.to_string()
            + &text
                .map(|line| format!("<li>{}</li>", &line))
                .collect::<Vec<_>>()
                .join("\n")
            + "</ul>";

        let html_body = format!(
            r#"
        <html>
        <body>
            <p>Here are the spam statistics for {}.</p>
            {}
            {}
        </body>
        </html>
        "#,
            self.domain, html_image_content, text_content
        );

        let message = SinglePart::builder()
            .header(header::ContentType::TEXT_HTML)
            .body(html_body);
        let mut multipart = MultiPart::related().singlepart(message);
        for part in parts {
            multipart = multipart.singlepart(part);
        }

        Message::builder()
            .from(self.sender)
            .to(self.recipient)
            .subject("Spam Statistics")
            .multipart(multipart)
    }
}
