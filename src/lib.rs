extern crate cgmath;
extern crate xml;
#[macro_use]
extern crate lazy_static;
#[cfg(feature = "serialized")]
extern crate byteorder;
#[cfg(feature = "serialized")]
extern crate miniz_oxide;
#[cfg(feature = "serialized")]
#[macro_use]
extern crate bitflags;

use cgmath::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
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

pub struct RGB {
    pub r: f32,
    pub g: f32,
    pub b: f32,
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

    pub fn as_rgb(self) -> RGB {
        // Mitsuba allow to give RGB values that can be true spectrum
        // where values are defined for wavelenght.
        // We do not support yet transformation between these spectrum
        // and RGB values
        if let Some(_) = self.value.find(":") {
            panic!(
                "True spectrum values are not supported yet! {:?}",
                self.value
            );
        }

        // Kinda anoying but Mitsuba allow multiple way to specify the
        // format of spectrum (single value, "r, g, b", "r g b" or "#color")
        // This is why this part of the code is a bit complicated
        let values = self.value.split(",");
        let values = if values.clone().count() > 1 {
            values
                .into_iter()
                .map(|v| v.trim().parse::<f32>().unwrap())
                .collect::<Vec<_>>()
        } else if let Some(p) = self.value.trim().find('#') {
            assert!(p == 0);
            // TODO: Do HEX conversion
            vec![0.0] // Black value
        } else {
            self.value
                .split_whitespace()
                .into_iter()
                .map(|v| v.parse::<f32>().unwrap())
                .collect::<Vec<_>>()
        };

        match values[..] {
            [r, g, b] => RGB { r, g, b },
            [v] => RGB { r: v, g: v, b: v },
            _ => panic!("Impossible to convert to RGB {:?}", values),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum BSDFColor<T> {
    Texture(Texture),
    Constant(T),
}
pub type BSDFColorSpectrum = BSDFColor<Spectrum>;
pub type BSDFColorFloat = BSDFColor<f32>;

#[derive(Debug)]
pub enum Value {
    Float(f32),
    Spectrum(Spectrum),
    String(String),
    Vector(Vector3<f32>),
    Point(Point3<f32>),
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
    pub fn as_vec(self) -> Vector3<f32> {
        match self {
            Value::Vector(v) => v,
            _ => panic!("Wrong type {:?} (as_vec)", self),
        }
    }
    pub fn as_point(self) -> Point3<f32> {
        match self {
            Value::Point(v) => v,
            _ => panic!("Wrong type {:?} (as_point)", self),
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

// Return values and list of unamed id
fn values_fn<R: Read, F>(
    events: &mut Events<R>,
    strict: bool,
    mut other: F,
) -> (HashMap<String, Value>, Vec<String>)
where
    F: FnMut(&mut Events<R>, &str, HashMap<String, String>) -> bool,
{
    let mut map = HashMap::new();
    let mut refs = Vec::new();
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
                    let name = found_attrib(&attributes, "name");
                    let value = found_attrib(&attributes, "id").unwrap();
                    match name {
                        Some(v) => {
                            map.insert(v, Value::Ref(value));
                        }
                        None => {
                            refs.push(value);
                        }
                    };
                    opened = true;
                }
                "vector" => {
                    let name = found_attrib(&attributes, "name").unwrap();
                    let x = found_attrib(&attributes, "x")
                        .unwrap_or("0.0".to_string())
                        .parse::<f32>()
                        .unwrap();
                    let y = found_attrib(&attributes, "y")
                        .unwrap_or("0.0".to_string())
                        .parse::<f32>()
                        .unwrap();
                    let z = found_attrib(&attributes, "z")
                        .unwrap_or("0.0".to_string())
                        .parse::<f32>()
                        .unwrap();
                    map.insert(name, Value::Vector(Vector3::new(x, y, z)));
                    opened = true;
                }
                "point" => {
                    let name = found_attrib(&attributes, "name").unwrap();
                    let x = found_attrib(&attributes, "x")
                        .unwrap_or("0.0".to_string())
                        .parse::<f32>()
                        .unwrap();
                    let y = found_attrib(&attributes, "y")
                        .unwrap_or("0.0".to_string())
                        .parse::<f32>()
                        .unwrap();
                    let z = found_attrib(&attributes, "z")
                        .unwrap_or("0.0".to_string())
                        .parse::<f32>()
                        .unwrap();
                    map.insert(name, Value::Point(Point3::new(x, y, z)));
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
                    return (map, refs);
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

fn values<R: Read>(events: &mut Events<R>, strict: bool) -> (HashMap<String, Value>, Vec<String>) {
    let f = |_: &mut Events<R>, _: &str, _: HashMap<String, String>| false;
    values_fn(events, strict, f)
}

fn read_value(m: &mut HashMap<String, Value>, n: &str, d: Value) -> Value {
    match m.remove(n) {
        Some(v) => v,
        None => d,
    }
}

fn read_value_or_texture_spectrum(
    m: &mut HashMap<String, Value>,
    n: &str,
    d: Value,
    textures: &HashMap<String, Texture>,
    scene: &Scene,
) -> BSDFColorSpectrum {
    match m.remove(n) {
        Some(v) => v.as_bsdf_color_spec(scene),
        None => match textures.get(n) {
            Some(v) => BSDFColorSpectrum::Texture(v.clone()),
            None => d.as_bsdf_color_spec(scene),
        },
    }
}

fn read_value_or_texture_f32(
    m: &mut HashMap<String, Value>,
    n: &str,
    d: Value,
    textures: &HashMap<String, Texture>,
    scene: &Scene,
) -> BSDFColorFloat {
    match m.remove(n) {
        Some(v) => v.as_bsdf_color_f32(scene),
        None => match textures.get(n) {
            Some(v) => BSDFColorFloat::Texture(v.clone()),
            None => d.as_bsdf_color_f32(scene),
        },
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
    TwoSided {
        bsdf: Box<BSDF>,
    },
}

impl BSDF {
    pub fn default() -> Self {
        BSDF::Diffuse {
            reflectance: BSDFColorSpectrum::Constant(Spectrum::from_f32(0.8)),
        }
    }
    pub fn parse<R: Read>(event: &mut Events<R>, bsdf_type: &str, scene: &mut Scene) -> Self {
        // Helpers for catch textures
        let mut textures = HashMap::new();
        let f_texture = |events: &mut Events<R>, t: &str, attrs: HashMap<String, String>| -> bool {
            match t {
                "texture" => {
                    let texture_name = attrs.get("name").unwrap();
                    let texture_id = attrs.get("id");
                    let texture = Texture::parse(events);
                    textures.insert(texture_name.clone(), texture.clone());
                    if let Some(id) = texture_id {
                        // If there is an id, we need to include it
                        // to the map for futher uses
                        scene.textures.insert(id.to_string(), texture);
                    }
                }
                _ => panic!("Twosided encounter unexpected token {:?}", t),
            }
            true
        };

        match bsdf_type {
            "phong" => {
                let (mut map, refs) = values_fn(event, true, f_texture);
                assert!(refs.is_empty());
                let exponent = read_value_or_texture_f32(
                    &mut map,
                    "exponent",
                    Value::Float(30.0),
                    &textures,
                    scene,
                );
                let specular_reflectance = read_value_or_texture_spectrum(
                    &mut map,
                    "specularReflectance",
                    Value::Spectrum(Spectrum::from_f32(0.2)),
                    &textures,
                    scene,
                );
                let diffuse_reflectance = read_value_or_texture_spectrum(
                    &mut map,
                    "diffuseReflectance",
                    Value::Spectrum(Spectrum::from_f32(0.5)),
                    &textures,
                    scene,
                );
                BSDF::Phong {
                    exponent,
                    specular_reflectance,
                    diffuse_reflectance,
                }
            }
            "diffuse" => {
                let (mut map, refs) = values_fn(event, true, f_texture);
                assert!(refs.is_empty());

                let reflectance = read_value_or_texture_spectrum(
                    &mut map,
                    "reflectance",
                    Value::Spectrum(Spectrum::from_f32(0.5)),
                    &textures,
                    scene,
                );
                BSDF::Diffuse { reflectance }
            }

            "dielectric" | "roughdielectric" | "thindielectric" => {
                let (mut map, refs) = values_fn(event, true, f_texture);
                assert!(refs.is_empty());
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
                let specular_reflectance = read_value_or_texture_spectrum(
                    &mut map,
                    "specularReflectance",
                    Value::Spectrum(Spectrum::from_f32(1.0)),
                    &textures,
                    scene,
                );
                let specular_transmittance = read_value_or_texture_spectrum(
                    &mut map,
                    "specularTransmittance",
                    Value::Spectrum(Spectrum::from_f32(1.0)),
                    &textures,
                    scene,
                );

                BSDF::Dielectric {
                    distribution,
                    int_ior,
                    ext_ior,
                    specular_reflectance,
                    specular_transmittance,
                    thin,
                }
            }
            "twosided" => {
                // We need to parse the next element, including BSDF
                let mut bsdfs = vec![];
                let f = |events: &mut Events<R>, t: &str, attrs: HashMap<String, String>| -> bool {
                    match t {
                        "bsdf" => {
                            let bsdf_type = attrs.get("type").unwrap();
                            bsdfs.push(BSDF::parse(events, bsdf_type, scene));
                        }
                        _ => panic!("Twosided encounter unexpected token {:?}", t),
                    }
                    true
                };
                let (map, refs) = values_fn(event, true, f);
                assert!(refs.is_empty());
                assert!(map.is_empty());
                assert_eq!(bsdfs.len(), 1);
                BSDF::TwoSided {
                    bsdf: Box::new(bsdfs[0].clone()),
                }
            }
            "conductor" | "roughconductor" | "roughplastic" | "plastic" => {
                skipping_entry(event);
                BSDF::default()
            }
            _ => panic!("Unsupported material {}", bsdf_type),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    pub filename: String,
    pub filter_type: String,
    pub gamma: f32,
}
impl Texture {
    pub fn parse<R: Read>(events: &mut Events<R>) -> Self {
        let (mut map, refs) = values(events, true);
        assert!(refs.is_empty());
        let filename = map.remove("filename").unwrap().as_string();
        let filter_type = read_value(
            &mut map,
            "filterType",
            Value::String("trilinear".to_string()),
        )
        .as_string();
        let gamma = read_value(&mut map, "alpha", Value::Float(1.0)).as_float();
        Texture {
            filename,
            filter_type,
            gamma,
        }
    }
}

#[derive(Debug)]
pub struct AreaEmitter {
    pub radiance: Spectrum,
    pub sampling_weight: f32, // 1
}

#[derive(Debug)]
pub enum Emitter {
    // Area light
    // (kinda special as Mesh will have ref to it)
    Area(AreaEmitter),
    // Point light
    Point {
        to_world: Transform,
        position: Point3<f32>,
        intensity: Spectrum,  // 1
        sampling_weight: f32, // 1
    },
    // Spotlight
    Spot {
        to_world: Transform, // Id
        intensity: Spectrum, // 1
        cutoff_angle: f32,   // 20 deg
        beam_width: f32,     // cutoff_angle * 3.0 / 4.0
        texture: Option<Texture>,
        sampling_weight: f32, // 1
    },
    // Directional
    Directional {
        to_world: Transform,     // Id
        direction: Vector3<f32>, // Mandatory
        irradiance: Spectrum,    // 1
        sampling_weight: f32,    // 1
    },
    // Collimated
    Collimated {
        to_world: Transform,  // Id
        power: Spectrum,      // 1
        sampling_weight: f32, // 1
    },
    // Constant env map
    Constant {
        radiance: Spectrum,   // 1
        sampling_weight: f32, // 1
    },
    // Env map
    EnvMap {
        to_world: Transform,  // Id
        filename: String,     // Mandatory
        scale: f32,           // 1
        gamma: Option<f32>,   // Optional (or automatic)
        cache: Option<bool>,  // Optional (or automatic)
        sampling_weight: f32, // 1
    },
}
impl Emitter {
    pub fn parse<R: Read>(events: &mut Events<R>, emitter_type: &str) -> Self {
        let mut to_world = Transform(Matrix4::one());
        // TODO: Texture
        let f = |events: &mut Events<R>, t: &str, _: HashMap<String, String>| -> bool {
            match t {
                "transform" => to_world = Transform::parse(events),
                _ => panic!("Unexpected token!"),
            }
            true
        };

        let (mut map, refs) = values_fn(events, true, f);
        assert!(refs.is_empty());

        let sampling_weight = read_value(&mut map, "samplingWeight", Value::Float(1.0)).as_float();
        match emitter_type {
            "area" => {
                let radiance = map.remove("radiance").unwrap().as_spectrum();
                Emitter::Area(AreaEmitter {
                    radiance,
                    sampling_weight,
                })
            }
            "point" => {
                let position = map.remove("position").unwrap().as_point();
                let intensity = read_value(
                    &mut map,
                    "intensity",
                    Value::Spectrum(Spectrum::from_f32(1.0)),
                )
                .as_spectrum();
                Emitter::Point {
                    to_world,
                    position,
                    intensity,
                    sampling_weight,
                }
            }
            "spot" => {
                let intensity = read_value(
                    &mut map,
                    "intensity",
                    Value::Spectrum(Spectrum::from_f32(1.0)),
                )
                .as_spectrum();
                let cutoff_angle =
                    read_value(&mut map, "cutoffAngle", Value::Float(20.0)).as_float();
                let beam_width = read_value(
                    &mut map,
                    "beamWidth",
                    Value::Float(cutoff_angle * 3.0 / 4.0),
                )
                .as_float();
                let texture = None;
                Emitter::Spot {
                    to_world,
                    intensity,
                    cutoff_angle,
                    beam_width,
                    texture,
                    sampling_weight,
                }
            }
            "directional" => {
                let direction = map.remove("direction").unwrap().as_vec();
                let irradiance = read_value(
                    &mut map,
                    "irradiance",
                    Value::Spectrum(Spectrum::from_f32(1.0)),
                )
                .as_spectrum();
                Emitter::Directional {
                    to_world,
                    direction,
                    irradiance,
                    sampling_weight,
                }
            }
            "collimated" => {
                let power = map.remove("power").unwrap().as_spectrum();
                Emitter::Collimated {
                    to_world,
                    power,
                    sampling_weight,
                }
            }
            "constant" => {
                let radiance = map.remove("radiance").unwrap().as_spectrum();
                Emitter::Constant {
                    radiance,
                    sampling_weight,
                }
            }
            "envmap" => {
                let filename = map.remove("filename").unwrap().as_string();
                let scale = read_value(&mut map, "scale", Value::Float(1.0)).as_float();
                let gamma = match map.remove("gamma") {
                    None => None,
                    Some(v) => Some(v.as_float()),
                };
                let cache = match map.remove("cache") {
                    None => None,
                    Some(v) => Some(v.as_bool()),
                };
                Emitter::EnvMap {
                    to_world,
                    filename,
                    scale,
                    gamma,
                    cache,
                    sampling_weight,
                }
            }
            _ => {
                panic!("[ERROR] Uncovered {} emitter type", emitter_type);
                // skipping_entry(events);
            }
        }
    }

    pub fn as_area(self) -> AreaEmitter {
        match self {
            Emitter::Area(v) => v,
            _ => panic!("Wrong emitter type {:?} (as_area)", self),
        }
    }
}

#[derive(Debug)]
pub struct ShapeOption {
    pub flip_normal: bool, // false
    pub bsdf: Option<BSDF>,
    pub to_world: Option<Transform>,
    pub emitter: Option<AreaEmitter>,
}

#[derive(Debug)]
pub struct SerializedShape {
    pub filename: String,
    pub shape_index: u32,
    pub face_normal: bool,             // false
    pub max_smooth_angle: Option<f32>, // optional
    pub option: ShapeOption,
}

#[derive(Debug)]
pub enum Shape {
    Serialized(SerializedShape),
    Obj {
        filename: String,
        face_normal: bool,             // false
        max_smooth_angle: Option<f32>, // optional
        flip_tex_coords: bool,         // true
        collapse: bool,                // false
        option: ShapeOption,
    },
    Ply {
        filename: String,
        face_normal: bool,             // false
        max_smooth_angle: Option<f32>, // optional
        srgb: bool,                    // true
        option: ShapeOption,
    },
    Cube {
        option: ShapeOption,
    },
    Sphere {
        center: Point3<f32>, // (0,0,0)
        radius: f32,         // 1
        option: ShapeOption,
    },
    Cylinder {
        p0: Point3<f32>, // (0,0,0)
        p1: Point3<f32>, // (0,0,1)
        radius: f32,     // 1
        option: ShapeOption,
    },
    Rectangle {
        option: ShapeOption,
    },
    Disk {
        option: ShapeOption,
    },
    // TODO: Do shape group can have bsdf, to_world... associated?
    ShapeGroup {
        shapes: Vec<Shape>,
    },
    // Depending if we have reference or the group
    Instance {
        shape: Option<Box<Shape>>,
        ref_shape: Option<String>,
    },
}
impl Shape {
    pub fn parse<R: Read>(events: &mut Events<R>, shape_type: &str, scene: &mut Scene) -> Self {
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
                    bsdf = Some(BSDF::parse(events, bsdf_type, scene));
                }
                "transform" => to_world = Some(Transform::parse(events)),
                "emitter" => {
                    let emitter_type = attrs.get("type").unwrap();
                    emitter = Some(Emitter::parse(events, emitter_type).as_area());
                }
                _ => panic!("Unexpected token!"),
            }
            true
        };

        let (mut map, refs) = values_fn(events, true, f);
        for r in refs {
            // Only BSDF are supported for refs without type
            if let Some(v) = scene.bsdfs.get(&r) {
                bsdf = Some(v.clone());
                continue;
            }
            todo!();
        }

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

        let flip_normal = read_value(&mut map, "flipNormal", Value::Boolean(false)).as_bool();

        // Create shape option (shared across all shapes)
        let option = ShapeOption {
            flip_normal,
            bsdf,
            to_world,
            emitter,
        };

        // Read some values in advance to reduce the redundancy
        let face_normal = read_value(&mut map, "faceNormal", Value::Boolean(false)).as_bool();
        let max_smooth_angle = match map.remove("maxSmoothAngle") {
            None => None,
            Some(v) => Some(v.as_float()),
        };

        match shape_type {
            "serialized" => {
                let filename = map.remove("filename").unwrap().as_string();
                let shape_index = map.remove("shapeIndex").unwrap().as_int() as u32;
                Shape::Serialized(SerializedShape {
                    filename,
                    shape_index,
                    face_normal,
                    max_smooth_angle,
                    option,
                })
            }
            "obj" => {
                let filename = map.remove("filename").unwrap().as_string();
                let flip_tex_coords =
                    read_value(&mut map, "flipTexCoords", Value::Boolean(true)).as_bool();
                let collapse = read_value(&mut map, "collapse", Value::Boolean(false)).as_bool();
                Shape::Obj {
                    filename,
                    face_normal,
                    max_smooth_angle,
                    flip_tex_coords,
                    collapse,
                    option,
                }
            }
            "ply" => {
                let filename = map.remove("filename").unwrap().as_string();
                let srgb = read_value(&mut map, "srgb", Value::Boolean(true)).as_bool();
                Shape::Ply {
                    filename,
                    face_normal,
                    max_smooth_angle,
                    srgb,
                    option,
                }
            }
            "cube" => Shape::Cube { option },
            "sphere" => {
                let center =
                    read_value(&mut map, "center", Value::Point(Point3::new(0.0, 0.0, 0.0)))
                        .as_point();
                let radius = read_value(&mut map, "radius", Value::Float(1.0)).as_float();
                Shape::Sphere {
                    center,
                    radius,
                    option,
                }
            }
            "cylinder" => {
                let p0 =
                    read_value(&mut map, "p0", Value::Point(Point3::new(0.0, 0.0, 0.0))).as_point();
                let p1 =
                    read_value(&mut map, "p1", Value::Point(Point3::new(0.0, 0.0, 1.0))).as_point();
                let radius = read_value(&mut map, "radius", Value::Float(1.0)).as_float();
                Shape::Cylinder {
                    p0,
                    p1,
                    radius,
                    option,
                }
            }
            "rectangle" => Shape::Rectangle { option },
            "disk" => Shape::Disk { option },
            _ => {
                // TODO: Need to implement shapegroup and instance
                panic!("[ERROR] Uncover {} shape type", shape_type);
                // skipping_entry(events);
            }
        }
    }
}

#[derive(Debug)]
pub struct Film {
    pub height: u32,
    pub width: u32,
}
impl Film {
    pub fn parse<R: Read>(events: &mut Events<R>) -> Self {
        let (mut map, refs) = values(events, false);
        assert!(refs.is_empty());
        let height = map.remove("height").unwrap().as_int() as u32;
        let width = map.remove("width").unwrap().as_int() as u32;
        Self { height, width }
    }
}

#[derive(Debug, Clone)]
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

    pub fn as_matrix(self) -> Matrix4<f32> {
        self.0
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
    pub film: Film,
    pub to_world: Transform,
}
impl Sensor {
    pub fn parse<R: Read>(events: &mut Events<R>, sensor_type: &str) -> Self {
        assert_eq!(sensor_type, "perspective");

        // Use closure to initialize these extra stuffs
        let mut film = None;
        let mut to_world = Transform(Matrix4::one());
        let f = |events: &mut Events<R>, t: &str, _: HashMap<String, String>| -> bool {
            match t {
                "film" => film = Some(Film::parse(events)),
                "transform" => to_world = Transform::parse(events),
                "sampler" => {
                    println!("Skipping sampler information");
                    skipping_entry(events);
                }
                _ => panic!("Unexpected token!"),
            }
            true
        };

        let (mut map, refs) = values_fn(events, false, f);
        assert!(refs.is_empty());

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
            film: film.unwrap(),
            to_world,
        }
    }
}

#[derive(Debug)]
pub struct Scene {
    pub bsdfs: HashMap<String, BSDF>,
    pub textures: HashMap<String, Texture>,
    pub shapes_id: HashMap<String, Shape>,
    pub shapes_unamed: Vec<Shape>,
    pub sensors: Vec<Sensor>,
    pub emitters: Vec<Emitter>,
}
impl Scene {
    // TODO:
    // pub fn shapes(&self) -> dyn Iterator<Item = &Shape> {
    //     self.shapes_id.iter().map(|(k,v)| v).chain(self.shapes_unamed.iter())
    // }
}

#[cfg(feature = "serialized")]
pub mod serialized;

fn parse_scene(filename: &str, mut scene: &mut Scene) {
    let file = File::open(filename).unwrap();
    let file = BufReader::new(file);

    let parser = EventReader::new(file);

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
                    let bsdf = BSDF::parse(&mut iter, &bsdf_type, &mut scene);
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
                    scene.emitters.push(emitter);
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
                "include" => {
                    // Read a new file
                    let other_filename = found_attrib(&attributes, "filename").unwrap();
                    let filename = std::path::Path::new(filename)
                        .parent()
                        .unwrap()
                        .join(std::path::Path::new(&other_filename));
                    parse_scene(
                        &filename.into_os_string().into_string().unwrap(),
                        &mut scene,
                    );
                    skipping_entry(&mut iter);
                }
                "shape" => {
                    let shape_type = found_attrib(&attributes, "type").unwrap();
                    let shape_id = found_attrib(&attributes, "id");
                    let shape = Shape::parse(&mut iter, &shape_type, &mut scene);
                    match shape_id {
                        Some(v) => {
                            scene.shapes_id.insert(v, shape);
                        }
                        None => {
                            scene.shapes_unamed.push(shape);
                        }
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
}

pub fn parse(file: &str) -> Scene {
    let mut scene = Scene {
        bsdfs: HashMap::new(),
        textures: HashMap::new(),
        shapes_id: HashMap::new(),
        shapes_unamed: Vec::new(),
        sensors: Vec::new(),
        emitters: Vec::new(),
    };
    parse_scene(file, &mut scene);
    scene
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
        for (k, v) in &scene.shapes_id {
            println!(" - {} = {:?}", k, v);
        }
        for v in &scene.shapes_unamed {
            println!(" - {:?}", v);
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
        print_scene(crate::parse(s));
    }

    #[test]
    fn aquarium() {
        let s = "./data/aquarium.xml";
        print_scene(crate::parse(s));
    }

    #[test]
    fn chess() {
        let s = "./data/chess.xml";
        print_scene(crate::parse(s));
    }

    #[test]
    fn house() {
        let s = "./data/house.xml";
        print_scene(crate::parse(s));
    }

    #[test]
    fn bsdf() {
        let s = "./data/bsdf.xml";
        print_scene(crate::parse(s));
    }

    #[test]
    fn cbox() {
        let s = "./data/cbox.xml";
        print_scene(crate::parse(s));
    }

    #[test]
    fn kitchen() {
        let s = "./data/kitchen.xml";
        print_scene(crate::parse(s));
    }
}
