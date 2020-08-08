extern crate cgmath;
extern crate xml;
#[macro_use]
extern crate lazy_static;
use std::fs::File;
use std::io::BufReader;

use cgmath::*;
use std::collections::HashMap;
use std::io::Read;
use xml::reader::{EventReader, Events, XmlEvent};

lazy_static! {
    static ref IOR_DATA: HashMap<String, f32> = {
        let mut m = HashMap::new();
        m.insert("vacuum".to_string(), 1.0);
        m.insert("helium".to_string(), 1.000036);
        m.insert("hydrogen".to_string(), 1.000132);
        m.insert("air".to_string(), 1.000277);
        m.insert("carbon dioxide".to_string(), 1.00045);
        m.insert("water".to_string(), 1.3330);
        m.insert("acetone".to_string(), 1.36);
        m.insert("ethanol".to_string(), 1.361);
        m.insert("carbon tetrachloride".to_string(), 1.461);
        m.insert("glycerol".to_string(), 1.4729);
        m.insert("benzene".to_string(), 1.501);
        m.insert("silicone oil".to_string(), 1.52045);
        m.insert("bromine".to_string(), 1.661);
        m.insert("water ice".to_string(), 1.31);
        m.insert("fused quartz".to_string(), 1.458);
        m.insert("pyrex".to_string(), 1.470);
        m.insert("acrylic glass".to_string(), 1.49);
        m.insert("polypropylene".to_string(), 1.49);
        m.insert("bk7".to_string(), 1.5046);
        m.insert("sodium chloride".to_string(), 1.544);
        m.insert("amber".to_string(), 1.55);
        m.insert("pet".to_string(), 1.5750);
        m.insert("diamond".to_string(), 2.419);
        m
    };
}

#[derive(Debug, Clone)]
pub struct Spectrum {
    pub value: String,
}
impl Spectrum {
    fn from_f32(v: f32) -> Self {
        Self {
            value: format!("{}", v),
        }
    }

    fn from_rgb(s: String) -> Self {
        Self { value: s }
    }

    // Implement material parsing
    // fn from_material(n: &str) -> (Self, Self) {
    //     todo!()
    // }
}

#[derive(Debug, PartialEq, Clone)]
pub enum BSDFColor<T> {
    Texture(Texture),
    Constant(T),
}
type BSDFColorSpectrum = BSDFColor<Spectrum>;
type BSDFColorFloat = BSDFColor<f32>;

#[derive(Debug)]
pub enum Value {
    Float(f32),
    Spectrum(Spectrum),
    String(String),
    Integer(i32),
    Boolean(bool),
    Ref(String),
}
impl Value {
    pub fn as_string(self) -> String {
        match self {
            Value::Ref(s) | Value::String(s) => s,
            _ => panic!("Wrong type {:?} (as_string)", self),
        }
    }
    pub fn as_float(self) -> f32 {
        match self {
            Value::Float(s) => s,
            _ => panic!("Wrong type {:?} (as_float)", self),
        }
    }
    pub fn as_int(self) -> i32 {
        match self {
            Value::Integer(s) => s,
            _ => panic!("Wrong type {:?} (as_int)", self),
        }
    }
    pub fn as_bool(self) -> bool {
        match self {
            Value::Boolean(s) => s,
            _ => panic!("Wrong type {:?} (as_bool)", self),
        }
    }
    pub fn as_spectrum(self) -> Spectrum {
        match self {
            Value::Spectrum(s) => s,
            _ => panic!("Wrong type {:?} (as_spectrum)", self),
        }
    }

    // Check if it is a ref
    pub fn is_ref(&self) -> bool {
        match self {
            Value::Ref(_) => true,
            _ => false,
        }
    }

    // Conversions
    pub fn as_bsdf_color_spec(self, scene: &Scene) -> BSDFColorSpectrum {
        match self {
            Value::Spectrum(v) => BSDFColorSpectrum::Constant(v),
            Value::Ref(v) => {
                let tex = scene.textures.get(&v).unwrap();
                BSDFColorSpectrum::Texture(tex.clone())
            }
            _ => panic!("Wrong type {:?} (as_bsdf_color_spec)", self),
        }
    }
    pub fn as_bsdf_color_f32(self, scene: &Scene) -> BSDFColorFloat {
        match self {
            Value::Float(v) => BSDFColorFloat::Constant(v),
            Value::Ref(v) => {
                let tex = scene.textures.get(&v).unwrap();
                BSDFColorFloat::Texture(tex.clone())
            }
            _ => panic!("Wrong type {:?} (as_bsdf_color_f32)", self),
        }
    }

    fn as_ior(self) -> f32 {
        match self {
            Value::Float(v) => v,
            Value::String(v) => *IOR_DATA.get(&v).unwrap(),
            _ => panic!("Wrong type {:?} (as_ior)", self),
        }
    }
}

fn found_attrib(attrs: &Vec<xml::attribute::OwnedAttribute>, name: &str) -> Option<String> {
    for a in attrs {
        if a.name.local_name == name {
            return Some(a.value.clone());
        }
    }
    None
}
fn found_attrib_or(
    attrs: &Vec<xml::attribute::OwnedAttribute>,
    name: &str,
    default: &str,
) -> String {
    match found_attrib(attrs, name) {
        Some(v) => v,
        None => default.to_string(),
    }
}
fn found_attrib_vec(
    attrs: &Vec<xml::attribute::OwnedAttribute>,
    name: &str,
) -> Option<Vector3<f32>> {
    match found_attrib(attrs, name) {
        Some(v) => {
            let v = v
                .split(",")
                .into_iter()
                .map(|v| v.trim().parse::<f32>().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(v.len(), 3);
            Some(Vector3::new(v[0], v[1], v[2]))
        }
        None => None,
    }
}

fn skipping_entry<R: Read>(events: &mut Events<R>) {
    // We will skip the entry
    let mut opened = 1;
    for e in events {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                println!("[WARN] skipping_entry {}", name);
                opened += 1;
            }
            Ok(XmlEvent::EndElement { .. }) => {
                if opened == 1 {
                    return;
                }
                opened -= 1;
            }
            Err(e) => {
                panic!("Parse values Error: {}", e);
            }
            _ => {}
        }
    }
}

fn values_fn<R: Read, F>(
    events: &mut Events<R>,
    strict: bool,
    mut other: F,
) -> HashMap<String, Value>
where
    F: FnMut(&mut Events<R>, &str, HashMap<String, String>) -> bool,
{
    let mut map = HashMap::new();
    let mut opened = false;
    let iter = events.into_iter();
    loop {
        let e = match iter.next() {
            Some(v) => v,
            None => panic!("Next empty"),
        };

        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => match name.local_name.as_str() {
                "float" => {
                    let name = found_attrib(&attributes, "name").unwrap();
                    let value = found_attrib(&attributes, "value").unwrap();
                    let value = value.parse::<f32>().unwrap();
                    map.insert(name, Value::Float(value));
                    opened = true;
                }
                "integer" => {
                    let name = found_attrib(&attributes, "name").unwrap();
                    let value = found_attrib(&attributes, "value").unwrap();
                    let value = value.parse::<i32>().unwrap();
                    map.insert(name, Value::Integer(value));
                    opened = true;
                }
                "boolean" => {
                    let name = found_attrib(&attributes, "name").unwrap();
                    let value = found_attrib(&attributes, "value").unwrap();
                    let value = value == "true";
                    map.insert(name, Value::Boolean(value));
                    opened = true;
                }
                "spectrum" => {
                    let name = found_attrib(&attributes, "name").unwrap();
                    let value = found_attrib(&attributes, "value").unwrap();
                    map.insert(name, Value::Spectrum(Spectrum { value }));
                    opened = true;
                }
                "rgb" => {
                    let name = found_attrib(&attributes, "name").unwrap();
                    let value = found_attrib(&attributes, "value").unwrap();
                    map.insert(name, Value::Spectrum(Spectrum::from_rgb(value)));
                    opened = true;
                }
                "ref" => {
                    let name = found_attrib(&attributes, "name").unwrap();
                    let value = found_attrib(&attributes, "id").unwrap();
                    map.insert(name, Value::Ref(value));
                    opened = true;
                }
                "string" => {
                    let name = found_attrib(&attributes, "name").unwrap();
                    let value = found_attrib(&attributes, "value").unwrap();
                    map.insert(name, Value::String(value));
                    opened = true;
                }
                _ => {
                    // TODO: Might be inefficient
                    let map = attributes
                        .iter()
                        .map(|a| (a.name.local_name.clone(), a.value.clone()))
                        .collect();
                    let captured = other(iter, &name.local_name, map);
                    if !captured {
                        if strict {
                            panic!("{:?} encounter when parsing values", name)
                        } else {
                            println!("[WARN] {:?} is skipped", name);
                            skipping_entry(iter);
                        }
                    }
                }
            },
            Ok(XmlEvent::EndElement { .. }) => {
                if !opened {
                    return map;
                }
                opened = false;
            }
            Ok(XmlEvent::Whitespace(_)) => {}
            Err(e) => {
                panic!("Parse values Error: {}", e);
            }
            _ => {
                panic!("{:?}", e);
            }
        }
    }
}

fn values<R: Read>(events: &mut Events<R>, strict: bool) -> HashMap<String, Value> {
    let f = |_: &mut Events<R>, _: &str, _: HashMap<String, String>| false;
    values_fn(events, strict, f)
}

fn read_value(m: &mut HashMap<String, Value>, n: &str, d: Value) -> Value {
    match m.remove(n) {
        Some(v) => v,
        None => d,
    }
}

#[derive(Debug, Clone)]
pub enum Alpha {
    Isotropic(BSDFColorFloat),
    Anisotropic {
        u: BSDFColorFloat,
        v: BSDFColorFloat,
    },
}

#[derive(Debug, Clone)]
pub struct Distribution {
    pub distribution: String,
    pub alpha: Alpha,
}
impl Distribution {
    fn parse(map: &mut HashMap<String, Value>, scene: &Scene) -> Self {
        let distribution =
            read_value(map, "distribution", Value::String("beckmann".to_string())).as_string();
        let alpha = {
            let is_alpha = map.get("alpha").is_some();
            if is_alpha {
                let alpha = map.remove("alpha").unwrap().as_bsdf_color_f32(scene);
                Alpha::Isotropic(alpha)
            } else {
                let u = read_value(map, "alpha_u", Value::Float(0.1)).as_bsdf_color_f32(scene);
                let v = read_value(map, "alpha_v", Value::Float(0.1)).as_bsdf_color_f32(scene);
                if u == v {
                    Alpha::Isotropic(u)
                } else {
                    Alpha::Anisotropic { u, v }
                }
            }
        };
        Self {
            distribution,
            alpha,
        }
    }
}

#[derive(Debug, Clone)]
pub enum BSDF {
    Phong {
        exponent: BSDFColorFloat,
        specular_reflectance: BSDFColorSpectrum,
        diffuse_reflectance: BSDFColorSpectrum,
    },
    Diffuse {
        reflectance: BSDFColorSpectrum,
    },
    Roughtdiffuse {
        relectance: BSDFColorSpectrum, // s(0.5)
        alpha: BSDFColorSpectrum,      // s(0.2)
        use_fast_approx: bool,         // false
    },
    Conductor {
        distribution: Option<Distribution>,
        // Potentially read values from materials
        eta: Spectrum,
        k: Spectrum,
        // Other
        ext_eta: f32,                            // Air
        specular_reflectance: BSDFColorSpectrum, // s(1.0)
    },
    Dielectric {
        distribution: Option<Distribution>,
        int_ior: f32,                              // intIOR "bk7"
        ext_ior: f32,                              // extIOR "air"
        specular_reflectance: BSDFColorSpectrum,   // s(1.0)
        specular_transmittance: BSDFColorSpectrum, // s(1.0)
        thin: bool,                                // to handle both objects
    },
}

impl BSDF {
    pub fn default() -> Self {
        BSDF::Diffuse {
            reflectance: BSDFColorSpectrum::Constant(Spectrum::from_f32(0.8)),
        }
    }
    pub fn parse<R: Read>(event: &mut Events<R>, bsdf_type: &str, scene: &Scene) -> Option<Self> {
        match bsdf_type {
            "phong" => {
                let mut values = values(event, true);
                let exponent = match values.remove("exponent") {
                    Some(v) => v,
                    None => Value::Float(30.0),
                }
                .as_bsdf_color_f32(scene);
                let specular_reflectance = match values.remove("specularReflectance") {
                    Some(v) => v,
                    None => Value::Spectrum(Spectrum::from_f32(0.2)),
                }
                .as_bsdf_color_spec(scene);
                let diffuse_reflectance = match values.remove("diffuseReflectance") {
                    Some(v) => v,
                    None => Value::Spectrum(Spectrum::from_f32(0.5)),
                }
                .as_bsdf_color_spec(scene);
                Some(BSDF::Phong {
                    exponent,
                    specular_reflectance,
                    diffuse_reflectance,
                })
            }
            "diffuse" => {
                let mut values = values(event, true);
                let reflectance = match values.remove("reflectance") {
                    Some(v) => v,
                    None => Value::Spectrum(Spectrum::from_f32(0.5)),
                }
                .as_bsdf_color_spec(scene);
                Some(BSDF::Diffuse { reflectance })
            }

            "dielectric" | "roughdielectric" | "thindielectric" => {
                let mut map = values(event, true);
                let distribution = if bsdf_type == "roughdielectric" {
                    Some(Distribution::parse(&mut map, scene))
                } else {
                    None
                };
                let thin = bsdf_type == "thindielectric";

                let int_ior =
                    read_value(&mut map, "intIOR", Value::String("bk7".to_string())).as_ior();
                let ext_ior =
                    read_value(&mut map, "extIOR", Value::String("air".to_string())).as_ior();
                let specular_reflectance = read_value(
                    &mut map,
                    "specularReflectance",
                    Value::Spectrum(Spectrum::from_f32(1.0)),
                )
                .as_bsdf_color_spec(scene);
                let specular_transmittance = read_value(
                    &mut map,
                    "specularTransmittance",
                    Value::Spectrum(Spectrum::from_f32(1.0)),
                )
                .as_bsdf_color_spec(scene);

                Some(BSDF::Dielectric {
                    distribution,
                    int_ior,
                    ext_ior,
                    specular_reflectance,
                    specular_transmittance,
                    thin,
                })
            }
            "conductor" | "roughconductor" => {
                println!("[WARN] Ignoring material of type {}", bsdf_type);
                skipping_entry(event);
                None
            }
            _ => panic!("Unsupported material {}", bsdf_type),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    filename: String,
}
impl Texture {
    pub fn parse<R: Read>(events: &mut Events<R>) -> Self {
        let mut map = values(events, true);
        let filename = map.remove("filename").unwrap().as_string();
        Texture { filename }
    }
}

#[derive(Debug)]
pub enum Emitter {
    Area { radiance: Spectrum },
}
impl Emitter {
    pub fn parse<R: Read>(events: &mut Events<R>, emitter_type: &str) -> Option<Self> {
        match emitter_type {
            "area" => {
                let mut map = values(events, true);
                let radiance = map.remove("radiance").unwrap().as_spectrum();
                Some(Emitter::Area { radiance })
            }
            _ => {
                println!("[WARN] Ignoring {} emitter type", emitter_type);
                skipping_entry(events);
                None
            }
        }
    }
}

#[derive(Debug)]
pub enum Shape {
    Serialized {
        filename: String,
        shape_index: u32,
        bsdf: Option<BSDF>,
        to_world: Option<Transform>,
        emitter: Option<Emitter>,
    },
}
impl Shape {
    pub fn parse<R: Read>(events: &mut Events<R>, shape_type: &str, scene: &Scene) -> Option<Self> {
        // FIXME: Can be object or references
        //  if there are references, we need to handle them
        //  differentely.
        let mut bsdf = None;
        let mut to_world = None;
        let mut emitter = None;

        let f = |events: &mut Events<R>, t: &str, attrs: HashMap<String, String>| -> bool {
            match t {
                "bsdf" => {
                    let bsdf_type = attrs.get("type").unwrap();
                    // FIXME: The unwrap can be catastrophic!
                    bsdf = Some(BSDF::parse(events, bsdf_type, scene).unwrap());
                }
                "transform" => to_world = Some(Transform::parse(events)),
                "emitter" => {
                    let emitter_type = attrs.get("type").unwrap();
                    emitter = Emitter::parse(events, emitter_type);
                }
                _ => panic!("Unexpected token!"),
            }
            true
        };

        match shape_type {
            "serialized" => {
                let mut map = values_fn(events, false, f);
                let filename = map.remove("filename").unwrap().as_string();
                let shape_index = map.remove("shapeIndex").unwrap().as_int() as u32;

                // Try to fill things that missing
                if bsdf.is_none() {
                    match map.remove("bsdf") {
                        None => {}
                        Some(v) => bsdf = Some(scene.bsdfs.get(&v.as_string()).unwrap().clone()),
                    }
                }
                if emitter.is_none() {
                    // TODO: Need to implement ref to emitters
                    assert!(map.remove("emitter").is_none());
                }

                Some(Shape::Serialized {
                    filename,
                    shape_index,
                    bsdf,
                    to_world,
                    emitter,
                })
            }
            _ => {
                println!("[WARN] Ignoring {} shape type", shape_type);
                skipping_entry(events);
                None
            }
        }
    }
}

#[derive(Debug)]
pub struct Film {
    height: u32,
    width: u32,
}
impl Film {
    pub fn parse<R: Read>(events: &mut Events<R>) -> Self {
        let mut map = values(events, false);
        let height = map.remove("height").unwrap().as_int() as u32;
        let width = map.remove("width").unwrap().as_int() as u32;
        Self { height, width }
    }
}

#[derive(Debug)]
pub struct Transform(Matrix4<f32>);
impl Transform {
    pub fn parse<R: Read>(events: &mut Events<R>) -> Self {
        let mut trans = Matrix4::one();
        let mut opened = 1;
        for e in events {
            match e {
                Ok(XmlEvent::StartElement {
                    name, attributes, ..
                }) => match name.local_name.as_str() {
                    "translate" => {
                        let x = found_attrib_or(&attributes, "x", "0.0")
                            .parse::<f32>()
                            .unwrap();
                        let y = found_attrib_or(&attributes, "y", "0.0")
                            .parse::<f32>()
                            .unwrap();
                        let z = found_attrib_or(&attributes, "z", "0.0")
                            .parse::<f32>()
                            .unwrap();
                        trans = trans * Matrix4::from_translation(Vector3::new(x, y, z));
                        opened += 1;
                    }
                    "scale" => {
                        let value = found_attrib(&attributes, "value");
                        match value {
                            Some(v) => {
                                let v = v.parse::<f32>().unwrap();
                                trans = trans * Matrix4::from_scale(v);
                            }
                            None => {
                                let x = found_attrib_or(&attributes, "x", "1.0")
                                    .parse::<f32>()
                                    .unwrap();
                                let y = found_attrib_or(&attributes, "y", "1.0")
                                    .parse::<f32>()
                                    .unwrap();
                                let z = found_attrib_or(&attributes, "z", "1.0")
                                    .parse::<f32>()
                                    .unwrap();

                                trans = trans * Matrix4::from_nonuniform_scale(x, y, z);
                            }
                        }
                        opened += 1;
                    }
                    "rotate" => {
                        let x = found_attrib_or(&attributes, "x", "0.0")
                            .parse::<f32>()
                            .unwrap();
                        let y = found_attrib_or(&attributes, "y", "0.0")
                            .parse::<f32>()
                            .unwrap();
                        let z = found_attrib_or(&attributes, "z", "0.0")
                            .parse::<f32>()
                            .unwrap();
                        let angle = found_attrib(&attributes, "angle")
                            .unwrap()
                            .parse::<f32>()
                            .unwrap();
                        let axis = Vector3::new(x, y, z);

                        trans = trans * Matrix4::from_axis_angle(axis, Deg(angle));
                        opened += 1;
                    }
                    "matrix" => {
                        let values = found_attrib(&attributes, "value").unwrap();
                        let values = values
                            .split(" ")
                            .into_iter()
                            .map(|v| v.parse::<f32>().unwrap())
                            .collect::<Vec<_>>();

                        let m00 = values[0];
                        let m01 = values[1];
                        let m02 = values[2];
                        let m03 = values[3];
                        let m10 = values[4];
                        let m11 = values[5];
                        let m12 = values[6];
                        let m13 = values[7];
                        let m20 = values[8];
                        let m21 = values[9];
                        let m22 = values[10];
                        let m23 = values[11];
                        let m30 = values[12];
                        let m31 = values[13];
                        let m32 = values[14];
                        let m33 = values[15];
                        let matrix = Matrix4::new(
                            m00, m01, m02, m03, m10, m11, m12, m13, m20, m21, m22, m23, m30, m31,
                            m32, m33,
                        );
                        //#[rustfmt::skip]
                        trans = trans * matrix;
                        opened += 1;
                    }
                    "lookat" => {
                        let origin = found_attrib_vec(&attributes, "origin").unwrap();
                        let target = found_attrib_vec(&attributes, "target").unwrap();
                        let up = found_attrib_vec(&attributes, "up").unwrap();

                        // Conversion
                        let dir = (target - origin).normalize();
                        let left = -dir.cross(up.normalize()).normalize();
                        let new_up = dir.cross(left);

                        use cgmath::Transform;
                        let matrix = Matrix4::new(
                            left.x, left.y, left.z, 0.0, new_up.x, new_up.y, new_up.z, 0.0, dir.x,
                            dir.y, dir.z, 0.0, origin.x, origin.y, origin.z, 1.0,
                        )
                        .inverse_transform()
                        .unwrap();

                        trans = trans * matrix;
                        opened += 1;
                    }
                    _ => panic!("uncover case {:?} for matrix op", name),
                },
                Ok(XmlEvent::EndElement { .. }) => {
                    if opened == 1 {
                        break;
                    }
                    opened -= 1;
                }
                Ok(XmlEvent::Whitespace(_)) => {}
                Err(e) => {
                    panic!("Parse values Error: {}", e);
                }
                _ => panic!("Transform default {:?}", e),
            }
        }

        Transform(trans)
    }
}

// Only support perspective sensor
#[derive(Debug)]
pub struct Sensor {
    // FIXME: Only support FOV
    //  focalLenght need to be added if we want to
    //  or doing the conversion
    pub fov: f32,           // No default
    pub fov_axis: String,   // "x"
    pub shutter_open: f32,  // 0.0
    pub shutter_close: f32, // 0.0
    pub near_clip: f32,     // 0.01
    pub far_clip: f32,      // 1000
    // More complex structures
    pub film: Option<Film>,
    pub to_world: Option<Transform>,
}
impl Sensor {
    pub fn parse<R: Read>(events: &mut Events<R>, sensor_type: &str) -> Self {
        assert_eq!(sensor_type, "perspective");

        // Use closure to initialize these extra stuffs
        let mut film = None;
        let mut to_world = None;
        let f = |events: &mut Events<R>, t: &str, _: HashMap<String, String>| -> bool {
            match t {
                "film" => film = Some(Film::parse(events)),
                "transform" => to_world = Some(Transform::parse(events)),
                "sampler" => {
                    println!("Skipping sampler information");
                    skipping_entry(events);
                }
                _ => panic!("Unexpected token!"),
            }
            true
        };

        let mut map = values_fn(events, false, f);
        let fov = map.remove("fov").unwrap().as_float();
        let fov_axis = read_value(&mut map, "fovAxis", Value::String("x".to_string())).as_string();
        let shutter_open = read_value(&mut map, "shutterOpen", Value::Float(0.0)).as_float();
        let shutter_close = read_value(&mut map, "shutterClose", Value::Float(0.0)).as_float();
        let near_clip = read_value(&mut map, "nearClip", Value::Float(0.01)).as_float();
        let far_clip = read_value(&mut map, "farClip", Value::Float(1000.0)).as_float();

        Self {
            fov,
            fov_axis,
            shutter_open,
            shutter_close,
            near_clip,
            far_clip,
            film,
            to_world,
        }
    }
}

#[derive(Debug)]
pub struct Scene {
    bsdfs: HashMap<String, BSDF>,
    textures: HashMap<String, Texture>,
    shapes: HashMap<String, Shape>,
    sensors: Vec<Sensor>,
    emitters: Vec<Emitter>,
}

pub fn mitsuba_print(file: &str) -> Scene {
    let file = File::open(file).unwrap();
    let file = BufReader::new(file);

    let parser = EventReader::new(file);

    let mut scene = Scene {
        bsdfs: HashMap::new(),
        textures: HashMap::new(),
        shapes: HashMap::new(),
        sensors: Vec::new(),
        emitters: Vec::new(),
    };

    let mut iter = parser.into_iter();
    loop {
        let e = match iter.next() {
            Some(x) => x,
            None => break,
        };

        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => match name.local_name.as_str() {
                "bsdf" => {
                    let bsdf_type = found_attrib(&attributes, "type").unwrap();
                    let bsdf_id = found_attrib(&attributes, "id").unwrap();
                    let bsdf = BSDF::parse(&mut iter, &bsdf_type, &scene);
                    let bsdf = match bsdf {
                        Some(v) => v,
                        None => BSDF::default(),
                    };
                    scene.bsdfs.insert(bsdf_id, bsdf);
                }
                "texture" => {
                    let texture_id = found_attrib(&attributes, "id").unwrap();
                    let texture = Texture::parse(&mut iter);
                    scene.textures.insert(texture_id, texture);
                }
                "sensor" => {
                    let sensor_type = found_attrib(&attributes, "type").unwrap();
                    let sensor = Sensor::parse(&mut iter, &sensor_type);
                    scene.sensors.push(sensor);
                }
                "emitter" => {
                    let emitter_type = found_attrib(&attributes, "type").unwrap();
                    let emitter = Emitter::parse(&mut iter, &emitter_type);
                    if let Some(e) = emitter {
                        scene.emitters.push(e);
                    }
                }
                "default" => {
                    println!("[WARN] default are ignored");
                    skipping_entry(&mut iter);
                }
                "scene" => {
                    // Nothing to do
                }
                "integrator" => {
                    // We ignoring the integrator
                    // as for scene parsing, it gives us no information
                    skipping_entry(&mut iter);
                }
                "shape" => {
                    let shape_type = found_attrib(&attributes, "type").unwrap();
                    let shape_id = found_attrib(&attributes, "id").unwrap();
                    let shape = Shape::parse(&mut iter, &shape_type, &scene);
                    if let Some(s) = shape {
                        scene.shapes.insert(shape_id, s);
                    }
                }
                _ => panic!("Unsupported primitive type {} {:?}", name, attributes),
            },
            Ok(XmlEvent::EndElement { .. }) => {
                // TODO: Might want to check the type in case...
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
            _ => {}
        }
    }
    return scene;
}

#[cfg(test)]
mod tests {
    fn print_scene(scene: crate::Scene) {
        println!("BSDFs");
        for (k, v) in &scene.bsdfs {
            println!(" - {} = {:?}", k, v);
        }
        println!("Textures");
        for (k, v) in &scene.textures {
            println!(" - {} = {:?}", k, v);
        }
        println!("Shapes");
        for (k, v) in &scene.shapes {
            println!(" - {} = {:?}", k, v);
        }
        println!("Sensor");
        for v in &scene.sensors {
            println!(" - {:?}", v);
        }
        println!("Emitters");
        for v in &scene.emitters {
            println!(" - {:?}", v);
        }
    }

    #[test]
    fn bookshelf() {
        let s = "./data/bookshelf.xml";
        print_scene(crate::mitsuba_print(s));
    }

    #[test]
    fn aquarium() {
        let s = "./data/aquarium.xml";
        print_scene(crate::mitsuba_print(s));
    }

    #[test]
    fn bsdf() {
        let s = "./data/bsdf.xml";
        print_scene(crate::mitsuba_print(s));
    }
}
