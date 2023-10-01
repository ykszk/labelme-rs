use base64::Engine;
pub use image;
use image::GenericImageView;
pub use indexmap;
use indexmap::{IndexMap, IndexSet};
pub use serde;
use serde::{Deserialize, Serialize};
pub use serde_json;
use std::error::Error;
use std::io::Cursor;
use std::path::Path;
pub use svg;
use svg::node::element;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::JpegDecoder;

pub type Flags = IndexMap<String, bool>;
pub type FlagSet = IndexSet<String>;
pub type Point = (f32, f32);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Shape {
    pub label: String,
    pub points: Vec<Point>,
    pub group_id: Option<String>,
    pub shape_type: String,
    pub flags: Flags,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct LabelMeData {
    pub version: String,
    pub flags: Flags,
    pub shapes: Vec<Shape>,
    pub imagePath: String,
    pub imageData: Option<String>,
    pub imageHeight: usize,
    pub imageWidth: usize,
}

pub fn img2base64(img: &image::DynamicImage, format: image::ImageOutputFormat) -> String {
    let mut cursor = Cursor::new(Vec::new());
    img.write_to(&mut cursor, format).unwrap();
    base64::engine::general_purpose::STANDARD.encode(cursor.into_inner())
}

impl LabelMeData {
    pub fn new(
        points: &[Point],
        labels: &[String],
        width: usize,
        height: usize,
        path: &str,
    ) -> Self {
        let shapes: Vec<Shape> = points
            .iter()
            .zip(labels)
            .map(|(p, l)| Shape {
                label: l.into(),
                points: vec![*p],
                group_id: None,
                shape_type: "point".into(),
                flags: Flags::new(),
            })
            .collect();
        Self {
            version: "4.5.7".into(),
            flags: Flags::new(),
            shapes,
            imagePath: path.into(),
            imageData: None,
            imageHeight: height,
            imageWidth: width,
        }
    }

    /// Convert to a shape_type-centered map with a structure map\[`shape_type`\]\[label\] -> points
    pub fn to_shape_map(&self) -> IndexMap<&str, IndexMap<&str, Vec<&Vec<Point>>>> {
        let mut map = IndexMap::new();
        for shape in &self.shapes {
            let m = map
                .entry(shape.shape_type.as_str())
                .or_insert_with(IndexMap::new);
            let v = m.entry(shape.label.as_str()).or_insert_with(Vec::new);
            v.push(&shape.points);
        }
        map
    }

    /// Scale points
    pub fn scale(&mut self, scale: f64) {
        for shape in &mut self.shapes {
            for p in &mut shape.points {
                p.0 = (scale * p.0 as f64) as f32;
                p.1 = (scale * p.1 as f64) as f32;
            }
        }
        self.imageWidth = (self.imageWidth as f64 * scale) as _;
        self.imageHeight = (self.imageHeight as f64 * scale) as _;
    }

    /// Count the number of labels
    ///
    /// ```
    /// let data = labelme_rs::LabelMeData::new(&[(1.0, 1.0), (2.0, 2.0), (3.0, 3.0)], &["L1".into(), "L2".into(), "L2".into()], 128, 128, "image.jpg");
    /// let counts = data.count_labels();
    /// assert_eq!(*counts.get("L1").unwrap(), 1usize);
    /// assert_eq!(*counts.get("L2").unwrap(), 2usize);
    /// assert_eq!(counts.get("L0").cloned().unwrap_or(0usize), 0usize);
    /// ```
    pub fn count_labels(&self) -> IndexMap<&str, usize> {
        let mut counts: IndexMap<&str, usize> = IndexMap::new();
        let mut shape_map = self.to_shape_map();
        if let Some(point_data) = shape_map.remove("point") {
            for (label, points) in point_data {
                counts
                    .entry(label)
                    .and_modify(|count| *count += points.len())
                    .or_insert_with(|| points.len());
            }
        }
        counts
    }

    pub fn to_svg(
        &self,
        label_colors: &LabelColorsHex,
        point_radius: usize,
        line_width: usize,
        img: &image::DynamicImage,
    ) -> svg::Document {
        let (image_width, image_height) = img.dimensions();
        let mut document = svg::Document::new()
            .set("width", image_width)
            .set("height", image_height)
            .set("viewBox", (0i64, 0i64, image_width, image_height))
            .set("xmlns:xlink", "http://www.w3.org/1999/xlink");
        let b64 = format!(
            "data:image/jpeg;base64,{}",
            img2base64(img, image::ImageOutputFormat::Jpeg(75))
        );
        let bg = element::Image::new()
            .set("x", 0i64)
            .set("y", 0i64)
            .set("width", image_width)
            .set("height", image_height)
            .set("xlink:href", b64);
        document = document.add(bg);
        let mut color_cycler = ColorCycler::new();
        let shape_map = self.to_shape_map();
        if let Some(point_data) = shape_map.get("point") {
            for (label, points) in point_data {
                let color = label_colors
                    .get(*label)
                    .map_or_else(|| color_cycler.cycle(), |s| s.as_str());
                let mut group = element::Group::new()
                    .set("class", format!("point {}", label))
                    .set("fill", color)
                    .set("stroke", "none");
                for point in points {
                    let point_xy = point[0];
                    let circle = element::Circle::new()
                        .set("cx", point_xy.0)
                        .set("cy", point_xy.1)
                        .set("r", point_radius);
                    group = group.add(circle);
                }
                document = document.add(group);
            }
        }
        if let Some(rectangle_data) = shape_map.get("rectangle") {
            for (label, rectangles) in rectangle_data {
                let color = label_colors
                    .get(*label)
                    .map_or_else(|| color_cycler.cycle(), |s| s.as_str());
                let mut group = element::Group::new()
                    .set("class", format!("rectangle {}", label))
                    .set("fill", "none")
                    .set("stroke", color)
                    .set("stroke-width", line_width);
                for rectangle in rectangles {
                    if rectangle.len() != 2 {
                        continue;
                    }
                    let rect = element::Rectangle::new()
                        .set("x", rectangle[0].0.min(rectangle[1].0))
                        .set("y", rectangle[0].1.min(rectangle[1].1))
                        .set("width", (rectangle[1].0 - rectangle[0].0).abs())
                        .set("height", (rectangle[1].1 - rectangle[0].1).abs());
                    group = group.add(rect);
                }
                document = document.add(group);
            }
        }
        if let Some(polygon_data) = shape_map.get("polygon") {
            let mut polygon_colors: IndexSet<&str> = IndexSet::default();
            for (label, polygons) in polygon_data {
                let color = label_colors
                    .get(*label)
                    .map_or_else(|| color_cycler.cycle(), |s| s.as_str());
                polygon_colors.insert(color);
                let mut group = element::Group::new()
                    .set("class", format!("polygon {}", label))
                    .set("fill", "none")
                    .set("stroke", color)
                    .set("stroke-width", line_width);
                for polygon in polygons {
                    let value: String = polygon
                        .iter()
                        .map(|(a, b)| format!("{} {}", a, b))
                        .collect::<Vec<String>>()
                        .join(" ");
                    let marker_url = format!("url(#dot{})", color);
                    let poly = element::Polygon::new()
                        .set("points", value)
                        .set("marker-start", marker_url.as_str())
                        .set("marker-mid", marker_url.as_str());
                    group = group.add(poly);
                }
                document = document.add(group);
            }
            let mut defs = svg::node::element::Definitions::new();
            for color in polygon_colors.into_iter() {
                let marker = svg::node::element::Marker::new()
                    .set("id", format!("dot{}", color))
                    .set(
                        "viewBox",
                        format!("0 0 {} {}", point_radius * 2, point_radius * 2),
                    )
                    .set("refX", point_radius)
                    .set("refY", point_radius)
                    .set("markerWidth", point_radius)
                    .set("markerHeight", point_radius)
                    .add(
                        element::Circle::new()
                            .set("cx", point_radius)
                            .set("cy", point_radius)
                            .set("r", point_radius)
                            .set("fill", color),
                    );
                defs = defs.add(marker);
            }
            document = document.add(defs);
        }
        if let Some(circle_data) = shape_map.get("circle") {
            for (label, circles) in circle_data {
                let color = label_colors
                    .get(*label)
                    .map_or_else(|| color_cycler.cycle(), |s| s.as_str());
                let mut group = element::Group::new()
                    .set("class", format!("circle {}", label))
                    .set("stroke-width", line_width);
                for circle in circles {
                    if circle.len() != 2 {
                        continue;
                    }
                    let center = element::Circle::new()
                        .set("cx", circle[0].0)
                        .set("cy", circle[0].1)
                        .set("r", point_radius)
                        .set("fill", color)
                        .set("stroke", "none");
                    group = group.add(center);
                    if circle.len() > 1 {
                        let (p1, p2) = (circle[0], circle[1]);
                        let radius = ((p1.0 - p2.0).powi(2) + (p1.1 - p2.1).powi(2)).sqrt();
                        let c = element::Circle::new()
                            .set("cx", circle[0].0)
                            .set("cy", circle[0].1)
                            .set("r", radius)
                            .set("fill", "none")
                            .set("stroke", color);
                        group = group.add(c);
                    }
                }
                document = document.add(group);
            }
        }
        document
    }
}

impl TryFrom<&str> for LabelMeData {
    type Error = serde_json::Error;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        serde_json::from_str(s)
    }
}

impl TryFrom<String> for LabelMeData {
    type Error = serde_json::Error;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&s)
    }
}

impl TryFrom<&Path> for LabelMeData {
    type Error = Box<dyn Error>;
    fn try_from(filename: &Path) -> Result<Self, Self::Error> {
        // It's faster to use `from_str` than to use `from_reader`
        // https://github.com/serde-rs/json/issues/160
        let s = std::fs::read_to_string(filename)?;
        Ok(s.as_str().try_into()?)
    }
}

pub fn load_image(path: &Path) -> Result<image::DynamicImage, Box<dyn Error>> {
    let img_fmt = image::ImageFormat::from_path(path)?;

    let img = match img_fmt {
        image::ImageFormat::Jpeg => {
            let buf = std::fs::read(path)?;
            let mut decoder = JpegDecoder::new(&buf);
            let pixels = decoder.decode()?;
            let color_space = decoder.get_input_colorspace().unwrap();
            let image_info = decoder.info().unwrap();
            match color_space {
                ColorSpace::Luma => image::ImageBuffer::from_raw(
                    image_info.width as u32,
                    image_info.height as u32,
                    pixels,
                )
                .map(image::DynamicImage::ImageLuma8)
                .unwrap(),
                ColorSpace::RGB | ColorSpace::RGBA | ColorSpace::YCbCr => {
                    image::ImageBuffer::from_raw(
                        image_info.width as u32,
                        image_info.height as u32,
                        pixels,
                    )
                    .map(image::DynamicImage::ImageRgb8)
                    .unwrap()
                }
                _ => panic!("Unsupported jpeg color space: {:?}", color_space),
            }
        }
        _ => image::open(path)?,
    };
    Ok(img)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Color(u8, u8, u8);

impl Color {
    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }
}

impl From<Color> for String {
    fn from(val: Color) -> Self {
        val.to_hex()
    }
}

pub type LabelColors = IndexMap<String, Color>;
pub type LabelColorsHex = IndexMap<String, String>;

pub static COLORS: [&str; 6] = ["red", "green", "blue", "cyan", "magenta", "yellow"];
pub struct ColorCycler {
    i: usize,
}

#[derive(thiserror::Error, Debug)]
pub enum LabelColorError {
    #[error("IO error: {0}")]
    IoError(std::io::Error),
    #[error("Invalid yaml: {0}")]
    YamlError(serde_yaml::Error),
    #[error("Invalid value: {0}")]
    ValueError(serde_yaml::Error),
}

pub fn load_label_colors(filename: &Path) -> Result<LabelColorsHex, LabelColorError> {
    let config: serde_yaml::Value = serde_yaml::from_reader(std::io::BufReader::new(
        std::fs::File::open(filename).map_err(LabelColorError::IoError)?,
    ))
    .map_err(LabelColorError::ValueError)?;
    let colors = config.get("label_colors");
    let label_colors: LabelColors = match colors {
        Some(colors) => {
            serde_yaml::from_value(colors.to_owned()).map_err(LabelColorError::ValueError)?
        }
        None => LabelColors::default(),
    };
    let hex = label_colors
        .into_iter()
        .map(|(k, v)| (k, v.into()))
        .collect();
    Ok(hex)
}

impl ColorCycler {
    /// Get next color
    pub fn cycle(&mut self) -> &'static str {
        let c = COLORS[self.i];
        self.i = (self.i + 1) % COLORS.len();
        c
    }
    pub fn new() -> Self {
        ColorCycler { i: 0 }
    }
}

impl Default for ColorCycler {
    fn default() -> Self {
        Self::new()
    }
}
