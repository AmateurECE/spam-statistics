use core::{fmt, slice};
use full_palette::{ORANGE, PURPLE};
use plotters::{prelude::*, style::full_palette::INDIGO};
use plotters_svg::SVGBackend;
use std::{
    io::Write,
    process::{Command, Stdio},
};

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

static GNUPLOT_SCRIPT: &str = r#"
set terminal svg size 600,400 dynamic enhanced fname 'Arial'
set output

set style data histogram
set style fill solid border -1
set boxwidth 0.5
set xtics rotate by -45
unset key
"#;

pub struct Image {
    pub svg: String,
    pub alt: String,
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum ImageError {
    #[error("failed to communicate with gnuplot")]
    Pipe,
    #[error("gnuplot")]
    Gnuplot(String),
}

pub struct Quantity<D> {
    pub name: String,
    pub domain: String,
    pub range: String,
    pub data: D,
}

impl<D> Quantity<D> {
    pub fn make_histogram<X, Y>(self) -> Result<Image, ImageError>
    where
        for<'a> &'a D: IntoIterator<IntoIter = slice::Iter<'a, (X, Y)>>,
        X: fmt::Display,
        Y: fmt::Display,
    {
        let data = self.data.into_iter();

        let mut script = GNUPLOT_SCRIPT.to_string();
        script += &format!("set ylabel \"{}\"\n", self.range);
        script += &format!("set xlabel \"{}\"\n", self.domain);
        script += &format!("set title \"{}\"\n", self.name);
        script += &format!("plot '-' using 2:xtic(1) title '{}'\n", self.name);
        script += &data
            // TODO: Kind of a hack to wrap the x value in double quotes. Can we pull this out into
            // a trait instead?
            .map(|(x, y)| format!("\"{}\" {}", x, y))
            .collect::<Vec<_>>()
            .join("\n");
        script += "e\n";

        let mut gnuplot = Command::new("gnuplot")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| ImageError::Gnuplot(e.to_string()))?;

        {
            let mut stdin = gnuplot.stdin.take().ok_or(ImageError::Pipe)?;
            stdin
                .write_all(script.as_bytes())
                .map_err(|_| ImageError::Pipe)?;
        }

        let output = gnuplot
            .wait_with_output()
            .map_err(|e| ImageError::Gnuplot(e.to_string()))?;
        if !output.status.success() {
            let error = ImageError::Gnuplot(
                String::from_utf8(output.stderr).map_err(|_| ImageError::Pipe)?,
            );
            return Err(error);
        }

        Ok(Image {
            svg: String::from_utf8(output.stdout).map_err(|_| ImageError::Pipe)?,
            alt: self.name,
        })
    }
}

impl Quantity<&[(&str, Color, f64)]> {
    pub fn make_pie(self) -> Result<Image, ImageError> {
        let mut svg = String::new();
        {
            let drawing_area = SVGBackend::with_string(&mut svg, (600, 400)).into_drawing_area();
            drawing_area.fill(&WHITE).expect("Couldn't fill background");

            let center = (300, 200);
            let radius = 175.0;

            let data = self.data.iter().filter(|(_, _, amount)| *amount != 0.0);

            let sizes = data.clone().map(|(_, _, size)| *size).collect::<Vec<_>>();
            let colors = data
                .clone()
                .map(|(_, color, _)| (*color).into())
                .collect::<Vec<_>>();
            let labels = data.clone().map(|(label, _, _)| label).collect::<Vec<_>>();

            let mut pie = Pie::new(&center, &radius, &sizes, &colors, &labels);
            pie.label_style(("Roboto", 20).into_font());
            drawing_area.draw(&pie).expect("Couldn't draw pie chart");

            drawing_area
                .present()
                .expect("Couldn't finalize pie chart graphic");
        }
        Ok(Image {
            svg,
            alt: self.name,
        })
    }
}
