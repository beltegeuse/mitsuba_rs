use crate::SerializedShape;

use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek};

use byteorder::{LittleEndian, ReadBytesExt};

use cgmath::*;

bitflags! {
    struct Flags: u32 {
        const HAS_NORMALS =     0x0001;
        const HAS_TEXCOORDS =   0x0002;
        const HAS_TANGENTS =    0x0004;
        const HAS_COLORS =      0x0008;
        const HAS_FACE_NORMAL = 0x0010;
        const SINGLE_PRECISION = 0x1000;
        const DOUBLE_PRECISION = 0x2000;
    }
}

pub struct Serialized {
    pub name: String,
    pub vertices: Vec<Vector3<f32>>,
    pub normals: Option<Vec<Vector3<f32>>>,
    pub texcoords: Option<Vec<Vector2<f32>>>,
    pub color: Option<Vec<Vector3<f32>>>,
    pub indices: Vec<Vector3<usize>>,
    pub face_normal: bool,
}

pub fn read_serialized(s: &SerializedShape, wk: &std::path::Path) -> Serialized {
    let filename = wk.join(std::path::Path::new(&s.filename));

    // Start to read the file
    let f = File::open(filename).unwrap();
    let mut f = BufReader::new(f);

    // Go to the end file
    let _pos_nb_meshes = f.seek(std::io::SeekFrom::End(-4)).unwrap();
    let nb_meshes = f.read_u32::<LittleEndian>().unwrap();
    //println!("nb meshes: {}", nb_meshes);

    // Load offsets
    let pos_offset = f
        .seek(std::io::SeekFrom::Current(-8 * nb_meshes as i64 - 4))
        .unwrap();
    let offsets = (0..nb_meshes)
        .map(|_| f.read_u64::<LittleEndian>().unwrap())
        .collect::<Vec<_>>();
    //println!("offsets: {:?}", offsets);

    // Read all the portion into memory
    let mesh_size = if s.shape_index == nb_meshes - 1 {
        pos_offset - offsets[s.shape_index as usize]
    } else {
        offsets[s.shape_index as usize + 1] - offsets[s.shape_index as usize]
    };
    let _pos_mesh = f
        .seek(std::io::SeekFrom::Start(
            offsets[s.shape_index as usize] as u64,
        ))
        .unwrap();
    let _id_format = f.read_u16::<LittleEndian>().unwrap();
    let id_file = f.read_u16::<LittleEndian>().unwrap();
    assert_eq!(id_file, 4);
    //println!("id_format: {:#x} | id_file: {:#x}", id_format, id_file);

    // Read the whole mesh (need to decompress after that)
    let buffer = {
        let mut buffer = vec![0u8; mesh_size as usize - 4];
        f.read_exact(buffer.as_mut()).unwrap();
        miniz_oxide::inflate::decompress_to_vec_zlib(&mut buffer).unwrap()
    };
    // println!("Compressed: {}", mesh_size);
    // println!("Uncompressed: {}", buffer.len());

    let mut f = Cursor::new(buffer);
    let flag = f.read_u32::<LittleEndian>().unwrap();
    let flag = Flags::from_bits(flag).unwrap();
    let name = {
        let mut buf_name = vec![];
        // This part can be dangerous!
        buf_name.push(f.read_u8().unwrap());
        while *buf_name.last().unwrap() != 0 {
            buf_name.push(f.read_u8().unwrap());
        }
        std::string::String::from_utf8_lossy(&buf_name).into_owned()
    };
    // println!("name: '{}'", name);

    let nb_vertices = f.read_u64::<LittleEndian>().unwrap();
    let nb_tri = f.read_u64::<LittleEndian>().unwrap();
    // println!("vertices: {} | nb_tri {}", nb_vertices, nb_tri);

    // Information about the shape
    let single_precision = if flag.intersects(Flags::SINGLE_PRECISION) {
        true
    } else {
        assert!(flag.intersects(Flags::DOUBLE_PRECISION));
        false
    };
    // println!("{:?}", flag);

    // Downscale the precision for the moment
    // TODO: Do templated version to not bound the precision
    //  from Vec precision
    let read_float = |f: &mut Cursor<Vec<u8>>| -> f32 {
        if single_precision {
            f.read_f32::<LittleEndian>().unwrap()
        } else {
            f.read_f64::<LittleEndian>().unwrap() as f32
        }
    };

    let vertices = (0..nb_vertices)
        .map(|_| {
            let x = read_float(&mut f);
            let y = read_float(&mut f);
            let z = read_float(&mut f);
            Vector3::new(x, y, z)
        })
        .collect();

    let normals = if flag.intersects(Flags::HAS_NORMALS) {
        Some(
            (0..nb_vertices)
                .map(|_| {
                    let x = read_float(&mut f);
                    let y = read_float(&mut f);
                    let z = read_float(&mut f);
                    Vector3::new(x, y, z)
                })
                .collect(),
        )
    } else {
        None
    };

    let texcoords = if flag.intersects(Flags::HAS_TEXCOORDS) {
        Some(
            (0..nb_vertices)
                .map(|_| {
                    let u = read_float(&mut f);
                    let v = read_float(&mut f);
                    Vector2::new(u, v)
                })
                .collect(),
        )
    } else {
        None
    };

    let color = if flag.intersects(Flags::HAS_COLORS) {
        Some(
            (0..nb_vertices)
                .map(|_| {
                    let x = read_float(&mut f);
                    let y = read_float(&mut f);
                    let z = read_float(&mut f);
                    Vector3::new(x, y, z)
                })
                .collect(),
        )
    } else {
        None
    };

    let indices = if nb_tri > std::u32::MAX as u64 {
        // use u64
        (0..nb_tri)
            .map(|_| {
                let x = f.read_u64::<LittleEndian>().unwrap() as usize;
                let y = f.read_u64::<LittleEndian>().unwrap() as usize;
                let z = f.read_u64::<LittleEndian>().unwrap() as usize;
                Vector3::new(x, y, z)
            })
            .collect()
    } else {
        // use u32
        (0..nb_tri)
            .map(|_| {
                let x = f.read_u32::<LittleEndian>().unwrap() as usize;
                let y = f.read_u32::<LittleEndian>().unwrap() as usize;
                let z = f.read_u32::<LittleEndian>().unwrap() as usize;
                Vector3::new(x, y, z)
            })
            .collect()
    };

    Serialized {
        name,
        vertices,
        normals,
        texcoords,
        color,
        indices,
        face_normal: flag.intersects(Flags::HAS_FACE_NORMAL),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn kitchen_serialized() {
        let s = crate::SerializedShape {
            filename: "./data/kitchen.serialized".to_string(),
            shape_index: 0,
            face_normal: true,
            max_smooth_angle: None,
            option: crate::ShapeOption {
                flip_normal: false,
                bsdf: None,
                to_world: None,
                emitter: None,
            },
        };
        let wk = std::path::Path::new(".");
        crate::serialized::read_serialized(&s, wk);
    }

    #[test]
    fn kitchen_serialized_all() {
        for shape_index in 0..1489 {
            let s = crate::SerializedShape {
                filename: "./data/kitchen.serialized".to_string(),
                shape_index: shape_index,
                face_normal: true,
                max_smooth_angle: None,
                option: crate::ShapeOption {
                    flip_normal: false,
                    bsdf: None,
                    to_world: None,
                    emitter: None,
                },
            };
            let wk = std::path::Path::new(".");
            crate::serialized::read_serialized(&s, wk);
        }
    }
}
