use core::{fmt, slice};
use full_palette::{ORANGE, PURPLE};
use plotters::{prelude::*, style::full_palette::INDIGO};
use plotters_svg::SVGBackend;
use roxmltree::Document;
use std::{
    io::Write,
    process::{Command, Stdio},
};
use xmlwriter::{Indent, Options, XmlWriter};

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

fn remove_width_height(svg_input: String) -> String {
    let doc = Document::parse(&svg_input).expect("Failed to parse SVG");
    let root = doc.root_element();

    let mut options = Options::default();
    options.indent = Indent::None;
    let mut writer = XmlWriter::new(options);

    fn write_node(node: roxmltree::Node, writer: &mut XmlWriter) {
        match node.node_type() {
            roxmltree::NodeType::Element => {
                writer.start_element(node.tag_name().name());

                // Copy attributes, excluding width and height for the <svg> element
                for attr in node.attributes() {
                    if node.tag_name().name() == "svg"
                        && (attr.name() == "width" || attr.name() == "height")
                    {
                        continue;
                    }
                    writer.write_attribute(attr.name(), attr.value());
                }

                // Have to render XML namespaces into the output, or else some mail clients
                // (Thunderbird) don't want to render it.
                for attr in node.namespaces() {
                    if let Some(name) = attr.name() {
                        let name = "xmlns:".to_string() + name;
                        writer.write_attribute(&name, attr.uri());
                    } else {
                        writer.write_attribute("xmlns", attr.uri());
                    }
                }

                for child in node.children() {
                    write_node(child, writer);
                }

                writer.end_element();
            }
            roxmltree::NodeType::Text => {
                writer.write_text(node.text().unwrap());
            }
            _ => {}
        }
    }

    write_node(root, &mut writer);
    writer.end_document()
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
            pie.label_style(("Roboto", 16).into_font());
            drawing_area.draw(&pie).expect("Couldn't draw pie chart");

            drawing_area
                .present()
                .expect("Couldn't finalize pie chart graphic");
        }

        let svg = remove_width_height(svg);
        Ok(Image {
            svg,
            alt: self.name,
        })
    }
}
