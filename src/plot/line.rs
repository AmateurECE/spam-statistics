use core::fmt;

use super::{buffer_size, into_png, ChartRange, Image, Quantity, FONT, IMAGE_SIZE};
use plotters::{
    coord::ranged1d::{AsRangedCoord, DefaultFormatting, ValueFormatter},
    prelude::*,
    style::full_palette::PURPLE,
};

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
