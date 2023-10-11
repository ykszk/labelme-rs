use anyhow::{Context, Result};
use std::{io::Read, path::PathBuf};

use labelme_rs::{load_label_colors, LabelColorsHex};
use lmrs::cli::SvgCmdArgs as CmdArgs;

pub fn cmd(args: CmdArgs) -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let json_data = labelme_rs::LabelMeData::try_from(if args.input.as_os_str() == "-" {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    } else {
        std::fs::read_to_string(&args.input)?
    })?;
    let label_colors = match args.config {
        Some(config) => load_label_colors(&config)?,
        None => LabelColorsHex::new(),
    };

    let original_dir = std::env::current_dir()?;
    let json_dir: PathBuf = if args.input.is_dir() {
        args.input.clone()
    } else if args.input.as_os_str() == "-" {
        PathBuf::from(".")
    } else {
        args.input
            .canonicalize()?
            .parent()
            .context("Failed to get parent")?
            .into()
    };
    std::env::set_current_dir(json_dir)?;
    let mut data_w_image: labelme_rs::LabelMeDataWImage = json_data.try_into()?;
    std::env::set_current_dir(original_dir)?;
    if let Some(resize) = args.resize {
        let resize_param = labelme_rs::ResizeParam::try_from(resize.as_str())?;
        data_w_image.resize(&resize_param);
    }
    let document = data_w_image.data.to_svg(
        &label_colors,
        args.radius,
        args.line_width,
        &data_w_image.image,
    );
    labelme_rs::svg::save(args.output, &document)?;
    Ok(())
}
