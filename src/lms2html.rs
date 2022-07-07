use clap::Parser;
use image::GenericImageView;
use indexmap::IndexSet;
use std::{io::Write, path::PathBuf};
#[macro_use]
extern crate log;

/// Create HTML from labelme directory
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Input labelme directory
    input: PathBuf,
    /// Output html filename
    output: String,
    /// Config filename. Used for `label_colors`
    #[clap(short, long)]
    config: Option<String>,
    /// Circle radius
    #[clap(long, default_value = "2")]
    radius: usize,
    /// Line width
    #[clap(long, default_value = "2")]
    line_width: usize,
    /// Resize image. Specify in imagemagick's `-resize`-like format
    #[clap(long)]
    resize: Option<String>,
    /// HTML title
    #[clap(long, default_value = "catalog")]
    title: String,
    /// CSS filename
    #[clap(long)]
    css: Option<String>,
}

use labelme_rs::{load_label_colors, LabelColorsHex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error")).init();

    let args = Args::parse();
    let mut templates = tera::Tera::new("/dev/null/*")?;
    templates.autoescape_on(vec![]);
    templates.add_raw_templates(vec![
        ("catalog.html", include_str!("templates/catalog.html")),
        ("img.html", include_str!("templates/img.html")),
        ("legend.html", include_str!("templates/legend.html")),
        (
            "tag_checkbox.html",
            include_str!("templates/tag_checkbox.html"),
        ),
    ])?;
    let entries: Vec<_> = glob::glob(args.input.join("*.json").to_str().unwrap())
        .expect("Failed to read glob pattern")
        .collect();
    let bar = indicatif::ProgressBar::new(entries.len() as _);
    bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed}<{eta}] | {wide_bar} | {pos}/{len}"),
    );
    let label_colors = match args.config {
        Some(config) => load_label_colors(&config)?,
        None => LabelColorsHex::new(),
    };
    let mut all_tags: IndexSet<String> = IndexSet::new();
    let mut svgs: Vec<String> = Vec::new();
    for entry in entries {
        let input = entry?;
        let mut json_data = labelme_rs::LabelMeData::load(input.to_str().unwrap())?;

        let img_filename = json_data.resolve_image_path(input.as_path());
        let mut img = image::open(&img_filename)?;
        if let Some(resize) = &args.resize {
            let orig_size = img.dimensions();
            let re = regex::Regex::new(r"^(\d+)%$")?;
            if let Some(cap) = re.captures(resize) {
                let p: f64 = cap.get(1).unwrap().as_str().parse::<u8>()? as f64 / 100.0;
                img = img.thumbnail(
                    (p * img.dimensions().0 as f64) as u32,
                    (p * img.dimensions().1 as f64) as u32,
                );
            } else {
                let re = regex::Regex::new(r"^(\d+)x(\d+)$")?;
                if let Some(cap) = re.captures(resize) {
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
        for (flag, checked) in json_data.flags.iter() {
            if *checked {
                all_tags.insert(flag.clone()); // should get_or_insert_with be used?
            }
        }
        let flags: Vec<_> = json_data
            .flags
            .iter()
            .filter(|(_k, v)| **v)
            .map(|(k, _v)| k.clone())
            .collect();
        let flags = flags.join(" ");
        let document = json_data.to_svg(&label_colors, args.radius, args.line_width, &img)?;
        let mut context = tera::Context::new();
        context.insert("tags", &flags);
        context.insert("flags", &flags);
        context.insert("name", input.file_name().unwrap().to_str().unwrap());
        context.insert("svg", &document.to_string());
        let fig = templates.render("img.html", &context)?;
        svgs.push(fig);
        bar.inc(1);
    }
    bar.finish();
    let tag_cbs = all_tags
        .iter()
        .map(|tag| {
            let mut context = tera::Context::new();
            context.insert("tag", &tag);
            templates.render("tag_checkbox.html", &context).unwrap()
        })
        .collect::<Vec<_>>()
        .join("\n");
    let legend = label_colors
        .iter()
        .map(|(k, v)| {
            let mut context = tera::Context::new();
            context.insert("label", &k);
            context.insert("color", &v);
            templates.render("legend.html", &context).unwrap()
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut writer = std::io::BufWriter::new(std::fs::File::create(args.output)?);
    let mut context = tera::Context::new();
    let style = if let Some(css) = args.css {
        std::fs::read_to_string(css)?
    } else {
        include_str!("templates/default.css").into()
    };
    context.insert("title", &args.title);
    context.insert("legend", &legend);
    context.insert("tag_checkboxes", &tag_cbs);
    context.insert("main", &svgs.join("\n"));
    context.insert("style", &style);
    let html = templates.render("catalog.html", &context)?;
    writer.write_all(html.as_bytes()).unwrap();
    Ok(())
}
