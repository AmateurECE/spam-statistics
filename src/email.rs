use lettre::{
    Message,
    address::AddressError,
    message::{Mailbox, MultiPart, SinglePart, header},
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

    pub fn make_message<I>(self, images: I) -> Result<Message, lettre::error::Error>
    where
        I: Iterator<Item = Image>,
    {
        let mut multipart: Option<MultiPart> = None;
        let mut html_image_content = String::new();
        for (i, image) in images.enumerate() {
            let cid = format!("image{}", i);
            html_image_content += &format!(r#"<img src="cid:{}" alt="{}" />"#, cid, image.alt);
            let singlepart = SinglePart::builder()
                .header(header::ContentType::parse(&mime::IMAGE_SVG.to_string()).unwrap())
                .header(header::ContentDisposition::inline())
                .header(header::ContentId::from(format!("<{}>", cid)))
                .body(image.svg);
            multipart = match multipart {
                Some(m) => Some(m.singlepart(singlepart)),
                None => Some(MultiPart::related().singlepart(singlepart)),
            };
        }

        let html_body = format!(
            r#"
        <html>
        <body>
            <p>Here are the spam statistics for {}.</p>
            {}
        </body>
        </html>
        "#,
            self.domain, html_image_content
        );

        let message = SinglePart::builder()
            .header(header::ContentType::TEXT_HTML)
            .body(html_body);
        let multipart = match multipart {
            Some(m) => m.singlepart(message),
            None => MultiPart::related().singlepart(message),
        };

        Message::builder()
            .from(self.sender)
            .to(self.recipient)
            .subject("Spam Statistics")
            .multipart(multipart)
    }
}
