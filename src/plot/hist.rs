use core::fmt;
use plotters::{
    coord::{
        ranged1d::{AsRangedCoord, SegmentedCoord, ValueFormatter},
        types::RangedSlice,
    },
    prelude::*,
    style::full_palette::PURPLE,
};

use super::{
    buffer_size, into_png, CartesianRange, Image, LinearRange, Quantity, TryIntoCartesianRange,
    FONT, IMAGE_SIZE,
};

// TODO: Implement this for (X, Y) as well
impl<X, I, R> Quantity<I>
where
    I: Iterator<Item = (X, usize)> + Clone,
    X: fmt::Display + Copy + Clone + core::fmt::Debug + PartialEq + PartialOrd + core::hash::Hash,
    for<'a> &'a [X]: AsRangedCoord<CoordDescType = RangedSlice<'a, X>, Value = &'a X>,
    std::ops::Range<X>: AsRangedCoord<CoordDescType = R, Value = X>,
    R: Ranged<ValueType = X> + DiscreteRanged + Clone,
    SegmentedCoord<R>: ValueFormatter<SegmentValue<<R as Ranged>::ValueType>>,
{
    pub fn make_histogram(self) -> Image {
        let mut bitmap = vec![0; buffer_size()];
        let CartesianRange {
            x: LinearRange {
                min: x_min,
                max: x_max,
            },
            y: LinearRange { max: y_max, .. },
        } = self.data.clone().try_into_cartesian_range().unwrap();
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
                .build_cartesian_2d((x_min..x_max).into_segmented(), 0..y_max)
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
                        .data(self.data),
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
