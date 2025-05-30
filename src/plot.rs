use core::{fmt, slice};
use std::{
    io::Write,
    process::{Command, Stdio},
};

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
enum ImageError {
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
    pub fn make_histogram<X, Y>(self) -> Result<Image, anyhow::Error>
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
            .spawn()?;

        {
            let mut stdin = gnuplot.stdin.take().ok_or(ImageError::Pipe)?;
            stdin.write_all(script.as_bytes())?;
        }

        let output = gnuplot.wait_with_output()?;
        if !output.status.success() {
            return Err(ImageError::Gnuplot(String::from_utf8(output.stderr)?).into());
        }

        Ok(Image {
            svg: String::from_utf8(output.stdout)?,
            alt: self.name,
        })
    }
}
