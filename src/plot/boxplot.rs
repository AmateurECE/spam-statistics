use std::collections::HashSet;

use crate::statistics::SpamResult;

use super::{buffer_size, into_png, Image, Quantity, FONT, IMAGE_SIZE};
use chrono::NaiveDate;
use plotters::{data::fitting_range, prelude::*};

// TODO: Make X and Y generic here
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
