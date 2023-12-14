use anyhow::{Context, Result};
use std::io::Read;

use labelme_rs::{load_label_colors, LabelColorsHex};
use lmrs::cli::SvgCmdArgs as CmdArgs;

pub fn cmd(args: CmdArgs) -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let mut json_data = labelme_rs::LabelMeData::try_from(if args.input.as_os_str() == "-" {
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

    if args.input.as_os_str() != "-" {
        let canonical_input = args.input.canonicalize()?;
        let json_dir = canonical_input
            .parent()
            .with_context(|| format!("Failed to get parent directory of:{:?}", args.input))?;
        json_data = json_data.to_absolute_path(json_dir);
    };
    let mut data_w_image: labelme_rs::LabelMeDataWImage = json_data.try_into()?;
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
