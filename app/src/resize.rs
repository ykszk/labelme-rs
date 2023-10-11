use anyhow::{ensure, Context, Result};
use labelme_rs::ResizeParam;
use lmrs::cli::ResizeCmdArgs as CmdArgs;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn cmd(args: CmdArgs) -> Result<()> {
    let reader: Box<dyn BufRead> = if args.input.as_os_str() == "-" {
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        Box::new(BufReader::new(File::open(&args.input)?))
    };
    for line in reader.lines() {
        let line = line?;
        let resize_param = ResizeParam::try_from(args.param.as_str())?;
        let mut obj = jzon::parse(&line)?;
        let orig_width = obj
            .get("imageWidth")
            .context("imageWidth field not found.")?
            .as_u32()
            .unwrap();
        let orig_height = obj
            .get("imageHeight")
            .context("imageHeight field not found.")?
            .as_u32()
            .unwrap();
        let scale = resize_param.scale(orig_width, orig_height);
        for shape in obj
            .get_mut("shapes")
            .context("`shapes` field not found.")?
            .as_array_mut()
            .context("`shaped` field is not an array")?
            .iter_mut()
        {
            let points = shape
                .get_mut("points")
                .context("`points` field not found in shapes")?
                .as_array_mut()
                .context("`points` field is not an array")?;
            for point in points {
                let p = point
                    .as_array_mut()
                    .context("points is in invalid format")?;
                ensure!(p.len() == 2, "The number of points is not 2");
                p[0] = jzon::JsonValue::from(scale * p[0].as_f64().context("Invalid point value")?);
                p[1] = jzon::JsonValue::from(scale * p[1].as_f64().context("Invalid point value")?);
            }
        }
        println!("{}", obj);
    }
    Ok(())
}
