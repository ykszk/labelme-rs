use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io::Cursor;
use std::{fs::File, io::BufReader};

pub type Flags = HashMap<String, bool>;
pub type FlagSet = HashSet<String>;
pub type Point = (f32, f32);

#[derive(Serialize, Deserialize, Debug)]
pub struct Shape {
    pub label: String,
    pub points: Vec<Point>,
    pub group_id: Option<String>,
    pub shape_type: String,
    pub flags: Flags,
}

#[derive(Serialize, Deserialize, Debug)]
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

    pub fn to_shape_map(self) -> HashMap<String, HashMap<String, Vec<Vec<Point>>>> {
        let mut map = HashMap::new();
        for shape in self.shapes {
            let m = map.entry(shape.shape_type).or_insert_with(HashMap::new);
            let v = m.entry(shape.label).or_insert_with(Vec::new);
            v.push(shape.points);
        }
        map
    }

    pub fn load(filename: &str) -> Result<Self, Box<dyn Error>> {
        Ok(serde_json::from_reader(BufReader::new(File::open(
            filename,
        )?))?)
    }

    pub fn save(&self, filename: &str) -> Result<(), Box<dyn Error>> {
        let writer = std::io::BufWriter::new(std::fs::File::create(filename)?);
        serde_json::to_writer_pretty(writer, self).map_err(|err| Box::new(err) as Box<dyn Error>)
    }
}
