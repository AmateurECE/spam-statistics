use chrono::NaiveDate;
use core::fmt;
use full_palette::{ORANGE, PURPLE};
use plotters::{
    backend::{PixelFormat, RGBPixel},
    coord::{
        ranged1d::{AsRangedCoord, DefaultFormatting, SegmentedCoord, ValueFormatter},
        types::RangedSlice,
    },
    data::fitting_range,
    prelude::*,
    style::{full_palette::INDIGO, Color as _},
};
use std::{
    cell::LazyCell,
    collections::{HashMap, HashSet},
    io::Cursor,
};

use crate::statistics::SpamResult;

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

pub struct Image {
    pub png: Vec<u8>,
    pub alt: String,
}

pub struct PieSlice {
    pub label: String,
    pub color: Color,
    pub ratio: f64,
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

impl<X, R> Quantity<&[X]>
where
    X: fmt::Display + Copy + Clone + core::fmt::Debug + PartialEq + Ord + core::hash::Hash,
    for<'a> &'a [X]: AsRangedCoord<CoordDescType = RangedSlice<'a, X>, Value = &'a X>,
    std::ops::Range<X>: AsRangedCoord<CoordDescType = R, Value = X>,
    R: Ranged<ValueType = X> + DiscreteRanged + Clone,
    SegmentedCoord<R>: ValueFormatter<SegmentValue<<R as Ranged>::ValueType>>,
{
    pub fn make_histogram(self) -> Image {
        let mut bitmap = vec![0; buffer_size()];
        let (min, max) = (
            *self.data.iter().min().unwrap(),
            *self.data.iter().max().unwrap(),
        );
        let font = FONT.with(|f| (*f).clone());
        let y_max = histogram_y_range(self.data);
        {
            let drawing_area =
                BitMapBackend::with_buffer(&mut bitmap, IMAGE_SIZE).into_drawing_area();
            drawing_area
                .fill(&WHITE)
                .expect("couldn't fill chart background");
            let mut chart_builder = ChartBuilder::on(&drawing_area);
            let mut chart_context = chart_builder
                .margin(5)
                .caption(&self.name, font.clone())
                .set_left_and_bottom_label_area_size(40)
                .build_cartesian_2d((min..max).into_segmented(), 0..y_max)
                .expect("couldn't build cartesian space");
            chart_context
                .configure_mesh()
                .x_desc(self.domain)
                .y_desc(self.range)
                .axis_desc_style(font)
                .draw()
                .expect("couldn't draw axes");
            chart_context
                .draw_series(
                    Histogram::vertical(&chart_context)
                        .style(PURPLE.filled())
                        .data(self.data.iter().map(|x| (*x, 1))),
                )
                .expect("couldn't draw histogram series");

            drawing_area
                .present()
                .expect("couldn't finalize pie chart graphic");
        }

        Image {
            png: into_png(bitmap),
            alt: self.name,
        }
    }
}

impl<X, Y, R, S> Quantity<&[(X, Y)]>
where
    X: ChartRange<Value = X> + fmt::Display + Copy + Clone + core::fmt::Debug + PartialEq + 'static,
    Y: ChartRange<Value = Y> + fmt::Display + Copy + Clone + core::fmt::Debug + PartialEq + 'static,
    std::ops::Range<X>: AsRangedCoord<CoordDescType = R, Value = X>,
    R: Ranged<FormatOption = DefaultFormatting, ValueType = X> + DiscreteRanged + Clone,
    std::ops::Range<Y>: AsRangedCoord<CoordDescType = S, Value = Y>,
    S: Ranged<ValueType = Y> + ValueFormatter<Y> + Clone,
{
    pub fn make_linechart(self) -> Image {
        let mut bitmap = vec![0; buffer_size()];
        let domain = self.data.iter().map(|(x, _)| *x);
        let (x_min, x_max) = X::chart_range(domain).unwrap();
        let range = self.data.iter().map(|(_, y)| *y);
        let (y_min, y_max) = Y::chart_range(range).unwrap();
        let font = FONT.with(|f| (*f).clone());
        {
            let drawing_area =
                BitMapBackend::with_buffer(&mut bitmap, IMAGE_SIZE).into_drawing_area();
            drawing_area
                .fill(&WHITE)
                .expect("couldn't fill chart background");
            let mut chart_builder = ChartBuilder::on(&drawing_area);
            let mut chart_context = chart_builder
                .margin(5)
                .caption(&self.name, font.clone())
                .set_left_and_bottom_label_area_size(40)
                .build_cartesian_2d(x_min..x_max, y_min..y_max)
                .expect("couldn't build cartesian space");
            chart_context
                .configure_mesh()
                .x_desc(self.domain)
                .y_desc(self.range)
                .axis_desc_style(font)
                .draw()
                .expect("couldn't draw axes");

            chart_context
                .draw_series(LineSeries::new(self.data.iter().cloned(), PURPLE))
                .expect("couldn't draw histogram series");
            chart_context
                .draw_series(
                    self.data
                        .iter()
                        .map(|(x, y)| Circle::new((*x, *y), 3, PURPLE.filled())),
                )
                .expect("couldn't draw histogram series");

            drawing_area
                .present()
                .expect("couldn't finalize pie chart graphic");
        }

        Image {
            png: into_png(bitmap),
            alt: self.name,
        }
    }
}

impl Quantity<&[(NaiveDate, SpamResult)]> {
    pub fn make_boxplot(self) -> Image {
        let mut dates = self
            .data
            .iter()
            .map(|(date, _)| date)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        dates.sort();
        let font = FONT.with(|f| (*f).clone());
        let mut bitmap = vec![0u8; buffer_size()];
        {
            let drawing_area =
                BitMapBackend::with_buffer(&mut bitmap, IMAGE_SIZE).into_drawing_area();
            drawing_area.fill(&WHITE).expect("couldn't fill background");

            let values_range = fitting_range(self.data.iter().map(|(_, result)| result));
            let x_spec = dates.to_vec();
            let (start, end) = (values_range.start as f32, values_range.end as f32);
            let mut chart = ChartBuilder::on(&drawing_area)
                .x_label_area_size(40)
                .y_label_area_size(40)
                .caption(&self.name, font.clone())
                .build_cartesian_2d(
                    x_spec.into_segmented(),
                    (start - start * 0.05)..(end + end * 0.05),
                )
                .expect("couldn't draw chart");
            chart.configure_mesh().draw().expect("couldn't draw mesh");

            chart
                .draw_series(dates.iter().map(|date| {
                    let series = Quartiles::new(
                        &self
                            .data
                            .iter()
                            .filter(|(received, _)| *received == **date)
                            .map(|(_, result)| *result)
                            .collect::<Vec<_>>(),
                    );
                    Boxplot::new_vertical(SegmentValue::CenterOf(date), &series)
                }))
                .expect("couldn't draw series");

            drawing_area.present().expect("couldn't finalize boxplot");
        }

        Image {
            png: into_png(bitmap),
            alt: self.name,
        }
    }
}

impl Quantity<&[PieSlice]> {
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
