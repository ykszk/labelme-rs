use anyhow::{ensure, Context, Result};
use labelme_rs::indexmap::{IndexMap, IndexSet};
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use labelme_rs::{load_label_colors, LabelColorsHex, LabelMeDataWImage};
use lmrs::cli::HtmlCmdArgs as CmdArgs;

pub fn cmd(args: CmdArgs) -> Result<()> {
    if let Some(jobs) = args.jobs {
        rayon::ThreadPoolBuilder::new()
            .num_threads(jobs)
            .build_global()?;
    }

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
        (
            "shape_toggle.html",
            include_str!("templates/shape_toggle.html"),
        ),
    ])?;

    let entries: Vec<(PathBuf, Box<labelme_rs::LabelMeData>)> = if args.input.is_dir() {
        debug!("Load from directory");
        let entries: Result<Vec<_>> = glob::glob(
            args.input
                .join("*.json")
                .to_str()
                .context("Failed to obtain glob string")?,
        )
        .expect("Failed to read glob pattern")
        .map(|entry| {
            let entry = entry?;
            let s = std::fs::read_to_string(&entry)?;
            let obj = labelme_rs::LabelMeData::try_from(s.as_str())?;
            Ok((entry, obj.into()))
        })
        .collect();
        entries?
    } else {
        let reader: Box<dyn BufRead> = if args.input.as_os_str() == "-" {
            debug!("Load from stdin");
            Box::new(BufReader::new(std::io::stdin()))
        } else {
            debug!("Load from file");
            Box::new(BufReader::new(
                File::open(&args.input)
                    .with_context(|| format!("Open {}", args.input.display()))?,
            ))
        };
        let entries: Result<Vec<_>> = reader
            .lines()
            .map(|line| {
                let line = line?;
                let json_data = labelme_rs::LabelMeDataLine::try_from(line.as_str())?;
                Ok((
                    PathBuf::from(json_data.filename),
                    Box::new(json_data.content),
                ))
            })
            .collect();
        entries?
    };

    ensure!(!entries.is_empty(), "No json file found.");
    let json_dir: PathBuf = if args.input.is_dir() {
        args.input.canonicalize()?
    } else if args.input.as_os_str() == "-" {
        PathBuf::from(".").canonicalize()?
    } else {
        args.input
            .canonicalize()?
            .parent()
            .context("Input has no parent directory")?
            .to_path_buf()
    };
    let json_dir = if let Some(image_dir) = &args.image_dir {
        image_dir.canonicalize()?
    } else {
        json_dir
    };
    let bar = indicatif::ProgressBar::new(entries.len() as _);
    bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed}<{eta}] | {wide_bar} | {pos}/{len}")?,
    );
    let shared_bar = Arc::new(Mutex::new(bar));
    let mut label_colors = match args.svg.config {
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
    let mut all_labels: IndexSet<String> = IndexSet::default();
    let mut all_shapes: IndexSet<String> = IndexSet::default();
    debug!("Collect tag and label info");
    for (_, json_data) in entries.iter() {
        for (flag, checked) in &json_data.flags {
            if *checked {
                all_tags
                    .entry(flag.to_string())
                    .and_modify(|v| *v = true)
                    .or_insert(true);
            }
        }
        for shape in &json_data.shapes {
            all_labels.insert(shape.label.clone());
            all_shapes.insert(shape.shape_type.clone());
        }
    }
    let mut cycler = labelme_rs::ColorCycler::default();
    for color in all_labels.iter() {
        label_colors
            .entry(color.to_string())
            .or_insert_with(|| cycler.cycle().to_string());
    }

    let resize_param = match args.svg.resize {
        Some(s) => Some(labelme_rs::ResizeParam::try_from(s.as_str())?),
        None => None,
    };

    debug!("Generate svgs");
    let svgs: Result<Vec<String>> = entries
        .into_par_iter()
        .map(|entry| {
            let input = entry.0;
            let mut json_data = entry.1;

            json_data.imagePath = json_data.imagePath.replace('\\', "/");
            let image_path = json_data.imagePath.clone();
            let json_data = json_data.to_absolute_path(&json_dir);
            let mut data_w_img: LabelMeDataWImage = LabelMeDataWImage::try_from(json_data)
                .with_context(|| format!("load {}", image_path))?;

            if let Some(param) = resize_param.as_ref() {
                data_w_img.resize(param);
            }

            let flags: Vec<_> = data_w_img
                .data
                .flags
                .iter()
                .filter(|(_k, v)| **v)
                .map(|(k, _v)| k.clone())
                .collect();
            let flags = flags.join(" ");
            let label_counts = data_w_img.data.count_labels();
            let title = label_counts
                .iter()
                .map(|(k, v)| format!("{k}:{v}"))
                .collect::<Vec<_>>()
                .join("\n");
            let document = data_w_img.data.to_svg(
                &label_colors,
                args.svg.radius,
                args.svg.line_width,
                &data_w_img.image,
            );
            let mut context = tera::Context::new();
            context.insert("tags", &flags);
            context.insert("flags", &flags);
            context.insert("title", &title);
            context.insert(
                "name",
                &input
                    .file_stem()
                    .context("Failed to get file_stem")?
                    .to_string_lossy(),
            );
            context.insert("svg", &document.to_string());
            let fig = templates
                .render("img.html", &context)
                .expect("Failed to render img.html");
            let bar = shared_bar.lock().unwrap();
            bar.inc(1);
            Ok(fig)
        })
        .collect();
    let svgs = svgs?;
    {
        shared_bar.lock().unwrap().finish();
    };
    debug!("Generate html");
    let shape_toggles: std::result::Result<Vec<_>, _> = all_shapes
        .iter()
        .map(|shape| {
            let mut context = tera::Context::new();
            context.insert("shape", &shape);
            templates.render("shape_toggle.html", &context)
        })
        .collect();
    let tag_cbs: std::result::Result<Vec<_>, _> = all_tags
        .iter()
        .filter_map(|(tag, checked)| {
            if *checked {
                let mut context = tera::Context::new();
                context.insert("tag", &tag);
                Some(context)
            } else {
                None
            }
        })
        .map(|context| templates.render("tag_checkbox.html", &context))
        .collect();
    let legends: std::result::Result<Vec<_>, _> = label_colors
        .iter()
        .map(|(k, v)| {
            let mut context = tera::Context::new();
            context.insert("label", &k);
            context.insert("color", &v);
            templates.render("legend.html", &context)
        })
        .collect();
    let mut context = tera::Context::new();
    let style = if let Some(css) = args.css {
        std::fs::read_to_string(css)?
    } else {
        include_str!("templates/default.css").into()
    };
    context.insert("title", &args.title);
    context.insert("legend", &legends?.join("\n"));
    context.insert("shape_toggles", &shape_toggles?.join("\n"));
    context.insert("tag_checkboxes", &tag_cbs?.join("\n"));
    context.insert("main", &svgs.join("\n"));
    context.insert("style", &style);
    let html = templates.render("catalog.html", &context)?;
    debug!("Write html");
    let mut writer = std::io::BufWriter::new(std::fs::File::create(args.output)?);
    writer.write_all(html.as_bytes())?;
    debug!("Done");
    Ok(())
}
