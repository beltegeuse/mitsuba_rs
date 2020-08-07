extern crate xml;

use std::fs::File;
use std::io::BufReader;

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
}
impl Value {
    fn as_string(self) -> String {
        match self {
            Value::String(s) | Value::Texture(s) => s,
            _ => panic!("Wrong type {:?} (as_string)", self),
        }
    }
    fn as_float(self) -> f32 {
        match self {
            Value::Float(s) => s,
            _ => panic!("Wrong type {:?} (as_float)", self),
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

fn values<R: Read>(events: &mut Events<R>) -> HashMap<String, Value> {
    let mut map = HashMap::new();
    let mut opened = false;
    for e in events {
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
                "string" => {
                    let name = found_attrib(&attributes, "name").unwrap();
                    let value = found_attrib(&attributes, "value").unwrap();
                    map.insert(name, Value::String(value));
                    opened = true;
                }
                _ => (),
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
    map
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
                let mut values = values(event);
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
                let mut values = values(event);
                let reflectance = match values.remove("reflectance") {
                    Some(v) => v,
                    None => Value::Spectrum(Spectrum::from_f32(0.5)),
                };
                Some(BSDF::Diffuse { reflectance })
            }
            // "conductor" | "roughconductor" => Some(BSDF::Conductor),
            "dielectric" | "roughdielectric" | "thindielectric" => {
                let mut map = values(event);
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
pub struct Scene {
    bsdfs: Vec<BSDF>,
}

pub fn mitsuba_print(file: &str) {
    let file = File::open(file).unwrap();
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
                    let bsdf_id = found_attrib(&attributes, "id");
                    let bsdf = BSDF::parse(&mut iter, &bsdf_type);
                    println!("bsdf [{:?}] = {:?}", bsdf_id, bsdf);
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
}

#[cfg(test)]
mod tests {
    #[test]
    fn bookshelf() {
        let s = "./data/bookshelf.xml";
        crate::mitsuba_print(s);
    }

    #[test]
    fn bsdf() {
        let s = "./data/bsdf.xml";
        crate::mitsuba_print(s);
    }
}
