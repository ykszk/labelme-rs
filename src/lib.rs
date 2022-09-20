pub use image;
use image::GenericImageView;
pub use indexmap;
use indexmap::{IndexMap, IndexSet};
pub use serde;
use serde::{Deserialize, Serialize};
pub use serde_json;
use std::collections::HashMap;
use std::error::Error;
use std::io::Cursor;
use std::path::{Component, Path, PathBuf};
use std::{fs::File, io::BufReader};
pub use svg;
use svg::node::element;

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

#[deprecated(since = "0.0.0", note = "Use LabelMeData instead")]
pub type PointData = LabelMeData;

pub fn img2base64(img: &image::DynamicImage, format: image::ImageOutputFormat) -> String {
    let mut cursor = Cursor::new(Vec::new());
    img.write_to(&mut cursor, format).unwrap();
    base64::encode(&cursor.into_inner())
}

pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
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

    /// Convert to a shape_type-centered map with a structure map\[shape_type\]\[label\] -> points
    pub fn to_shape_map(self) -> HashMap<String, HashMap<String, Vec<Vec<Point>>>> {
        let mut map = HashMap::new();
        for shape in self.shapes {
            let m = map.entry(shape.shape_type).or_insert_with(HashMap::new);
            let v = m.entry(shape.label).or_insert_with(Vec::new);
            v.push(shape.points);
        }
        map
    }

    /// Scale points
    pub fn scale(&mut self, scale: f64) {
        for shape in self.shapes.iter_mut() {
            for p in shape.points.iter_mut() {
                p.0 = (scale * p.0 as f64) as f32;
                p.1 = (scale * p.1 as f64) as f32;
            }
        }
        self.imageWidth = (self.imageWidth as f64 * scale) as _;
        self.imageHeight = (self.imageHeight as f64 * scale) as _;
    }

    pub fn load(filename: &Path) -> Result<Self, Box<dyn Error>> {
        Ok(serde_json::from_reader(BufReader::new(File::open(
            filename,
        )?))?)
    }

    pub fn save(&self, filename: &Path) -> Result<(), Box<dyn Error>> {
        let writer = std::io::BufWriter::new(std::fs::File::create(filename)?);
        serde_json::to_writer_pretty(writer, self).map_err(|err| Box::new(err) as Box<dyn Error>)
    }

    /// Resolve imagePath
    ///
    /// # Arguments
    ///
    /// * json_path: absolute path
    ///
    /// ```
    /// use std::path::Path;
    /// let mut data = labelme_rs::LabelMeData::new(&[], &[], 128, 128, "image.jpg");
    ///
    /// let image_path = data.resolve_image_path(Path::new("image.json")).into_os_string().into_string().unwrap();
    /// assert_eq!(image_path, "image.jpg");
    ///
    /// let image_path = data.resolve_image_path(Path::new("/path/to/image.json")).into_os_string().into_string().unwrap();
    /// #[cfg(target_os = "windows")]
    /// assert_eq!(image_path, r"\path\to\image.jpg");
    /// #[cfg(not(target_os = "windows"))]
    /// assert_eq!(image_path, "/path/to/image.jpg");
    ///
    /// data.imagePath = "../image.jpg".into();
    /// let image_path = data.resolve_image_path(Path::new("/path/to/image.json")).into_os_string().into_string().unwrap();
    /// #[cfg(target_os = "windows")]
    /// assert_eq!(image_path, r"\path\image.jpg");
    /// #[cfg(not(target_os = "windows"))]
    /// assert_eq!(image_path, "/path/image.jpg");
    ///
    /// data.imagePath = "../images/image.jpg".into();
    /// let image_path = data.resolve_image_path(Path::new("/path/to/image.json")).into_os_string().into_string().unwrap();
    /// #[cfg(target_os = "windows")]
    /// assert_eq!(image_path, r"\path\images\image.jpg");
    /// #[cfg(not(target_os = "windows"))]
    /// assert_eq!(image_path, "/path/images/image.jpg");
    ///
    /// data.imagePath = r"..\images\image.jpg".into();
    /// let image_path = data.resolve_image_path(Path::new("/path/to/image.json")).into_os_string().into_string().unwrap();
    /// #[cfg(target_os = "windows")]
    /// assert_eq!(image_path, r"\path\images\image.jpg");
    /// #[cfg(not(target_os = "windows"))]
    /// assert_eq!(image_path, "/path/images/image.jpg");
    /// ```
    pub fn resolve_image_path(&self, json_path: &Path) -> PathBuf {
        let img_rel_to_json = json_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(&self.imagePath.replace('\\', "/"));
        normalize_path(&img_rel_to_json)
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
    pub fn count_labels(self) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
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
        self,
        label_colors: &LabelColorsHex,
        point_radius: usize,
        line_width: usize,
        img: &image::DynamicImage,
    ) -> Result<svg::Document, Box<dyn Error>> {
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
                    .get(label)
                    .cloned()
                    .unwrap_or_else(|| color_cycler.cycle().into());
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
                    .get(label)
                    .cloned()
                    .unwrap_or_else(|| color_cycler.cycle().into());
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
                        .set("x", rectangle[0].0)
                        .set("y", rectangle[0].1)
                        .set("width", rectangle[1].0 - rectangle[0].0)
                        .set("height", rectangle[1].1 - rectangle[0].1);
                    group = group.add(rect);
                }
                document = document.add(group);
            }
        }
        if let Some(circle_data) = shape_map.get("circle") {
            for (label, circles) in circle_data {
                let color = label_colors
                    .get(label)
                    .cloned()
                    .unwrap_or_else(|| color_cycler.cycle().into());
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
                        .set("fill", color.clone())
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
                            .set("stroke", color.clone());
                        group = group.add(c);
                    }
                }
                document = document.add(group);
            }
        }
        Ok(document)
    }
}

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

pub type LabelColors = IndexMap<String, Color>;
pub type LabelColorsHex = IndexMap<String, String>;

pub static COLORS: [&str; 6] = ["red", "green", "blue", "cyan", "magenta", "yellow"];
pub struct ColorCycler {
    i: usize,
}

pub fn load_label_colors(filename: &Path) -> Result<LabelColorsHex, Box<dyn std::error::Error>> {
    let config: serde_yaml::Value =
        serde_yaml::from_reader(std::io::BufReader::new(std::fs::File::open(filename)?))?;
    let colors = config.get("label_colors");
    let label_colors: LabelColors = match colors {
        Some(colors) => serde_yaml::from_value(colors.to_owned())?,
        None => LabelColors::new(),
    };
    let hex = label_colors
        .into_iter()
        .map(|(k, v)| (k, v.to_hex()))
        .collect();
    Ok(hex)
}

impl ColorCycler {
    /// Get next color
    pub fn cycle(&mut self) -> &str {
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
