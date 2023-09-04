use clap::Args;
use labelme_rs::image::GenericImageView;
use labelme_rs::indexmap::IndexMap;
use labelme_rs::serde_json;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Args)]
pub struct CmdArgs {
    /// Input labelme directory or jsonl with `filename` data (e.g. output of `lmrs jsonl`).
    /// Specify "-" to use stdin as input
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
    /// The number of jobs. Use all available cores by default.
    #[clap(short, long)]
    jobs: Option<usize>,
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
    let n_jobs = if let Some(n) = args.jobs {
        n
    } else {
        std::thread::available_parallelism()?.get()
    };
    info!("Use {n_jobs} cores");
    info!("Load jsons");
    let mut entries: Vec<(PathBuf, Box<labelme_rs::LabelMeData>)> = if args.input.is_dir() {
        let entries: Result<Vec<_>, Box<dyn std::error::Error>> =
            glob::glob(args.input.join("*.json").to_str().unwrap())
                .expect("Failed to read glob pattern")
                .map(|entry| {
                    let entry = entry?;
                    let json_data = labelme_rs::LabelMeData::try_from(entry.as_path())?;
                    Ok((entry, Box::new(json_data)))
                })
                .collect();
        entries?
    } else {
        let reader: Box<dyn BufRead> = if args.input.as_os_str() == "-" {
            Box::new(BufReader::new(std::io::stdin()))
        } else {
            Box::new(BufReader::new(File::open(&args.input).unwrap()))
        };
        let mut entries: Vec<_> = Vec::new();
        for line in reader.lines() {
            let line = line?;
            let mut json_data: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(&line).unwrap();
            let v_filename = json_data
                .remove("filename")
                .ok_or_else(|| format!("Key '{}' not found", "filename"))?;
            let serde_json::Value::String(filename) = v_filename else {panic!("expected String")};
            let json_data = labelme_rs::LabelMeData::try_from(line.as_str())?;
            entries.push((filename.into(), Box::new(json_data)));
        }
        entries
    };
    if entries.is_empty() {
        return Err("No json file found.".into());
    }
    let json_dir: PathBuf = if args.input.is_dir() {
        args.input.clone()
    } else if args.input.as_os_str() == "-"  {
        PathBuf::from(".")
    } else {
        args.input.parent().unwrap().into()
    };
    let bar = indicatif::ProgressBar::new(entries.len() as _);
    bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed}<{eta}] | {wide_bar} | {pos}/{len}")?,
    );
    let shared_bar = Arc::new(Mutex::new(bar));
    let label_colors = match args.config {
        Some(config) => load_label_colors(&config)?,
        None => LabelColorsHex::new(),
    };
    let mut all_tags: IndexMap<String, bool> = match args.flags {
        Some(filename) => {
            let buff_reader = std::io::BufReader::new(std::fs::File::open(filename)?);
            buff_reader
                .lines()
                .map(|l| l.expect("Could not parse line"))
                .map(|e| (e, false))
                .collect()
        }
        None => IndexMap::new(),
    };
    let mut svgs: Vec<String> = Vec::with_capacity(entries.len());
    let resize_param = match args.resize {
        Some(s) => Some(dsl::ResizeParam::try_from(s.as_str())?),
        None => None,
    };

    let original_dir = std::env::current_dir().expect("Failed to acquire cwd");
    std::env::set_current_dir(&json_dir)
        .unwrap_or_else(|_| panic!("Failed to change directory to {:?}", json_dir));
    std::thread::scope(|scope| {
        let mut handles: Vec<_> = Vec::with_capacity(n_jobs);
        let chunk_size = (entries.len() as f64 / n_jobs as f64).ceil() as usize;
        for chunk in entries.chunks_mut(chunk_size) {
            handles.push(scope.spawn(|| {
                let mut svgs: Vec<String> = Vec::with_capacity(chunk.len());
                let mut all_tags: IndexMap<String, bool> = IndexMap::new();
                for entry in chunk.iter_mut() {
                    let input = &mut entry.0;
                    let json_data = &mut entry.1;

                    let img_filename = if let Some(image_dir) = &args.image_dir {
                        let image_path = json_data.imagePath.replace('\\', "/");
                        let filename = Path::new(&image_path).file_name().unwrap();
                        image_dir.join(filename)
                    } else {
                        let p = PathBuf::from(&json_data.imagePath);
                        p.canonicalize().unwrap_or_else(|_| {
                            panic!("Failed to canonicalize {}", json_data.imagePath)
                        })
                    };
                    let mut img = labelme_rs::load_image(&img_filename).unwrap_or_else(|e| {
                        panic!("Failed to load image {:?}: {:?}", img_filename, e)
                    });
                    match &resize_param {
                        Some(param) => {
                            let orig_size = img.dimensions();
                            img = param.resize(&img);
                            debug!(
                                "Image is resized to {} x {}",
                                img.dimensions().0,
                                img.dimensions().1
                            );
                            let scale = img.dimensions().0 as f64 / orig_size.0 as f64;
                            if (scale - 1.0).abs() > f64::EPSILON {
                                debug!("Points are scaled by {}", scale);
                                json_data.scale(scale);
                            }
                        }
                        None => {}
                    };

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
                        .map(|(k, v)| format!("{k}:{v}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    let document = json_data
                        .to_svg(&label_colors, args.radius, args.line_width, &img)
                        .expect("SVG conversion failed");
                    let mut context = tera::Context::new();
                    context.insert("tags", &flags);
                    context.insert("flags", &flags);
                    context.insert("title", &title);
                    context.insert("name", input.file_name().unwrap().to_str().unwrap());
                    context.insert("svg", &document.to_string());
                    let fig = templates
                        .render("img.html", &context)
                        .expect("Failed to render img.html");
                    svgs.push(fig);
                    let bar = shared_bar.lock().unwrap();
                    bar.inc(1);
                }
                Ok((svgs, all_tags))
            }));
        }
        for handle in handles {
            let svgs_all_tags: Result<_, String> = handle.join().unwrap();
            let mut svgs_all_tags = svgs_all_tags.unwrap();
            svgs.append(&mut svgs_all_tags.0);
            for (flag, checked) in svgs_all_tags.1.iter() {
                if *checked {
                    *all_tags.entry(flag.into()).or_insert(true) = true;
                }
            }
        }
    });
    {
        shared_bar.lock().unwrap().finish();
    };
    info!("Generate html");
    std::env::set_current_dir(original_dir)
        .unwrap_or_else(|_| panic!("Failed to change directory to {:?}", json_dir));
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
    info!("Write html");
    let mut writer = std::io::BufWriter::new(std::fs::File::create(args.output)?);
    writer.write_all(html.as_bytes()).unwrap();
    info!("Done");
    Ok(())
}
