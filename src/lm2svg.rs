use clap::Parser;
use image::GenericImageView;
use std::path::Path;
use svg::node::element::{self, Circle};

/// Load and print labelme annotations
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Input json filename
    input: String,
    /// Output svg filename
    output: Option<String>,
    /// Config filename
    #[clap(short, long)]
    config: Option<String>,
    /// Circle radius
    #[clap(short, long, default_value = "2")]
    radius: usize,
}
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

//pub type Color = (u8, u8, u8);
pub type LabelColors = std::collections::HashMap<String, Color>;

static COLORS: [&str; 6] = ["red", "green", "blue", "cyan", "magenta", "yellow"];
struct ColorCycler {
    i: usize,
}
impl ColorCycler {
    pub fn cycle(&mut self) -> &str {
        let c = COLORS[self.i];
        self.i = (self.i + 1) % COLORS.len();
        c
    }
    pub fn new() -> Self {
        ColorCycler { i: 0 }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let json_data = labelme_rs::LabelMeData::load(&args.input)?;
    let label_colors = match args.config {
        Some(config) => {
            let config: serde_yaml::Value =
                serde_yaml::from_reader(std::io::BufReader::new(std::fs::File::open(config)?))?;
            let colors = config.get("label_colors").unwrap();
            serde_yaml::from_value(colors.to_owned())?
        }
        None => LabelColors::new(),
    };
    let mut color_cycler = ColorCycler::new();

    let img_filename = Path::new(&args.input)
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(&json_data.imagePath);
    let img = image::open(&img_filename)?;
    let (image_width, image_height) = img.dimensions();

    let b64 = labelme_rs::img2base64(&img, image::ImageOutputFormat::Jpeg(75));
    let mut document = svg::Document::new()
        .set("width", image_width)
        .set("height", image_height)
        .set("viewBox", (0i64, 0i64, image_width, image_height))
        .set("xmlns:xlink", "http://www.w3.org/1999/xlink");

    let b64 = format!("data:image/jpeg;base64,{}", b64);
    let bg = element::Image::new()
        .set("x", 0i64)
        .set("y", 0i64)
        .set("width", image_width)
        .set("height", image_height)
        .set("xlink:href", b64);
    document = document.add(bg);
    let shape_map = json_data.to_shape_map();
    let point_data = shape_map.get("point").unwrap();
    for (label, points) in point_data {
        let color = label_colors
            .get(label)
            .map(|rgb| rgb.to_hex())
            .unwrap_or_else(|| color_cycler.cycle().into());
        let mut group = element::Group::new()
            .set("class", label.clone())
            .set("fill", color)
            .set("stroke", "none");

        for point in points {
            let point_xy = point[0];
            let circle = Circle::new()
                .set("cx", point_xy.0)
                .set("cy", point_xy.1)
                .set("r", args.radius);
            group = group.add(circle);
        }
        document = document.add(group);
    }
    if let Some(filename) = args.output {
        svg::save(filename, &document)?;
    }
    Ok(())
}
