use core::fmt;
use plotters::{
    coord::{
        ranged1d::{AsRangedCoord, SegmentedCoord, ValueFormatter},
        types::RangedSlice,
    },
    prelude::*,
    style::full_palette::PURPLE,
};

use super::{buffer_size, histogram_y_range, into_png, Image, Quantity, FONT, IMAGE_SIZE};

// TODO: Implement this for (X, Y) as well
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
