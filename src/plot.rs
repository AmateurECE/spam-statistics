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
"#;

pub struct Image {
    pub svg: String,
    pub alt: String,
}

#[derive(Clone, Debug)]
struct ImageError(String);
impl std::error::Error for ImageError {}
impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
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
        X: Ord + fmt::Display,
    {
        let data = self.data.into_iter();
        let domain = data.clone().map(|(x, _)| x);
        let no_data = ImageError("No data!".into());
        let min = domain.clone().min().ok_or_else(|| no_data.clone())?;
        let max = domain.max().ok_or_else(|| no_data)?;

        let mut script = GNUPLOT_SCRIPT.to_string();
        script += &format!("set yrange [ {} : {} ]\n", min, max);
        script += &format!("set ylabel \"{}\"", self.range);
        script += &format!("set xlabel \"{}\"", self.domain);
        script += &format!("set title \"{}\"\n", self.name);
        script += &format!("plot '-' using 2:xtic(1) title '{}'\n", self.name);

        let mut gnuplot = Command::new("gnuplot")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let mut stdin = gnuplot
            .stdin
            .take()
            .ok_or(ImageError("Failed to open gnuplot stdin".into()))?;

        stdin.write_all(script.as_bytes())?;
        let output = gnuplot.wait_with_output()?;
        if !output.status.success() {
            return Err(ImageError(String::from_utf8(output.stderr)?).into());
        }

        Ok(Image {
            svg: String::from_utf8(output.stdout)?,
            alt: self.name,
        })
    }
}
