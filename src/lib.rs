extern crate cgmath;
extern crate xml;

use std::fs::File;
use std::io::BufReader;

use cgmath::*;
use std::collections::HashMap;
use std::io::Read;
use xml::reader::{EventReader, Events, XmlEvent};

#[derive(Debug)]
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

#[derive(Debug)]
pub enum Value {
    Float(f32),
    Texture(String),
    Spectrum(Spectrum),
    String(String),
    Integer(i32),
    Boolean(bool),
    Ref(String),
}
impl Value {
    pub fn as_string(self) -> String {
        match self {
            Value::String(s) | Value::Texture(s) => s,
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
}

fn found_attrib(attrs: &Vec<xml::attribute::OwnedAttribute>, name: &str) -> Option<String> {
    for a in attrs {
        if a.name.local_name == name {
            return Some(a.value.clone());
        }
    }
    None
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
    F: FnMut(&mut Events<R>, &str) -> bool,
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
                "texture" => {
                    let name = found_attrib(&attributes, "name").unwrap();
                    let value = found_attrib(&attributes, "value").unwrap();
                    map.insert(name, Value::Texture(value));
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
                    let captured = other(iter, &name.local_name);
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
    let f = |_: &mut Events<R>, _: &str| false;
    values_fn(events, strict, f)
}

fn read_value(m: &mut HashMap<String, Value>, n: &str, d: Value) -> Value {
    match m.remove(n) {
        Some(v) => v,
        None => d,
    }
}

#[derive(Debug)]
pub struct Distribution {
    pub distribution: String,
    pub alpha_u: f32,
    pub alpha_v: f32,
}
impl Distribution {
    fn parse(map: &mut HashMap<String, Value>) -> Self {
        let distribution =
            read_value(map, "distribution", Value::String("beckmann".to_string())).as_string();
        let (alpha_u, alpha_v) = {
            let is_alpha = map.get("alpha").is_some();
            if is_alpha {
                let alpha = map.remove("alpha").unwrap().as_float();
                (alpha, alpha)
            } else {
                let alpha_u = read_value(map, "alpha_u", Value::Float(0.1)).as_float();
                let alpha_v = read_value(map, "alpha_v", Value::Float(0.1)).as_float();
                (alpha_u, alpha_v)
            }
        };
        Self {
            distribution,
            alpha_u,
            alpha_v,
        }
    }
}

#[derive(Debug)]
pub enum BSDF {
    Phong {
        exponent: Value,
        specular_reflectance: Value,
        diffuse_reflectance: Value,
    },
    Diffuse {
        reflectance: Value,
    },
    Roughtdiffuse {
        relectance: Value,     // s(0.5)
        alpha: Value,          // s(0.2)
        use_fast_approx: bool, // false
    },
    Conductor {
        distribution: Option<Distribution>,
        // Potentially read values from materials
        eta: Spectrum,
        k: Spectrum,
        // Other
        ext_eta: Value,              // Air
        specular_reflectance: Value, // s(1.0)
    },
    Dielectric {
        distribution: Option<Distribution>,
        int_ior: Value,                // intIOR "bk7"
        ext_ior: Value,                // extIOR "air"
        specular_reflectance: Value,   // s(1.0)
        specular_transmittance: Value, // s(1.0)
        thin: bool,                    // to handle both objects
    },
}

impl BSDF {
    pub fn default() -> Self {
        BSDF::Diffuse {
            reflectance: Value::Spectrum(Spectrum::from_f32(0.8)),
        }
    }
    pub fn parse<R: Read>(event: &mut Events<R>, bsdf_type: &str) -> Option<Self> {
        match bsdf_type {
            "phong" => {
                let mut values = values(event, true);
                let exponent = match values.remove("exponent") {
                    Some(v) => v,
                    None => Value::Float(30.0),
                };
                let specular_reflectance = match values.remove("specularReflectance") {
                    Some(v) => v,
                    None => Value::Spectrum(Spectrum::from_f32(0.2)),
                };
                let diffuse_reflectance = match values.remove("diffuseReflectance") {
                    Some(v) => v,
                    None => Value::Spectrum(Spectrum::from_f32(0.5)),
                };
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
                };
                Some(BSDF::Diffuse { reflectance })
            }
            // "conductor" | "roughconductor" => Some(BSDF::Conductor),
            "dielectric" | "roughdielectric" | "thindielectric" => {
                let mut map = values(event, true);
                let distribution = if bsdf_type == "roughdielectric" {
                    Some(Distribution::parse(&mut map))
                } else {
                    None
                };
                let thin = bsdf_type == "thindielectric";

                let int_ior = read_value(&mut map, "intIOR", Value::String("bk7".to_string()));
                let ext_ior = read_value(&mut map, "extIOR", Value::String("air".to_string()));
                let specular_reflectance = read_value(
                    &mut map,
                    "specularReflectance",
                    Value::Spectrum(Spectrum::from_f32(1.0)),
                );
                let specular_transmittance = read_value(
                    &mut map,
                    "specularTransmittance",
                    Value::Spectrum(Spectrum::from_f32(1.0)),
                );

                Some(BSDF::Dielectric {
                    distribution,
                    int_ior,
                    ext_ior,
                    specular_reflectance,
                    specular_transmittance,
                    thin,
                })
            }
            _ => None,
        }
    }
}

#[derive(Debug)]
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
        skipping_entry(events);
        println!("WARN: Transform parsing is not implemented");
        Transform(Matrix4::one())
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
        let f = |events: &mut Events<R>, t: &str| -> bool {
            match t {
                "film" => film = Some(Film::parse(events)),
                "transform" => to_world = Some(Transform::parse(events)),
                "sampler" => (skipping_entry(events)),
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
    sensors: Vec<Sensor>,
}

pub fn mitsuba_print(file: &str) -> Scene {
    let file = File::open(file).unwrap();
    let file = BufReader::new(file);

    let parser = EventReader::new(file);

    let mut scene = Scene {
        bsdfs: HashMap::new(),
        textures: HashMap::new(),
        sensors: Vec::new(),
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
                    let bsdf = BSDF::parse(&mut iter, &bsdf_type);
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
                _ => (),
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
    #[test]
    fn bookshelf() {
        let s = "./data/bookshelf.xml";
        println!("{:?})", crate::mitsuba_print(s));
    }

    #[test]
    fn bsdf() {
        let s = "./data/bsdf.xml";
        println!("{:?})", crate::mitsuba_print(s));
    }
}
