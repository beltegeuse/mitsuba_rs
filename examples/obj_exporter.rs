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


fn export_obj(scene_info: mitsuba_rs::Scene, file: &mut File, mat_file: &mut File) {
    let mut file = BufWriter::new(file);
    let mut mat_file = BufWriter::new(mat_file);

    let normalize_rgb = |rgb: &mut pbrt_rs::parser::RGB| {
        let max = rgb.r.max(rgb.b.max(rgb.g));
        if max > 1.0 {
            rgb.r /= max;
            rgb.g /= max;
            rgb.b /= max;
        }
    };

    let default_mat = |f: &mut BufWriter<&mut File>| {
        writeln!(f, "Ns 1.0").unwrap();
        writeln!(f, "Ka 1.000000 1.000000 1.000000").unwrap();
        writeln!(f, "Kd 0.8 0.8 0.8").unwrap();
        writeln!(f, "Ke 0.000000 0.000000 0.000000").unwrap();
        writeln!(f, "Ni 1.000000").unwrap();
        writeln!(f, "d 1.000000").unwrap();
        writeln!(f, "illum 1").unwrap();
    };
    let emission_mat = |id_light: u32,
                        shape_name: String,
                        shape_emission: &Option<pbrt_rs::parser::Spectrum>,
                        f_obj: &mut BufWriter<&mut File>,
                        f_mat: &mut BufWriter<&mut File>| {
        info!("Exporting emission:");
        info!(" - shape_name: {}", shape_name);

        match shape_emission {
            Some(pbrt_rs::parser::Spectrum::RGB(ref rgb)) => {
                info!(" - emission: [{}, {}, {}]", rgb.r, rgb.g, rgb.b);
                writeln!(f_obj, "usemtl light_{}", id_light).unwrap();
                // Write the material file because the light is special materials
                writeln!(f_mat, "newmtl light_{}", id_light).unwrap();
                writeln!(f_mat, "Ns 0.0").unwrap();
                writeln!(f_mat, "Ka 0.000000 0.000000 0.000000").unwrap();
                writeln!(f_mat, "Kd 0.0 0.0 0.0").unwrap();
                writeln!(f_mat, "Ke {} {} {}", rgb.r, rgb.g, rgb.b).unwrap();
                writeln!(f_mat, "Ni 0.000000").unwrap();
                writeln!(f_mat, "d 1.000000").unwrap();
                writeln!(f_mat, "illum 7").unwrap();
                f_mat.write_all(b"\n").unwrap();
            }
            _ => panic!("No support for this emission profile"),
        }
    };

    {
        // Write default material
        writeln!(mat_file, "newmtl export_default").unwrap();
        default_mat(&mut mat_file);
        mat_file.write_all(b"\n").unwrap();
    }

    // Need to write manually the obj file
    // --- Write all uname shapes
    let mut offset_point = 1;
    let mut offset_normal = 1;
    let mut offset_uv = 1;
    let mut nb_light = 0;
    for (i, shape) in scene_info.shapes.into_iter().enumerate() {
        let material_name = shape.material_name.clone();
        let shape_emission = shape.emission;
        match shape.data {
            pbrt_rs::Shape::TriMesh {
                indices,
                points,
                uv,
                normals,
            } => {
                // Load the relevent data and make the transformation
                let mat = shape.matrix;
                let uv = if let Some(uv) = uv { uv } else { vec![] };
                let normals = match normals {
                    Some(ref v) => v
                        .into_iter()
                        .map(|n| mat.transform_vector(n.clone()))
                        .collect(),
                    None => Vec::new(),
                };
                let points = points
                    .into_iter()
                    .map(|n| mat.transform_point(n.clone()))
                    .collect::<Vec<Point3<f32>>>();

                // We only support trianbles, so it is much easier
                // Moreover, normal, uv are aligned
                writeln!(file, "o Unamed_{}", i).unwrap();
                // --- Geometry
                let mut number_channels = 1;
                for p in &points {
                    writeln!(file, "v {} {} {}", p.x, p.y, p.z).unwrap();
                }
                file.write_all(b"\n").unwrap();
                if !uv.is_empty() {
                    number_channels += 1;
                    for t in &uv {
                        writeln!(file, "vt {} {}", t.x, t.y).unwrap();
                    }
                    file.write_all(b"\n").unwrap();
                }
                if !normals.is_empty() {
                    number_channels += 1;
                    for n in &normals {
                        writeln!(file, "vn {} {} {}", n.x, n.y, n.z).unwrap();
                    }
                    file.write_all(b"\n").unwrap();
                }

                // --- Write material
                match material_name {
                    None => {
                        if shape_emission.is_none() {
                            writeln!(file, "usemtl export_default").unwrap();
                        } else {
                            emission_mat(
                                nb_light,
                                format!("Unamed_{}", i),
                                &shape_emission,
                                &mut file,
                                &mut mat_file,
                            );
                            nb_light += 1;
                        }
                    }
                    Some(ref m) => {
                        if shape_emission.is_none() {
                            writeln!(file, "usemtl {}", m).unwrap();
                        } else {
                            warn!("Overwrite materials as it is a light");
                            emission_mat(
                                nb_light,
                                format!("Unamed_{}", i),
                                &shape_emission,
                                &mut file,
                                &mut mat_file,
                            );
                            nb_light += 1;
                        }
                    }
                };
                for index in indices {
                    let i1 = index.x;
                    let i2 = index.y;
                    let i3 = index.z;

                    match number_channels {
                        1 => writeln!(
                            file,
                            "f {} {} {}",
                            i1 + offset_point,
                            i2 + offset_point,
                            i3 + offset_point
                        )
                        .unwrap(),
                        2 => {
                            if normals.is_empty() {
                                writeln!(
                                    file,
                                    "f {}/{} {}/{} {}/{}",
                                    i1 + offset_point,
                                    i1 + offset_uv,
                                    i2 + offset_point,
                                    i2 + offset_uv,
                                    i3 + offset_point,
                                    i3 + offset_uv
                                )
                                .unwrap();
                            } else {
                                writeln!(
                                    file,
                                    "f {}//{} {}//{} {}//{}",
                                    i1 + offset_point,
                                    i1 + offset_normal,
                                    i2 + offset_point,
                                    i2 + offset_normal,
                                    i3 + offset_point,
                                    i3 + offset_normal
                                )
                                .unwrap();
                            }
                        }
                        3 => writeln!(
                            file,
                            "f {}/{}/{} {}/{}/{} {}/{}/{}",
                            i1 + offset_point,
                            i1 + offset_uv,
                            i1 + offset_normal,
                            i2 + offset_point,
                            i2 + offset_uv,
                            i2 + offset_normal,
                            i3 + offset_point,
                            i3 + offset_uv,
                            i3 + offset_normal
                        )
                        .unwrap(),
                        _ => panic!("Unsupported number of channels"),
                    }
                }
                file.write_all(b"\n").unwrap();
                offset_point += points.len();
                offset_normal += normals.len();
                offset_uv += uv.len();
            }
            _ => panic!("All meshes need to be converted to trimesh!"),
        }
    } // End shapes

    // Export the materials
    let mut textures = vec![];
    info!("Exporting bsdfs...");
    for (name, bdsf) in scene_info.materials.iter() {
        info!(" - {}", name);
        writeln!(mat_file, "newmtl {}", name).unwrap();
        match bdsf {
            pbrt_rs::BSDF::Matte { kd, .. } => {
                writeln!(mat_file, "Ns 1.0").unwrap();
                writeln!(mat_file, "Ka 0.0 0.0 0.0").unwrap();
                writeln!(mat_file, "Tf 1.0 1.0 1.0").unwrap();
                writeln!(mat_file, "Ks 0.0 0.0 0.0").unwrap();
                writeln!(mat_file, "illum 4").unwrap();
                match kd {
                    pbrt_rs::parser::Spectrum::RGB(ref rgb) => {
                        let mut rgb = rgb.clone();
                        normalize_rgb(&mut rgb);
                        writeln!(mat_file, "Kd {} {} {}", rgb.r, rgb.g, rgb.b).unwrap()
                    }
                    pbrt_rs::parser::Spectrum::Texture(ref tex_name) => {
                        writeln!(mat_file, "Kd 0.0 0.0 0.0").unwrap();
                        let texture = &scene_info.textures[tex_name];
                        warn!(" - Texture file: {}", texture.filename);
                        writeln!(mat_file, "map_Kd {}", texture.filename).unwrap();
                        textures.push(texture.filename.clone());
                    }
                    _ => panic!("Unsupported texture for matte material"),
                }
            }
            pbrt_rs::BSDF::Glass { .. } => {
                writeln!(mat_file, "Ns 1000").unwrap();
                writeln!(mat_file, "Ka 0.0 0.0 0.0").unwrap();
                writeln!(mat_file, "Kd 0.0 0.0 0.0").unwrap();
                writeln!(mat_file, "Tf 0.1 0.1 0.1").unwrap();
                writeln!(mat_file, "Ks 0.5 0.5 0.5").unwrap();
                writeln!(mat_file, "Ni 1.31").unwrap(); // Glass
                writeln!(mat_file, "d 1.000000").unwrap();
                writeln!(mat_file, "illum 7").unwrap();
                // TODO: Read the properties
            }
            pbrt_rs::BSDF::Mirror { kr, .. } => {
                writeln!(mat_file, "Ns 100000.0").unwrap();
                writeln!(mat_file, "Ka 0.0 0.0 0.0").unwrap();
                writeln!(mat_file, "Kd 0.0 0.0 0.0").unwrap();
                writeln!(mat_file, "Tf 1.0 1.0 1.0").unwrap();
                writeln!(mat_file, "Ni 1.00").unwrap();
                writeln!(mat_file, "illum 3").unwrap();
                match kr {
                    pbrt_rs::parser::Spectrum::RGB(ref rgb) => {
                        let mut rgb = rgb.clone();
                        normalize_rgb(&mut rgb);
                        writeln!(mat_file, "Ks {} {} {}", rgb.r, rgb.g, rgb.b).unwrap()
                    }
                    _ => panic!("Unsupported texture for mirror material"),
                }
            }
            pbrt_rs::BSDF::Substrate { ks, kd, .. } => {
                writeln!(mat_file, "Ka 0.0 0.0 0.0").unwrap();
                writeln!(mat_file, "Tf 1.0 1.0 1.0").unwrap();
                writeln!(mat_file, "Ni 1.0").unwrap();
                writeln!(mat_file, "illum 4").unwrap();
                match ks {
                    pbrt_rs::parser::Spectrum::RGB(ref rgb) => {
                        let mut rgb = rgb.clone();
                        normalize_rgb(&mut rgb);
                        writeln!(mat_file, "Ks {} {} {}", rgb.r, rgb.g, rgb.b).unwrap()
                    }
                    pbrt_rs::parser::Spectrum::Texture(ref tex_name) => {
                        writeln!(mat_file, "Ks 0.0 0.0 0.0").unwrap();
                        let texture = &scene_info.textures[tex_name];
                        warn!(" - Texture file: {}", texture.filename);
                        writeln!(mat_file, "map_Ks {}", texture.filename).unwrap();
                        textures.push(texture.filename.clone());
                    }
                    _ => panic!("Unsupported texture for metal material"),
                }
                warn!("Rougness conversion is broken");
                writeln!(mat_file, "Ns {}", 0.1).unwrap();
                // match distribution.roughness {
                //     pbrt_rs::Param::Float(ref v) => {
                //         // TODO: Need a conversion formula for phong
                //         writeln!(mat_file, "Ns {}", 2.0 / v[0]).unwrap();
                //         info!("Found roughness: {}", 2.0 / v[0]);
                //     }
                //     _ => panic!("Unsupported texture for metal material"),
                // }
                match kd {
                    pbrt_rs::parser::Spectrum::RGB(ref rgb) => {
                        let mut rgb = rgb.clone();
                        normalize_rgb(&mut rgb);
                        writeln!(mat_file, "Kd {} {} {}", rgb.r, rgb.g, rgb.b).unwrap()
                    }
                    pbrt_rs::parser::Spectrum::Texture(ref tex_name) => {
                        writeln!(mat_file, "Kd 0.0 0.0 0.0").unwrap();
                        let texture = &scene_info.textures[tex_name];
                        warn!(" - Texture file: {}", texture.filename);
                        writeln!(mat_file, "map_Kd {}", texture.filename).unwrap();
                        textures.push(texture.filename.clone());
                    }
                    _ => panic!("Unsupported texture for metal material"),
                }
            }
            pbrt_rs::BSDF::Metal { k, .. } => {
                writeln!(mat_file, "Ka 0.0 0.0 0.0").unwrap();
                writeln!(mat_file, "Kd 0.0 0.0 0.0").unwrap();
                writeln!(mat_file, "Tf 1.0 1.0 1.0").unwrap();
                writeln!(mat_file, "Ni 1.00").unwrap();
                writeln!(mat_file, "illum 3").unwrap();
                match k {
                    pbrt_rs::parser::Spectrum::RGB(ref rgb) => {
                        let mut rgb = rgb.clone();
                        normalize_rgb(&mut rgb);
                        writeln!(mat_file, "Ks {} {} {}", rgb.r, rgb.g, rgb.b).unwrap()
                    }
                    pbrt_rs::parser::Spectrum::Texture(ref tex_name) => {
                        writeln!(mat_file, "Ks 0.0 0.0 0.0").unwrap();
                        let texture = &scene_info.textures[tex_name];
                        warn!(" - Texture file: {}", texture.filename);
                        writeln!(mat_file, "map_Ks {}", texture.filename).unwrap();
                        textures.push(texture.filename.clone());
                    }
                    _ => panic!("Unsupported texture for metal material"),
                }
                warn!("Rougness conversion is broken");
                writeln!(mat_file, "Ns {}", 0.1).unwrap();
                // match metal.roughness {
                //     pbrt_rs::Param::Float(ref v) => {
                //         // TODO: Need a conversion formula for phong
                //         writeln!(mat_file, "Ns {}", 2.0 / v[0]).unwrap();
                //         info!("Found roughness: {}", 2.0 / v[0]);
                //     }
                //     _ => panic!("Unsupported texture for metal material"),
                // }
            }
        }
        mat_file.write_all(b"\n").unwrap();
    } // End of materials

    info!("Number of textures detected: {}", textures.len());
    for tex in &textures {
        info!(" - {}", tex);
    }
}

fn main() {
    let args = Args::parse();

    env_logger::Builder::from_default_env()
        .format_timestamp(None)
        .parse_filters("info")
        .init();

    let scene = mitsuba_rs::parse(&args.input).unwrap();
    for (k, v) in &scene.shapes_id {
        println!("{}: {:?}", k, v);
    }
    for v in &scene.shapes_unamed {
        println!("{:?}", v);
    }
}