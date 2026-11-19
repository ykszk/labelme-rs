use anyhow::Result;
use labelme_rs::{serde_json, LabelMeData, LabelMeDataLine};
use lmrs::cli::MatCmdArgs as CmdArgs;
use std::fs::File;
use std::io::{stdout, BufRead, BufReader, BufWriter};

pub fn cmd(args: CmdArgs) -> Result<()> {
    // 3 x 3 matrix as an array
    let mut mat = [0.0; 9];
    // set the identity matrix
    mat[0] = 1.0;
    mat[4] = 1.0;
    mat[8] = 1.0;

    if let Some(matrix) = args.matrix {
        mat[0] = matrix[0];
        mat[1] = matrix[1];
        mat[2] = matrix[2];
        mat[3] = matrix[3];
        mat[4] = matrix[4];
        mat[5] = matrix[5];
        mat[6] = matrix[6];
        mat[7] = matrix[7];
        mat[8] = matrix[8];
    };

    if let Some(tr) = args.translate {
        mat[2] = tr[0];
        mat[5] = tr[1];
    }

    if let Some(scale) = args.scale {
        mat[0] = scale[0];
        mat[4] = scale[1];
    }

    if let Some(rot) = args.rotate {
        let cos = rot.to_radians().cos();
        let sin = rot.to_radians().sin();
        mat[0] = cos;
        mat[1] = -sin;
        mat[3] = sin;
        mat[4] = cos;
    }

    debug!("Matrix: {:?}", mat);

    if args.ndjson {
        let reader: Box<dyn BufRead> = if args.input.as_os_str() == "-" {
            Box::new(BufReader::new(std::io::stdin()))
        } else {
            Box::new(BufReader::new(File::open(&args.input)?))
        };
        for line in reader.lines() {
            let line = line?;
            let mut lm_line: LabelMeDataLine = serde_json::from_str(&line)?;
            apply_mat(mat, &mut lm_line.content)?;
            let writer = stdout();
            serde_json::to_writer(writer.lock(), &lm_line)?;
            println!();
        }
    } else {
        let mut data: LabelMeData = if args.input.as_os_str() == "-" {
            // read from stdin
            let reader = BufReader::new(std::io::stdin());
            let lm_data: LabelMeData = serde_json::from_reader(reader)?;
            lm_data
        } else {
            LabelMeData::try_from(args.input.as_path())?
        };

        apply_mat(mat, &mut data)?;

        if let Some(output) = args.output {
            let writer = BufWriter::new(File::create(output)?);
            serde_json::to_writer_pretty(writer, &data)?;
        } else {
            let writer = stdout();
            serde_json::to_writer_pretty(writer.lock(), &data)?;
        }
    }

    Ok(())
}

/// Transform the points. Masks follow translations and positive axis-aligned scaling;
/// other transformations of data with masks are an error, as a mask image cannot follow them.
fn apply_mat(mat: [f64; 9], data: &mut LabelMeData) -> Result<()> {
    let w = mat[8];
    let scale_translate = mat[1] == 0.0
        && mat[3] == 0.0
        && mat[6] == 0.0
        && mat[7] == 0.0
        && mat[0] / w > 0.0
        && mat[4] / w > 0.0;
    if scale_translate {
        for shape in &mut data.shapes {
            shape.scale_translate(mat[0] / w, mat[4] / w, mat[2] / w, mat[5] / w);
        }
        return Ok(());
    }
    anyhow::ensure!(
        data.shapes.iter().all(|s| s.mask.is_none()),
        "Cannot transform mask shapes with this matrix: only translation and positive \
         scaling are supported"
    );
    for shape in &mut data.shapes {
        for point in &mut shape.points {
            let x = point.0;
            let y = point.1;
            let new_x = mat[0] * x + mat[1] * y + mat[2];
            let new_y = mat[3] * x + mat[4] * y + mat[5];
            let w = mat[6] * x + mat[7] * y + mat[8];
            point.0 = new_x / w;
            point.1 = new_y / w;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use labelme_rs::Shape;

    fn with_mask() -> LabelMeData {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/data/Mandrill.json");
        let mut data = LabelMeData::try_from(path.as_path()).unwrap();
        data.shapes.retain(|s| s.shape_type == "mask");
        assert_eq!(data.shapes.len(), 1);
        data
    }

    fn mask_size(shape: &Shape) -> (u32, u32) {
        shape.mask_image().unwrap().dimensions()
    }

    #[test]
    fn masks_follow_translation_and_scaling() {
        let mut data = with_mask();
        let (w, h) = mask_size(&data.shapes[0]);
        let origin = data.shapes[0].points[0];
        apply_mat([2.0, 0.0, 5.0, 0.0, 3.0, -1.0, 0.0, 0.0, 1.0], &mut data).unwrap();
        let shape = &data.shapes[0];
        assert_eq!(
            shape.points[0],
            (origin.0 * 2.0 + 5.0, origin.1 * 3.0 - 1.0)
        );
        assert_eq!(mask_size(shape), (w * 2, h * 3));
    }

    #[test]
    fn masks_refuse_rotation_and_flips() {
        let rotation = [0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        let flip = [-1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        for mat in [rotation, flip] {
            let mut data = with_mask();
            let before = data.clone();
            assert!(apply_mat(mat, &mut data).is_err());
            assert_eq!(data, before, "nothing is changed");
        }
    }
}
