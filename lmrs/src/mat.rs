use anyhow::Result;
use labelme_rs::{serde_json, LabelMeData};
use lmrs::cli::MatCmdArgs as CmdArgs;
use std::fs::File;
use std::io::{stdout, BufReader, BufWriter};

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

    let mut data: LabelMeData = if args.input.as_os_str() == "-" {
        // read from stdin
        let reader = BufReader::new(std::io::stdin());
        let lm_data: LabelMeData = serde_json::from_reader(reader)?;
        lm_data
    } else {
        LabelMeData::try_from(args.input.as_path())?
    };

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

    if let Some(output) = args.output {
        let writer = BufWriter::new(File::create(output)?);
        serde_json::to_writer_pretty(writer, &data)?;
    } else {
        let writer = stdout();
        serde_json::to_writer_pretty(writer.lock(), &data)?;
    }

    Ok(())
}
