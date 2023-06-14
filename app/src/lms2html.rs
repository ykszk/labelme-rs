use clap::Args;
use labelme_rs::image::GenericImageView;
use labelme_rs::indexmap::IndexMap;
use std::{io::BufRead, io::Write, path::Path, path::PathBuf};

#[derive(Debug, Args)]
pub struct CmdArgs {
    /// Input labelme directory
    input: PathBuf,
    /// Output html filename
    output: PathBuf,
    /// Config filename. Used for `label_colors`
    #[clap(short, long)]
    config: Option<PathBuf>,
    /// Flags filename. Used to sort flags
    #[clap(short, long)]
    flags: Option<PathBuf>,
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
    css: Option<PathBuf>,
    /// Override imagePath's directory
    #[clap(long)]
    image_dir: Option<PathBuf>,
}

use labelme_rs::{load_label_colors, LabelColorsHex};

pub fn cmd(args: CmdArgs) -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error")).init();

    let mut templates = tera::Tera::default();
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
    if !args.input.exists() {
        return Err(format!("Input {} not found.", args.input.to_string_lossy()).into());
    }
    if args.input.is_file() {
        return Err(format!("Input {} is not a directory.", args.input.to_string_lossy()).into());
    }
    let entries: Vec<_> = glob::glob(args.input.join("*.json").to_str().unwrap())
        .expect("Failed to read glob pattern")
        .collect();
    if entries.is_empty() {
        return Err("No json file found.".into());
    }
    let bar = indicatif::ProgressBar::new(entries.len() as _);
    bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed}<{eta}] | {wide_bar} | {pos}/{len}"),
    );
    let label_colors = match args.config {
        Some(config) => load_label_colors(&config)?,
        None => LabelColorsHex::new(),
    };
    let mut all_tags: IndexMap<String, bool> = match args.flags {
        Some(filename) => {
            let buff_reader = std::io::BufReader::new(std::fs::File::open(filename)?);
            IndexMap::from_iter(
                buff_reader
                    .lines()
                    .map(|l| l.expect("Could not parse line"))
                    .map(|e| (e, false)),
            )
        }
        None => IndexMap::new(),
    };
    let mut svgs: Vec<String> = Vec::new();
    for entry in entries {
        let input = entry?;
        let mut json_data = labelme_rs::LabelMeData::try_from(input.as_path())?;

        let img_filename = if let Some(image_dir) = &args.image_dir {
            let filename = Path::new(&json_data.imagePath).file_name().unwrap();
            image_dir.join(filename)
        } else {
            json_data.resolve_image_path(&std::fs::canonicalize(&input)?)
        };
        let mut img = labelme_rs::image::open(&img_filename).unwrap_or_else(|_| {
            panic!(
                "Image file {} not found.",
                img_filename.as_os_str().to_str().unwrap()
            )
        });
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
                *all_tags.entry(flag.into()).or_insert(true) = true;
            }
        }
        let flags: Vec<_> = json_data
            .flags
            .iter()
            .filter(|(_k, v)| **v)
            .map(|(k, _v)| k.clone())
            .collect();
        let flags = flags.join(" ");
        let label_counts = json_data.clone().count_labels();
        let title = label_counts
            .iter()
            .map(|(k, v)| format!("{}:{}", k, v))
            .collect::<Vec<_>>()
            .join("\n");
        let document = json_data.to_svg(&label_colors, args.radius, args.line_width, &img)?;
        let mut context = tera::Context::new();
        context.insert("tags", &flags);
        context.insert("flags", &flags);
        context.insert("title", &title);
        context.insert("name", input.file_name().unwrap().to_str().unwrap());
        context.insert("svg", &document.to_string());
        let fig = templates.render("img.html", &context)?;
        svgs.push(fig);
        bar.inc(1);
    }
    bar.finish();
    let tag_cbs = all_tags
        .iter()
        .filter_map(|(tag, checked)| {
            if *checked {
                let mut context = tera::Context::new();
                context.insert("tag", &tag);
                Some(templates.render("tag_checkbox.html", &context).unwrap())
            } else {
                None
            }
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
