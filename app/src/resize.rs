use anyhow::{Context, Result};
use labelme_rs::{serde_json, LabelMeDataLine, ResizeParam};
use lmrs::cli::ResizeCmdArgs as CmdArgs;
use std::fs::File;
use std::io::{stdout, BufRead, BufReader, BufWriter};
use std::path::PathBuf;

pub fn cmd(args: CmdArgs) -> Result<()> {
    let reader: Box<dyn BufRead> = if args.input.as_os_str() == "-" {
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        Box::new(BufReader::new(File::open(&args.input)?))
    };
    let resize_param = ResizeParam::try_from(args.param.as_str())?;
    for line in reader.lines() {
        let line = line?;
        let mut lm_line: LabelMeDataLine = line.as_str().try_into()?;
        let scale = resize_param.scale(
            lm_line.content.imageWidth as u32,
            lm_line.content.imageHeight as u32,
        );
        lm_line.content.scale(scale);
        let writer = BufWriter::new(stdout().lock());
        serde_json::to_writer(writer, &lm_line)?;
        println!();
        if let Some(ref image_dir) = args.image {
            let image_path = PathBuf::from(&lm_line.content.imagePath);
            let mut data_w_image: labelme_rs::LabelMeDataWImage = lm_line
                .content
                .try_into()
                .with_context(|| format!("Opening {:?}", image_path))?;
            data_w_image.resize(&resize_param);
            let outname = image_dir.join(image_path.file_name().unwrap());
            data_w_image.image.save(outname)?;
        }
    }
    Ok(())
}
