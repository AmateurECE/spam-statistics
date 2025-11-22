use plotters::{
    prelude::*,
    style::{
        full_palette::{INDIGO, ORANGE, PURPLE},
        RGBColor, BLUE, GREEN, RED, YELLOW,
    },
};

use super::{buffer_size, into_png, Image, Quantity, FONT, IMAGE_SIZE};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[allow(dead_code)]
pub enum Color {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Indigo,
    Violet,
}

pub struct Slice {
    pub label: String,
    pub color: Color,
    pub ratio: f64,
}

impl From<Color> for RGBColor {
    fn from(value: Color) -> RGBColor {
        match value {
            Color::Red => RED,
            Color::Orange => ORANGE,
            Color::Yellow => YELLOW,
            Color::Green => GREEN,
            Color::Blue => BLUE,
            Color::Indigo => INDIGO,
            Color::Violet => PURPLE,
        }
    }
}

impl Quantity<&[Slice]> {
    pub fn make_pie(self) -> Image {
        let font = FONT.with(|f| (*f).clone());
        let mut bitmap = vec![0; buffer_size()];
        {
            let drawing_area =
                BitMapBackend::with_buffer(&mut bitmap, IMAGE_SIZE).into_drawing_area();
            drawing_area.fill(&WHITE).expect("Couldn't fill background");

            let center = (300, 200);
            let radius = 100.0;

            let data = self.data.iter().filter(|slice| slice.ratio != 0.0);

            let sizes = data.clone().map(|slice| slice.ratio).collect::<Vec<_>>();
            let colors = data
                .clone()
                .map(|slice| slice.color.into())
                .collect::<Vec<_>>();
            let labels = data.clone().map(|slice| &slice.label).collect::<Vec<_>>();

            let mut pie = Pie::new(&center, &radius, &sizes, &colors, &labels);
            pie.label_style(font.clone());
            drawing_area
                .titled(&self.name, font)
                .expect("Couldn't apply title to chart")
                .draw(&pie)
                .expect("Couldn't draw pie chart");

            drawing_area
                .present()
                .expect("Couldn't finalize pie chart graphic");
        }

        Image {
            png: into_png(bitmap),
            alt: self.name,
        }
    }
}
