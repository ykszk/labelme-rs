use std::path::PathBuf;

use clap::Parser;
use image::GenericImageView;
#[macro_use]
extern crate log;

/// Create SVG image from a labeme annotation
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Input json filename
    input: PathBuf,
    /// Output svg filename
    output: PathBuf,
    /// Config filename. Used for `label_colors`
    #[clap(short, long)]
    config: Option<PathBuf>,
    /// Circle radius
    #[clap(long, default_value = "2")]
    radius: usize,
    /// Line width
    #[clap(long, default_value = "2")]
    line_width: usize,
    /// Resize image. Specify in imagemagick's `-resize`-like format
    #[clap(long)]
    resize: Option<String>,
}

use labelme_rs::{load_label_colors, LabelColorsHex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    let mut json_data = labelme_rs::LabelMeData::load(&args.input)?;
    let label_colors = match args.config {
        Some(config) => load_label_colors(&config)?,
        None => LabelColorsHex::new(),
    };

    let img_filename = json_data.resolve_image_path(std::path::Path::new(&args.input));
    let mut img = image::open(&img_filename)?;
    if let Some(resize) = args.resize {
        let orig_size = img.dimensions();
        let re = regex::Regex::new(r"^(\d+)%$")?;
        if let Some(cap) = re.captures(&resize) {
            let p: f64 = cap.get(1).unwrap().as_str().parse::<u8>()? as f64 / 100.0;
            img = img.thumbnail(
                (p * img.dimensions().0 as f64) as u32,
                (p * img.dimensions().1 as f64) as u32,
            );
        } else {
            let re = regex::Regex::new(r"^(\d+)x(\d+)$")?;
            if let Some(cap) = re.captures(&resize) {
                let w: u32 = cap.get(1).unwrap().as_str().parse()?;
                let h: u32 = cap.get(2).unwrap().as_str().parse()?;
                img = img.thumbnail(w, h);
            } else {
                return Err(format!("{} is invalid resize argument", resize).into());
            }
        };
        info!(
            "Image is resized to {} x {}",
            img.dimensions().0,
            img.dimensions().1
        );
        let scale = img.dimensions().0 as f64 / orig_size.0 as f64;
        if scale != 1.0 {
            info!("Points are scaled by {}", scale);
            json_data.scale(scale);
        }
    }
    let document = json_data.to_svg(&label_colors, args.radius, args.line_width, &img)?;
    svg::save(args.output, &document)?;
    Ok(())
}
