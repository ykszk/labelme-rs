use base64::Engine;
pub use image;
use image::GenericImageView;
pub use indexmap;
use indexmap::{IndexMap, IndexSet};
use regex::Regex;
pub use serde;
use serde::{Deserialize, Serialize};
pub use serde_json;
use std::collections::HashMap;
use std::error::Error;
use std::io::Cursor;
use std::path::Path;
pub use svg;
use svg::node::element;
use thiserror::Error;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::JpegDecoder;
#[macro_use]
extern crate lazy_static;

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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
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

#[derive(Error, Debug)]
pub enum LabelMeDataError {
    #[error("IO Error")]
    IoError(#[from] std::io::Error),
    #[error("Json Error")]
    SerdeError(#[from] serde_json::Error),
    #[error("Image Error")]
    ImageError(#[from] image::ImageError),
}

#[derive(Debug, Clone)]
pub struct LabelMeDataWImage {
    pub data: LabelMeData,
    pub image: image::DynamicImage,
}

impl TryFrom<&Path> for LabelMeDataWImage {
    type Error = LabelMeDataError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let s = std::fs::read_to_string(path)?;
        let mut data: LabelMeData = s.try_into()?;
        data.imagePath = data.imagePath.replace('\\', "/");
        if let Some(parent) = path.parent() {
            let path = parent.canonicalize()?;
            data = data.to_absolute_path(path.as_path());
        }
        let data = LabelMeDataWImage::try_from(data)?;
        Ok(data)
    }
}

impl TryFrom<LabelMeData> for LabelMeDataWImage {
    type Error = image::ImageError;

    fn try_from(data: LabelMeData) -> Result<Self, Self::Error> {
        let image = image::open(&data.imagePath)?;
        Ok(Self { data, image })
    }
}

impl LabelMeDataWImage {
    pub fn new(data: LabelMeData, image: image::DynamicImage) -> Self {
        Self { data, image }
    }

    pub fn resize(&mut self, param: &ResizeParam) {
        let scale = param.scale(self.image.width(), self.image.height());
        if scale > 0.0 && scale != 1.0 {
            self.image = param.resize(&self.image);
            self.data.scale(scale)
        }
    }
}

/// LabeleMeData with additional `filename` field for ndjsons
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct LabelMeDataLine {
    pub content: LabelMeData,
    pub filename: String,
}

impl TryFrom<&str> for LabelMeDataLine {
    type Error = serde_json::Error;

    fn try_from(json: &str) -> Result<Self, Self::Error> {
        serde_json::from_str(json)
    }
}

/// Resizing parameter represented by percentage or size.
/// Resizing does not change image's aspect ratio.
/// Use imagemagick's `-resize`-like format to construct.
#[derive(Debug, Clone, PartialEq)]
pub enum ResizeParam {
    Percentage(f64),
    Size(u32, u32),
}

lazy_static! {
    static ref RE_PERCENT: Regex = Regex::new(r"^(\d+)%$").unwrap();
    static ref RE_SIZE: Regex = Regex::new(r"^(\d+)x(\d+)$").unwrap();
}

#[derive(Error, Debug)]
pub enum ResizeParamError {
    #[error("int parse error")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Invalid format: {0}")]
    FormatError(String),
}

impl TryFrom<&str> for ResizeParam {
    type Error = ResizeParamError;

    /// Parse resize parameter
    /// ```
    /// use labelme_rs::ResizeParam;
    /// assert_eq!(ResizeParam::try_from("33%").unwrap(), ResizeParam::Percentage(0.33));
    /// assert_eq!(ResizeParam::try_from("300x400").unwrap(), ResizeParam::Size(300, 400));
    /// assert!(ResizeParam::try_from("300x400!").is_err()); // Flags `!><^%@` etc. are not supported.
    /// ```
    fn try_from(param: &str) -> Result<Self, Self::Error> {
        if let Some(cap) = RE_PERCENT.captures(param) {
            let p: f64 = cap.get(1).unwrap().as_str().parse::<u32>()? as f64 / 100.0;
            Ok(ResizeParam::Percentage(p))
        } else if let Some(cap) = RE_SIZE.captures(param) {
            let w: u32 = cap.get(1).unwrap().as_str().parse()?;
            let h: u32 = cap.get(2).unwrap().as_str().parse()?;
            Ok(ResizeParam::Size(w, h))
        } else {
            Err(ResizeParamError::FormatError(param.into()))
        }
    }
}

impl ResizeParam {
    /// Resize image
    pub fn resize(&self, img: &image::DynamicImage) -> image::DynamicImage {
        match self {
            Self::Percentage(..) => {
                let size = self.size(img.dimensions().0, img.dimensions().1);
                img.thumbnail(size.0, size.1)
            }
            Self::Size(w, h) => img.thumbnail(*w, *h),
        }
    }

    /// Calculate size after resizing
    /// ```
    /// use labelme_rs::ResizeParam;
    /// let param = ResizeParam::try_from("300x400").unwrap();
    /// assert_eq!(param.size(512, 512), (300, 300));
    pub fn size(&self, width: u32, height: u32) -> (u32, u32) {
        match self {
            Self::Percentage(p) => (
                (p * width as f64).round() as u32,
                (p * height as f64).round() as u32,
            ),
            Self::Size(..) => {
                let p = self.scale(width, height);
                Self::Percentage(p).size(width, height)
            }
        }
    }

    /// Calculate scaling factor from the given image dimension to self
    /// ```
    /// use labelme_rs::ResizeParam;
    /// let param = ResizeParam::try_from("75%").unwrap();
    /// assert_eq!(param.scale(10, 10), 0.75);
    /// let param = ResizeParam::try_from("300x400").unwrap();
    /// assert_eq!(param.scale(150, 200), 2.0);
    /// assert_eq!(param.scale(512, 512), 0.5859375);
    /// ```
    pub fn scale(&self, width: u32, height: u32) -> f64 {
        match self {
            Self::Percentage(p) => *p,
            Self::Size(nwidth, nheight) => {
                let wratio = *nwidth as f64 / width as f64;
                let hratio = *nheight as f64 / height as f64;
                f64::min(wratio, hratio)
            }
        }
    }
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

    /// Scale points, imageWidth and imageHeight
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

    /// Shift points.
    /// Does not change imageWidth and imageHeight
    pub fn shift(&mut self, tx: f64, ty: f64) {
        for shape in &mut self.shapes {
            for p in &mut shape.points {
                p.0 = (p.0 as f64 + tx) as f32;
                p.1 = (p.1 as f64 + ty) as f32;
            }
        }
    }

    /// Update `imagePath` to absolute path if it is relative
    pub fn to_absolute_path(mut self, canonical_json_dir: &Path) -> Self {
        let image_path = Path::new(&self.imagePath);
        if image_path.is_relative() {
            self.imagePath = canonical_json_dir
                .join(image_path)
                .to_str()
                .unwrap()
                .to_string();
        }
        self
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
        let mut color_cycler = ColorCycler::default();
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
        let mut line_colors: IndexSet<&str> = IndexSet::default();
        if let Some(line_data) = shape_map.get("line") {
            for (label, lines) in line_data {
                let color = label_colors
                    .get(*label)
                    .map_or_else(|| color_cycler.cycle(), |s| s.as_str());
                line_colors.insert(color);
                let mut group = element::Group::new()
                    .set("class", format!("line {}", label))
                    .set("fill", "none")
                    .set("stroke", color)
                    .set("stroke-width", line_width);
                for line in lines {
                    let line = element::Line::new()
                        .set("x1", line[0].0)
                        .set("y1", line[0].1)
                        .set("x2", line[1].0)
                        .set("y2", line[1].1);
                    group = group.add(line);
                }
                document = document.add(group);
            }
        }
        if let Some(polyline_data) = shape_map.get("linestrip") {
            for (label, polylines) in polyline_data {
                let color = label_colors
                    .get(*label)
                    .map_or_else(|| color_cycler.cycle(), |s| s.as_str());
                line_colors.insert(color);
                let mut group = element::Group::new()
                    .set("class", format!("linestrip {}", label))
                    .set("fill", "none")
                    .set("stroke", color)
                    .set("stroke-width", line_width);
                for polyline in polylines {
                    let points = polyline
                        .iter()
                        .map(|p| format!("{} {}", p.0, p.1))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let polyline = element::Polyline::new().set("points", points);
                    group = group.add(polyline);
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
    type Error = LabelMeDataError;
    fn try_from(filename: &Path) -> Result<Self, Self::Error> {
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

#[derive(Serialize, Deserialize, Debug)]
pub struct LabelColorsInConfig {
    label_colors: LabelColors,
}

pub type LabelColors = HashMap<String, Color>;
pub type LabelColorsHex = HashMap<String, String>;

pub static TAB10: [&str; 10] = [
    "#1f77b4", "#ff7f0f", "#2ca02c", "#d62728", "#9467bd", "#8c564b", "#e377c2", "#7f7f7f",
    "#bcbd22", "#16becf",
];
pub static NEW_TAB10: [&str; 10] = [
    "#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14e", "#edc949", "#af7aa1", "#ff9da7",
    "#9c755f", "#bab0ac",
];
pub static RGBCMY: [&str; 6] = ["red", "green", "blue", "cyan", "magenta", "yellow"];

#[derive(Debug)]
pub struct ColorCycler {
    i: usize,
    palette: Vec<&'static str>,
}

impl Default for ColorCycler {
    fn default() -> Self {
        ColorCycler {
            i: 0,
            palette: Vec::from(TAB10),
        }
    }
}

impl From<Vec<&'static str>> for ColorCycler {
    fn from(palette: Vec<&'static str>) -> Self {
        ColorCycler { i: 0, palette }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum LabelColorError {
    #[error("IO error")]
    IoError(#[from] std::io::Error),
    #[error("Yaml error")]
    YamlError(#[from] serde_yaml::Error),
}

/// Load colormap written in yaml
/// Example: `label_colors:{"L1": [255, 0, 0], "L2": [0, 255, 0]}`
pub fn load_label_colors(filename: &Path) -> Result<LabelColorsHex, LabelColorError> {
    let config: LabelColorsInConfig =
        serde_yaml::from_reader(std::io::BufReader::new(std::fs::File::open(filename)?))?;
    let hex =
        LabelColorsHex::from_iter(config.label_colors.into_iter().map(|(k, v)| (k, v.into())));
    Ok(hex)
}

impl ColorCycler {
    /// Get next color
    pub fn cycle(&mut self) -> &'static str {
        let c = self.palette[self.i];
        self.i = (self.i + 1) % self.palette.len();
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Context, Result};

    #[test]
    fn test_lmdata_line() -> Result<()> {
        let lmd = LabelMeData::default();
        let lmd_string = serde_json::to_string(&lmd)?;
        let lmd: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&lmd_string)?;
        let mut lmdl: serde_json::Map<String, serde_json::Value> = serde_json::Map::default();
        assert!(lmdl.insert("filename".into(), "1.json".into()).is_none());
        assert!(lmdl.insert("content".into(), lmd.into()).is_none());
        let lmdl_string = serde_json::to_string(&lmdl)?;
        let restored: LabelMeDataLine = lmdl_string.as_str().try_into()?;
        assert_eq!(restored.filename, "1.json");
        Ok(())
    }

    #[test]
    fn test_image_load() -> Result<()> {
        let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/data");
        let json_file = data_dir.join("Mandrill.json");
        let jsons = std::fs::read_to_string(&json_file)
            .with_context(|| format!("Read json file:{:?}", json_file))?;
        let data = LabelMeData::try_from(jsons)?;
        let data = data.to_absolute_path(data_dir.canonicalize()?.as_path());
        let w_image: LabelMeDataWImage = data.try_into()?;
        let expected = data_dir.join("Mandrill.jpg").canonicalize()?;
        assert_eq!(w_image.data.imagePath, expected.to_string_lossy());
        Ok(())
    }

    #[test]
    fn test_resize() -> anyhow::Result<()> {
        let param = ResizeParam::Size(50, 10);
        let scale = param.scale(100, 100);
        assert_eq!(scale, 0.1);
        let param = ResizeParam::Size(10, 50);
        let scale = param.scale(100, 100);
        assert_eq!(scale, 0.1);
        let param = ResizeParam::Size(1000, 200);
        let scale = param.scale(100, 100);
        assert_eq!(scale, 2.0);
        Ok(())
    }

    #[test]
    fn test_color_cycler() {
        let mut cycler = ColorCycler::default();
        for i in 0..=11 {
            assert_eq!(cycler.cycle(), TAB10[i % 10]);
        }
    }
}
