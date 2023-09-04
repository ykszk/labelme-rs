use clap::Args;
use labelme_rs::image::GenericImageView;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct CmdArgs {
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

pub fn cmd(args: CmdArgs) -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let mut json_data = labelme_rs::LabelMeData::try_from(args.input.as_path())?;
    let label_colors = match args.config {
        Some(config) => load_label_colors(&config)?,
        None => LabelColorsHex::new(),
    };

    let original_dir = std::env::current_dir()?;
    let json_dir = args
        .input
        .parent()
        .unwrap_or_else(|| panic!("Failed to find parent directory of {:?}", args.input));
    std::env::set_current_dir(json_dir)?;

    let img_filename = json_dir.join(&json_data.imagePath);
    std::env::set_current_dir(original_dir)?;
    let mut img = labelme_rs::image::open(img_filename)?;
    if let Some(resize) = args.resize {
        let resize_param = dsl::ResizeParam::try_from(resize.as_str())?;
        let orig_size = img.dimensions();
        img = resize_param.resize(&img);
        let scale = img.dimensions().0 as f64 / orig_size.0 as f64;
        if (scale - 1.0).abs() > f64::EPSILON {
            info!("Points are scaled by {}", scale);
            json_data.scale(scale);
        }
    }
    let document = json_data.to_svg(&label_colors, args.radius, args.line_width, &img)?;
    labelme_rs::svg::save(args.output, &document)?;
    Ok(())
}
