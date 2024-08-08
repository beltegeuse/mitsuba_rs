extern crate env_logger;
#[macro_use]
extern crate log;

use clap::Parser;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::path::Path;
use cgmath::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_name = "FILE", required = true)]
    input: String,
    #[arg(short, long, value_name = "FILE", required = true)]
    output: String,
}



fn export_obj(scene_info: mitsuba_rs::Scene, file: &mut File, wk: &Path) {
    let mut file = BufWriter::new(file);
   
    let mut shapes = scene_info.shapes_id.values().collect::<Vec<_>>();
    shapes.extend(scene_info.shapes_unamed.iter());

    // Need to write manually the obj file
    // --- Write all uname shapes
    let mut offset_point = 1;
    for (i, shape) in shapes.into_iter().enumerate() {
        match shape {
            mitsuba_rs::Shape::Obj{
                filename, 
                option,
                ..
            } => {
                // Load the relevent data and make the transformation
                let mat = if let Some(m) = &option.to_world {
                    m.clone().as_matrix()
                } else {
                    Matrix4::identity()
                };

                // Load the obj file
                let filename = wk.join(filename);   
                debug!("Loading obj file: {}", filename.to_str().unwrap());
                let obj = tobj::load_obj(&filename.to_str().unwrap(), &tobj::LoadOptions {
                    single_index: false,
                    triangulate: true,
                    ..Default::default()
                }).unwrap();

        
                for (local_i, m) in obj.0.into_iter().enumerate() {
                    writeln!(file, "o Unamed_{}_{}", i, local_i).unwrap();
                    let mesh = &m.mesh;

                    // Write the position
                    for p in mesh.positions.chunks(3) {
                        let p = Point3::new(p[0], p[1], p[2]);
                        let p = mat.transform_point(p);
                        writeln!(file, "v {} {} {}", p.x, p.y, p.z).unwrap();
                    }
                    file.write_all(b"\n").unwrap();

                    // Write the indices
                    for i in mesh.indices.chunks(3) {
                        writeln!(file, "f {} {} {}", i[0] + offset_point, i[1] + offset_point, i[2] + offset_point).unwrap();
                    }
                    file.write_all(b"\n").unwrap();
                    offset_point += (mesh.positions.len() / 3) as u32;
                }
            },
            mitsuba_rs::Shape::Ply { filename, option, .. } => {
                // Load the relevent data and make the transformation
                let mat = if let Some(m) = &option.to_world {
                    m.clone().as_matrix()
                } else {
                    Matrix4::identity()
                };

                let filename = wk.join(filename);
                debug!("Loading ply file: {}", filename.to_str().unwrap());
                let mesh = mitsuba_rs::ply::read_ply(&filename);

                writeln!(file, "o Unamed_{}", i).unwrap();
                // Write the position
                for p in mesh.points.iter() {
                    let p = mat.transform_point(*p);
                    writeln!(file, "v {} {} {}", p.x, p.y, p.z).unwrap();
                }
                file.write_all(b"\n").unwrap();
                
                // Write the indices
                for i in mesh.indices {
                    writeln!(file, "f {} {} {}", i[0] as u32 + offset_point, i[1] as u32 + offset_point, i[2] as u32+ offset_point).unwrap();
                }
                file.write_all(b"\n").unwrap();

                offset_point += mesh.points.len() as u32;
            }
            _ => warn!("Ignoring shape {:?}", shape),
        }
    } // End shapes

}

fn main() {
    let args = Args::parse();

    env_logger::Builder::from_default_env()
        .format_timestamp(None)
        .parse_filters("info")
        .init();

    let scene = mitsuba_rs::parse(&args.input).unwrap();
    let mut file = File::create(&args.output).unwrap();
    let wk = Path::new(args.input.as_str()).parent().unwrap();
    export_obj(scene, &mut file, wk)
}