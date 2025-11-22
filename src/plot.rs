use plotters::{
    backend::{PixelFormat, RGBPixel},
    style::{FontDesc, IntoFont},
};
use std::{cell::LazyCell, io::Cursor};

pub mod boxplot;
pub mod hist;
pub mod line;
pub mod pie;

pub struct Image {
    pub png: Vec<u8>,
    pub alt: String,
}

pub struct Quantity<D> {
    pub name: String,
    pub domain: String,
    pub range: String,
    pub data: D,
}

thread_local! {
static FONT: LazyCell<FontDesc<'static>> = LazyCell::new(|| ("Roboto", 16).into_font());
}
const IMAGE_SIZE: (u32, u32) = (600, 400);

//
// Miscellaneous
//

const fn buffer_size() -> usize {
    let (width, height) = IMAGE_SIZE;
    let width: usize = width as usize;
    let height: usize = height as usize;
    width * height * RGBPixel::PIXEL_SIZE
}

fn into_png(bitmap: Vec<u8>) -> Vec<u8> {
    let mut png = Vec::<u8>::new();
    {
        let cursor = Cursor::new(&mut png);
        let (width, height) = IMAGE_SIZE;
        let mut encoder = png::Encoder::new(cursor, width, height);
        encoder.set_color(png::ColorType::Rgb);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&bitmap).unwrap();
    }
    png
}

//
// CartesianRange
//

struct LinearRange<X> {
    min: X,
    max: X,
}

struct CartesianRange<X, Y> {
    x: LinearRange<X>,
    y: LinearRange<Y>,
}

trait TryIntoCartesianRange {
    type X;
    type Y;
    fn try_into_cartesian_range(self) -> Option<CartesianRange<Self::X, Self::Y>>;
}

impl<X, Y, I> TryIntoCartesianRange for I
where
    I: Iterator<Item = (X, Y)>,
    X: PartialOrd + Copy,
    Y: PartialOrd + Copy,
{
    type X = X;
    type Y = Y;
    fn try_into_cartesian_range(mut self) -> Option<CartesianRange<Self::X, Self::Y>> {
        let (x, y) = self.next()?;
        let init = CartesianRange {
            x: LinearRange { min: x, max: x },
            y: LinearRange { min: y, max: y },
        };
        Some(self.fold(
            init,
            |CartesianRange {
                 x:
                     LinearRange {
                         min: x_min,
                         max: x_max,
                     },
                 y:
                     LinearRange {
                         min: y_min,
                         max: y_max,
                     },
             },
             (x, y)| CartesianRange {
                x: LinearRange {
                    min: if x.lt(&x_min) { x } else { x_min },
                    max: if x.gt(&x_max) { x } else { x_max },
                },
                y: LinearRange {
                    min: if y.lt(&y_min) { y } else { y_min },
                    max: if y.gt(&y_max) { y } else { y_max },
                },
            },
        ))
    }
}
