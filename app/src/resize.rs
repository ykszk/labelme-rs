use anyhow::{ensure, Context, Result};
use lmrs::cli::ResizeCmdArgs as CmdArgs;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn resize_scale(width: u32, height: u32, nwidth: u32, nheight: u32) -> f64 {
    let wratio = nwidth as f64 / width as f64;
    let hratio = nheight as f64 / height as f64;

    f64::min(wratio, hratio)
}

pub fn cmd(args: CmdArgs) -> Result<()> {
    let reader: Box<dyn BufRead> = if args.input.as_os_str() == "-" {
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        Box::new(BufReader::new(File::open(&args.input)?))
    };
    for line in reader.lines() {
        let line = line?;
        let resize_param = lmrs::ResizeParam::try_from(args.param.as_str())?;
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
        let scale = match resize_param {
            lmrs::ResizeParam::Percentage(p) => p,
            lmrs::ResizeParam::Size(w, h) => resize_scale(orig_width, orig_height, w, h),
        };
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

#[test]
fn test_resize() -> anyhow::Result<()> {
    let scale = resize_scale(100, 100, 50, 10);
    assert_eq!(scale, 0.1);
    let scale = resize_scale(100, 100, 10, 50);
    assert_eq!(scale, 0.1);
    let scale = resize_scale(100, 100, 1000, 200);
    assert_eq!(scale, 2.0);
    Ok(())
}
