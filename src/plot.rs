use chrono::NaiveDate;
use plotters::{
    backend::{PixelFormat, RGBPixel},
    style::{FontDesc, IntoFont},
};
use std::{cell::LazyCell, collections::HashMap, io::Cursor};

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

const fn buffer_size() -> usize {
    let (width, height) = IMAGE_SIZE;
    let width: usize = width as usize;
    let height: usize = height as usize;
    width * height * RGBPixel::PIXEL_SIZE
}

pub trait ChartRange {
    type Value;
    fn chart_range<I>(iter: I) -> Option<(Self::Value, Self::Value)>
    where
        I: Iterator<Item = Self::Value> + Clone;
}

impl ChartRange for NaiveDate {
    type Value = NaiveDate;
    fn chart_range<I>(iter: I) -> Option<(Self::Value, Self::Value)>
    where
        I: Iterator<Item = Self::Value> + Clone,
    {
        let min = iter.clone().min()?;
        let max = iter.max()?;
        Some((min, max))
    }
}

impl ChartRange for f64 {
    type Value = f64;
    fn chart_range<I>(iter: I) -> Option<(Self::Value, Self::Value)>
    where
        I: Iterator<Item = Self::Value> + Clone,
    {
        let min = iter.clone().fold(f64::INFINITY, |a, b| a.min(b));
        let max = iter.clone().fold(f64::MIN, |a, b| a.max(b));
        Some((min, max))
    }
}

fn histogram_y_range<X>(data: &[X]) -> usize
where
    X: core::cmp::Eq + core::hash::Hash + Copy,
{
    let mut counts: HashMap<X, usize> = HashMap::new();
    for x in data {
        *counts.entry(*x).or_insert(0) += 1;
    }
    *counts.values().max().unwrap()
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
