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
#[cfg(feature = "ply")]
extern crate ply_rs;
#[macro_use]
extern crate quick_error;

use cgmath::*;
use log::warn;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use xml::reader::{EventReader, Events, XmlEvent};

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        /// Spectrum to RGB conversion error
        RGB(reason: &'static str, value: String) {
            display("RGB error: {} (value: {})", reason, value)
        }
        /// Value to Concrete type error
        Value(attempt: &'static str, value: Value) {
            display("Value error: {} (value: {:?})", attempt, value)
        }
        /// Unknown Reference
        UnknownReference(name: String) {
            display("Unknown reference (name: {:?})", name)
        }
        /// Attribute not found
        AttribNotFound(name: String, additional_info: String) {
            display("Impossible to found {} attribute when parsing {}", name, additional_info)
        }
        /// Other error
        Other(err: Box<dyn std::error::Error>) {
            source(&**err)
        }
    }
}
type Result<T> = std::result::Result<T, Error>;

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

lazy_static! {
    // Precomputed (eta, k) in RGB from Mitsuba
    static ref MATERIALS: HashMap<String, (String, String)> = {
        let mut m = HashMap::new();
        m.insert("a-C".to_string(), ("2.9296, 2.22909, 1.97023".to_string(), "0.892218, 0.791926, 0.815701".to_string()));
        m.insert("Ag".to_string(), ("0.155518, 0.116786, 0.138372".to_string(), "4.83241, 3.12322, 2.14934".to_string()));
        m.insert("AlAs_palik".to_string(), ("3.60456, 3.23388, 2.20695".to_string(), "0.000688883, -0.000497156, 0.00753".to_string()));
        m.insert("AlAs".to_string(), ("3.60456, 3.23388, 2.20695".to_string(), "0.000688883, -0.000497156, 0.00753".to_string()));
        m.insert("AlSb_palik".to_string(), ("-0.0339369, 4.14454, 4.6468".to_string(), "-0.0330091, 0.10024, 1.29296".to_string()));
        m.insert("AlSb".to_string(), ("-0.0339369, 4.14454, 4.6468".to_string(), "-0.0330091, 0.10024, 1.29296".to_string()));
        m.insert("Al".to_string(), ("1.66026, 0.881462, 0.521613".to_string(), "9.22807, 6.27106, 4.84111".to_string()));
        m.insert("Au".to_string(), ("0.143552, 0.377438, 1.43825".to_string(), "3.98397, 2.38495, 1.60434".to_string()));
        m.insert("Be_palik".to_string(), ("4.17079, 3.18615, 2.78579".to_string(), "3.84741, 3.00959, 2.86869".to_string()));
        m.insert("Be".to_string(), ("4.17079, 3.18615, 2.78579".to_string(), "3.84741, 3.00959, 2.86869".to_string()));
        m.insert("Cr".to_string(), ("4.48822, 2.90684, 1.66261".to_string(), "5.21422, 4.2289, 3.75312".to_string()));
        m.insert("CsI_palik".to_string(), ("2.14445, 1.70236, 1.66293".to_string(), "0, 0, 0".to_string()));
        m.insert("CsI".to_string(), ("2.14445, 1.70236, 1.66293".to_string(), "0, 0, 0".to_string()));
        m.insert("Cu2O_palik".to_string(), ("3.54545, 2.94339, 2.71336".to_string(), "0.120343, 0.205637, 0.637298".to_string()));
        m.insert("Cu2O".to_string(), ("3.54545, 2.94339, 2.71336".to_string(), "0.120343, 0.205637, 0.637298".to_string()));
        m.insert("CuO_palik".to_string(), ("3.25286, 2.44938, 2.2043".to_string(), "0.520388, 0.569286, 0.726005".to_string()));
        m.insert("CuO".to_string(), ("3.25286, 2.44938, 2.2043".to_string(), "0.520388, 0.569286, 0.726005".to_string()));
        m.insert("Cu_palik".to_string(), ("0.241275, 0.903804, 1.10182".to_string(), "3.95032, 2.46606, 2.1349".to_string()));
        m.insert("Cu".to_string(), ("0.208084, 0.919438, 1.10263".to_string(), "3.92329, 2.45611, 2.14264".to_string()));
        m.insert("d-C_palik".to_string(), ("2.71105, 2.31816, 2.23336".to_string(), "0, 0, 0".to_string()));
        m.insert("d-C".to_string(), ("2.71105, 2.31816, 2.23336".to_string(), "0, 0, 0".to_string()));
        m.insert("Hg_palik".to_string(), ("2.42652, 1.45642, 0.914459".to_string(), "6.34803, 4.39248, 3.42748".to_string()));
        m.insert("HgTe_palik".to_string(), ("4.77919, 3.23163, 2.66017".to_string(), "1.63198, 1.58015, 1.72981".to_string()));
        m.insert("HgTe".to_string(), ("4.77919, 3.23163, 2.66017".to_string(), "1.63198, 1.58015, 1.72981".to_string()));
        m.insert("Hg".to_string(), ("2.42652, 1.45642, 0.914459".to_string(), "6.34803, 4.39248, 3.42748".to_string()));
        m.insert("Ir_palik".to_string(), ("3.08243, 2.08491, 1.62028".to_string(), "5.59771, 4.06635, 3.27186".to_string()));
        m.insert("Ir".to_string(), ("3.08243, 2.08491, 1.62028".to_string(), "5.59771, 4.06635, 3.27186".to_string()));
        m.insert("K_palik".to_string(), ("0.0621071, 0.0466027, 0.0384292".to_string(), "2.12697, 1.3591, 0.9177".to_string()));
        m.insert("K".to_string(), ("0.0621071, 0.0466027, 0.0384292".to_string(), "2.12697, 1.3591, 0.9177".to_string()));
        m.insert("Li_palik".to_string(), ("0.269451, 0.200441, 0.223339".to_string(), "3.54131, 2.3517, 1.68708".to_string()));
        m.insert("Li".to_string(), ("0.269451, 0.200441, 0.223339".to_string(), "3.54131, 2.3517, 1.68708".to_string()));
        m.insert("MgO_palik".to_string(), ("2.08988, 1.65047, 1.5956".to_string(), "4.38534e-12, -3.64587e-12, 2.53198e-11".to_string()));
        m.insert("MgO".to_string(), ("2.08988, 1.65047, 1.5956".to_string(), "4.38534e-12, -3.64587e-12, 2.53198e-11".to_string()));
        m.insert("Mo_palik".to_string(), ("4.49907, 3.51237, 2.78469".to_string(), "4.12556, 3.4205, 3.15286".to_string()));
        m.insert("Mo".to_string(), ("4.49907, 3.51237, 2.78469".to_string(), "4.12556, 3.4205, 3.15286".to_string()));
        m.insert("Na_palik".to_string(), ("0.0607451, 0.0557257, 0.061748".to_string(), "3.20798, 2.12669, 1.58797".to_string()));
        m.insert("Nb_palik".to_string(), ("3.40884, 2.78681, 2.39788".to_string(), "3.44282, 2.73855, 2.57476".to_string()));
        m.insert("Nb".to_string(), ("3.40884, 2.78681, 2.39788".to_string(), "3.44282, 2.73855, 2.57476".to_string()));
        m.insert("Ni_palik".to_string(), ("2.36454, 1.66584, 1.46819".to_string(), "4.48772, 3.05528, 2.34846".to_string()));
        m.insert("Rh_palik".to_string(), ("2.59089, 1.86185, 1.55039".to_string(), "6.79501, 4.70811, 3.9766".to_string()));
        m.insert("Rh".to_string(), ("2.59089, 1.86185, 1.55039".to_string(), "6.79501, 4.70811, 3.9766".to_string()));
        m.insert("Se-e_palik".to_string(), ("5.56086, 4.22302, 4.0475".to_string(), "0.761602, 1.0705, 1.5996".to_string()));
        m.insert("Se-e".to_string(), ("5.56086, 4.22302, 4.0475".to_string(), "0.761602, 1.0705, 1.5996".to_string()));
        m.insert("Se_palik".to_string(), ("3.9737, 2.88842, 2.82447".to_string(), "0.631111, 0.6311, 0.54084".to_string()));
        m.insert("Se".to_string(), ("3.9737, 2.88842, 2.82447".to_string(), "0.631111, 0.6311, 0.54084".to_string()));
        m.insert("SiC_palik".to_string(), ("3.17069, 2.52702, 2.47947".to_string(), "1.54029e-06, -1.49586e-06, 1.47636e-05".to_string()));
        m.insert("SiC".to_string(), ("3.17069, 2.52702, 2.47947".to_string(), "1.54029e-06, -1.49586e-06, 1.47636e-05".to_string()));
        m.insert("SnTe_palik".to_string(), ("4.538, 1.9804, 1.2824".to_string(), "0, 0, 0".to_string()));
        m.insert("SnTe".to_string(), ("4.538, 1.9804, 1.2824".to_string(), "0, 0, 0".to_string()));
        m.insert("Ta_palik".to_string(), ("2.05979, 2.38107, 2.62559".to_string(), "2.43996, 1.74619, 1.94588".to_string()));
        m.insert("Ta".to_string(), ("2.05979, 2.38107, 2.62559".to_string(), "2.43996, 1.74619, 1.94588".to_string()));
        m.insert("Te-e_palik".to_string(), ("7.48425, 4.31476, 2.37063".to_string(), "5.59077, 4.93514, 4.00092".to_string()));
        m.insert("Te-e".to_string(), ("7.48425, 4.31476, 2.37063".to_string(), "5.59077, 4.93514, 4.00092".to_string()));
        m.insert("Te_palik".to_string(), ("7.36846, 4.4946, 2.63798".to_string(), "3.26738, 3.51453, 3.2917".to_string()));
        m.insert("Te".to_string(), ("7.36846, 4.4946, 2.63798".to_string(), "3.26738, 3.51453, 3.2917".to_string()));
        m.insert("ThF4_palik".to_string(), ("1.83057, 1.44222, 1.38804".to_string(), "0, 0, 0".to_string()));
        m.insert("ThF4".to_string(), ("1.83057, 1.44222, 1.38804".to_string(), "0, 0, 0".to_string()));
        m.insert("TiC_palik".to_string(), ("3.82591, 2.83346, 2.59154".to_string(), "3.18532, 2.41109, 2.1752".to_string()));
        m.insert("TiC".to_string(), ("3.82591, 2.83346, 2.59154".to_string(), "3.18532, 2.41109, 2.1752".to_string()));
        m.insert("TiN_palik".to_string(), ("1.70939, 1.20202, 1.35264".to_string(), "3.02375, 2.10671, 1.1688".to_string()));
        m.insert("TiN".to_string(), ("1.70939, 1.20202, 1.35264".to_string(), "3.02375, 2.10671, 1.1688".to_string()));
        m.insert("TiO2-e_palik".to_string(), ("3.10648, 2.51369, 2.585".to_string(), "6.15808e-05, -5.38042e-05, 0.000381368".to_string()));
        m.insert("TiO2-e".to_string(), ("3.10648, 2.51369, 2.585".to_string(), "6.15808e-05, -5.38042e-05, 0.000381368".to_string()));
        m.insert("TiO2_palik".to_string(), ("3.45676, 2.80243, 2.90704".to_string(), "0.000130815, -0.000114216, 0.000809353".to_string()));
        m.insert("TiO2".to_string(), ("3.45676, 2.80243, 2.90704".to_string(), "0.000130815, -0.000114216, 0.000809353".to_string()));
        m.insert("VC_palik".to_string(), ("3.67232, 2.77627, 2.53826".to_string(), "2.97645, 2.25387, 1.97359".to_string()));
        m.insert("VC".to_string(), ("3.67232, 2.77627, 2.53826".to_string(), "2.97645, 2.25387, 1.97359".to_string()));
        m.insert("VN_palik".to_string(), ("2.87842, 2.14604, 1.94788".to_string(), "2.83558, 2.13656, 1.64581".to_string()));
        m.insert("VN".to_string(), ("2.87842, 2.14604, 1.94788".to_string(), "2.83558, 2.13656, 1.64581".to_string()));
        m.insert("V_palik".to_string(), ("4.27582, 3.50453, 2.76487".to_string(), "3.50165, 2.89143, 3.10752".to_string()));
        m.insert("W".to_string(), ("4.37464, 3.29988, 2.99998".to_string(), "3.49872, 2.60604, 2.27449".to_string()));
        m.insert("none".to_string(), ("0.0, 0.0, 0.0".to_string(), "1.0, 1.0, 1.0".to_string()));
        m
    };
}

pub struct RGB {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

fn hex_to_int(c: char) -> i8 {
    let c = c.to_ascii_uppercase();
    match c {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        '5' => 5,
        '6' => 6,
        '7' => 7,
        '8' => 8,
        '9' => 9,
        'A' => 10,
        'B' => 11,
        'C' => 12,
        'D' => 13,
        'E' => 14,
        'F' => 15,
        _ => panic!("{} have no hex representation", c),
    }
}

#[derive(Debug, Clone, PartialEq)]
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

    pub fn as_rgb(self) -> Result<RGB> {
        // Mitsuba allow to give RGB values that can be true spectrum
        // where values are defined for wavelenght.
        // We do not support yet transformation between these spectrum
        // and RGB values
        if let Some(_) = self.value.find(":") {
            return Err(Error::RGB(
                "True spectrum values are not supported yet!",
                self.value,
            ));
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
        } else if let Some(_p) = self.value.trim().find('#') {
            // Conversion "#rrggbb" into rgb
            let values = self.value.trim().chars().skip(1);
            let values = values.collect::<Vec<_>>();
            if values.len() != 6 {
                return Err(Error::RGB("HEX color is not in proper format", self.value));
            }
            // Process by pairs
            values
                .chunks_exact(2)
                .map(|v| {
                    let (v1, v2) = unsafe { (v.get_unchecked(0), v.get_unchecked(1)) };
                    let v1 = hex_to_int(*v1);
                    let v2 = hex_to_int(*v2);
                    let v = v2 as i32 * 10 + v1 as i32;
                    v as f32 / 255.0
                })
                .collect()
        } else {
            self.value
                .split_whitespace()
                .into_iter()
                .map(|v| v.parse::<f32>().unwrap())
                .collect::<Vec<_>>()
        };

        match values[..] {
            [r, g, b] => Ok(RGB { r, g, b }),
            [v] => Ok(RGB { r: v, g: v, b: v }),
            _ => Err(Error::RGB("Impossible to convert to RGB", self.value)),
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

macro_rules! value_as {
    ( $name:ident, $value:path => $target:ty ) => {
        impl Value {
            pub fn $name(self) -> Result<$target> {
                match self {
                    $value(s) => Ok(s),
                    _ => Err(Error::Value(stringify!($name), self)),
                }
            }
        }
    };
}

value_as!(as_string, Value::String => String);
value_as!(as_ref, Value::Ref => String);
value_as!(as_float, Value::Float => f32);
value_as!(as_int, Value::Integer => i32);
value_as!(as_bool, Value::Boolean => bool);
value_as!(as_spectrum, Value::Spectrum => Spectrum);
value_as!(as_vec, Value::Vector => Vector3<f32>);
value_as!(as_point, Value::Point => Point3<f32>);

impl Value {
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

fn found_attrib_or_error(
    attrs: &Vec<xml::attribute::OwnedAttribute>,
    name: &str,
    additional_info: &str,
) -> Result<String> {
    for a in attrs {
        if a.name.local_name == name {
            return Ok(a.value.clone());
        }
    }
    Err(Error::AttribNotFound(
        name.to_string(),
        additional_info.to_string(),
    ))
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
            Ok(XmlEvent::StartElement { .. }) => {
                // We might want to be verbose to debug
                // println!("[WARN] skipping_entry {}", name);
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

fn match_value_or_defaults(value: String, defaults: &HashMap<String, String>) -> Result<String> {
    if value.is_empty() {
        Ok(value)
    } else {
        match value.chars().nth(0).unwrap() {
            '$' => match defaults.get(value.get(1..).unwrap()) {
                Some(v) => Ok(v.clone()),
                None => Err(Error::UnknownReference(value.get(1..).unwrap().to_string())),
            },
            _ => Ok(value),
        }
    }
}

// Return values and list of unamed id
fn values_fn<R: Read, F>(
    events: &mut Events<R>,
    defaults: &HashMap<String, String>,
    strict: bool,
    mut other: F,
) -> Result<(HashMap<String, Value>, Vec<String>)>
where
    F: FnMut(&mut Events<R>, &str, HashMap<String, String>) -> Result<bool>,
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
                    let name = found_attrib_or_error(&attributes, "name", "float")?;
                    let value = found_attrib_or_error(&attributes, "value", "float")?;
                    let value = match_value_or_defaults(value, defaults)?;
                    let value = value
                        .parse::<f32>()
                        .expect(&format!("Impossible to convert {} to f32", value));
                    map.insert(name, Value::Float(value));
                    opened = true;
                }
                "integer" => {
                    let name = found_attrib_or_error(&attributes, "name", "integer")?;
                    let value = found_attrib_or_error(&attributes, "value", "integer")?;
                    let value = match_value_or_defaults(value, defaults)?;
                    let value = value
                        .parse::<i32>()
                        .expect(&format!("Impossible to convert {} to i32", value));
                    map.insert(name, Value::Integer(value));
                    opened = true;
                }
                "boolean" => {
                    let name = found_attrib_or_error(&attributes, "name", "boolean")?;
                    let value = found_attrib_or_error(&attributes, "value", "boolean")?;
                    let value = match_value_or_defaults(value, defaults)?;
                    if value != "true" && value != "false" {
                        panic!(
                            "The boolean param '{}' with value '{}' is not a boolean",
                            name, value
                        );
                    }
                    let value = value == "true";
                    map.insert(name, Value::Boolean(value));
                    opened = true;
                }
                "spectrum" => {
                    let name = found_attrib_or_error(&attributes, "name", "spectrum")?;
                    let value = found_attrib_or_error(&attributes, "value", "spectrum")?;
                    let value = match_value_or_defaults(value, defaults)?;
                    map.insert(name, Value::Spectrum(Spectrum { value }));
                    opened = true;
                }
                "rgb" => {
                    let name = found_attrib_or_error(&attributes, "name", "rgb")?;
                    let value = found_attrib_or_error(&attributes, "value", "rgb")?;
                    let value = match_value_or_defaults(value, defaults)?;
                    map.insert(name, Value::Spectrum(Spectrum::from_rgb(value)));
                    opened = true;
                }
                "ref" => {
                    let name = found_attrib(&attributes, "name");
                    let value = found_attrib_or_error(&attributes, "id", "ref")?;
                    let value = match_value_or_defaults(value, defaults)?;
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
                    let name = found_attrib_or_error(&attributes, "name", "vector")?;
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
                    let name = found_attrib_or_error(&attributes, "name", "point")?;
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
                    let name = found_attrib_or_error(&attributes, "name", "string")?;
                    let value = found_attrib_or_error(&attributes, "value", "string")?;
                    let value = match_value_or_defaults(value, defaults)?;
                    map.insert(name, Value::String(value));
                    opened = true;
                }
                _ => {
                    // TODO: Might be inefficient
                    let map = attributes
                        .iter()
                        .map(|a| (a.name.local_name.clone(), a.value.clone()))
                        .collect();
                    let captured = other(iter, &name.local_name, map)?;
                    if !captured {
                        if strict {
                            panic!("{:?} encounter when parsing values", name)
                        } else {
                            // If we need to debug...
                            // println!("[WARN] {:?} is skipped", name);
                            skipping_entry(iter);
                        }
                    }
                }
            },
            Ok(XmlEvent::EndElement { .. }) => {
                if !opened {
                    return Ok((map, refs));
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

fn values<R: Read>(
    events: &mut Events<R>,
    defaults: &HashMap<String, String>,
    strict: bool,
) -> Result<(HashMap<String, Value>, Vec<String>)> {
    let f = |_: &mut Events<R>, _: &str, _: HashMap<String, String>| Ok(false);
    values_fn(events, defaults, strict, f)
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
impl Alpha {
    fn parse(map: &mut HashMap<String, Value>, scene: &Scene) -> Self {
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
    }
}

#[derive(Debug, Clone)]
pub struct Distribution {
    pub distribution: String,
    pub alpha: Alpha,
}
impl Distribution {
    fn parse(map: &mut HashMap<String, Value>, scene: &Scene) -> Result<Self> {
        let distribution =
            read_value(map, "distribution", Value::String("beckmann".to_string())).as_string()?;
        let alpha = Alpha::parse(map, scene);
        Ok(Self {
            distribution,
            alpha,
        })
    }
}

#[derive(Debug, Clone)]
pub enum WardVariant {
    Ward,
    WardDuer,
    Balanced,
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
        reflectance: BSDFColorSpectrum, // s(0.5)
        alpha: BSDFColorFloat,          // 0.2
        use_fast_approx: bool,          // false
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
    Plastic {
        distribution: Option<Distribution>,
        int_ior: f32,                            // intIOR "polypropylene"
        ext_ior: f32,                            // extIOR "air"
        specular_reflectance: BSDFColorSpectrum, // s(1.0)
        diffuse_reflectance: BSDFColorSpectrum,  // s(0.5)
        nonlinear: bool,                         // false
    },
    TwoSided {
        bsdf: Box<BSDF>,
    },
    MixtureBSDF {
        weights: Vec<f32>,
        bsdfs: Vec<BSDF>,
    },
    Ward {
        variant: WardVariant,                    // balanced
        alpha: Alpha,                            // 0.1 (Iso)
        specular_reflectance: BSDFColorSpectrum, // 0.2
        diffuse_reflectance: BSDFColorSpectrum,  // 0.5
    },
    Mask {
        opacity: BSDFColorSpectrum,
        bsdf: Box<BSDF>,
    },
}

impl BSDF {
    pub fn default() -> Self {
        BSDF::Diffuse {
            reflectance: BSDFColorSpectrum::Constant(Spectrum::from_f32(0.8)),
        }
    }
    pub fn parse<R: Read>(
        event: &mut Events<R>,
        defaults: &HashMap<String, String>,
        bsdf_type: &str,
        scene: &mut Scene,
    ) -> Result<Self> {
        // Helpers for catch textures
        let mut textures = HashMap::new();
        let f_texture =
            |events: &mut Events<R>, t: &str, attrs: HashMap<String, String>| -> Result<bool> {
                match t {
                    "texture" => {
                        let texture_name = attrs.get("name").unwrap();
                        let texture_type = attrs.get("type").unwrap();

                        let texture_id = attrs.get("id");
                        let texture = Texture::parse(events, defaults, &texture_type)?;
                        textures.insert(texture_name.clone(), texture.clone());
                        if let Some(id) = texture_id {
                            // If there is an id, we need to include it
                            // to the map for futher uses
                            scene.textures.insert(id.to_string(), texture);
                        }
                    }
                    _ => panic!("Twosided encounter unexpected token {:?}", t),
                }
                Ok(true)
            };

        match bsdf_type {
            "phong" => {
                let (mut map, refs) = values_fn(event, defaults, true, f_texture)?;
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
                Ok(BSDF::Phong {
                    exponent,
                    specular_reflectance,
                    diffuse_reflectance,
                })
            }
            "diffuse" => {
                let (mut map, refs) = values_fn(event, defaults, true, f_texture)?;
                assert!(refs.is_empty());

                let reflectance = read_value_or_texture_spectrum(
                    &mut map,
                    "reflectance",
                    Value::Spectrum(Spectrum::from_f32(0.5)),
                    &textures,
                    scene,
                );

                // If we use the default parameter, we will try
                // an alternative name
                let reflectance =
                    if reflectance == BSDFColorSpectrum::Constant(Spectrum::from_f32(0.5)) {
                        read_value_or_texture_spectrum(
                            &mut map,
                            "diffuseReflectance",
                            Value::Spectrum(Spectrum::from_f32(0.5)),
                            &textures,
                            scene,
                        )
                    } else {
                        reflectance
                    };

                Ok(BSDF::Diffuse { reflectance })
            }
            "dielectric" | "roughdielectric" | "thindielectric" => {
                let (mut map, refs) = values_fn(event, defaults, true, f_texture)?;
                assert!(refs.is_empty());
                let distribution = if bsdf_type == "roughdielectric" {
                    Some(Distribution::parse(&mut map, scene)?)
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

                Ok(BSDF::Dielectric {
                    distribution,
                    int_ior,
                    ext_ior,
                    specular_reflectance,
                    specular_transmittance,
                    thin,
                })
            }
            "mask" => {
                // Read the child element
                let mut bsdfs = vec![];
                let f = |events: &mut Events<R>,
                         t: &str,
                         attrs: HashMap<String, String>|
                 -> Result<bool> {
                    match t {
                        "bsdf" => {
                            let bsdf_type = attrs.get("type").unwrap();
                            bsdfs.push(BSDF::parse(events, defaults, bsdf_type, scene)?);
                        }
                        _ => {},
                    }
                    Ok(true)
                };
                let (mut map, refs) = values_fn(event, defaults, true, f)?;
                let bsdf = if bsdfs.is_empty() {
                    println!("[WARN] Mask BSDF is empty, using default");
                    BSDF::default()
                } else {
                    bsdfs[0].clone()
                };
                assert!(refs.is_empty());
 
                let opacity = read_value_or_texture_spectrum(
                    &mut map,
                    "opacity",
                    Value::Spectrum(Spectrum::from_f32(1.0)),
                    &textures,
                    scene,
                );
 
                Ok(BSDF::Mask { opacity, bsdf: Box::new(bsdf) })
            }
            "twosided" => {
                // We need to parse the next element, including BSDF
                let mut bsdfs = vec![];
                let f = |events: &mut Events<R>,
                         t: &str,
                         attrs: HashMap<String, String>|
                 -> Result<bool> {
                    match t {
                        "bsdf" => {
                            let bsdf_type = attrs.get("type").unwrap();
                            bsdfs.push(BSDF::parse(events, defaults, bsdf_type, scene)?);
                        }
                        _ => panic!("Twosided encounter unexpected token {:?}", t),
                    }
                    Ok(true)
                };
                let (map, refs) = values_fn(event, defaults, true, f)?;
                assert!(refs.is_empty());
                assert!(map.is_empty());
                assert_eq!(bsdfs.len(), 1);

                Ok(BSDF::TwoSided {
                    bsdf: Box::new(bsdfs[0].clone()),
                })
            }
            "roughplastic" | "plastic" => {
                let (mut map, refs) = values_fn(event, defaults, true, f_texture)?;
                assert!(refs.is_empty());
                let distribution = if bsdf_type == "roughplastic" {
                    Some(Distribution::parse(&mut map, scene)?)
                } else {
                    None
                };

                let int_ior = read_value(
                    &mut map,
                    "intIOR",
                    Value::String("polypropylene".to_string()),
                )
                .as_ior();
                let ext_ior =
                    read_value(&mut map, "extIOR", Value::String("air".to_string())).as_ior();
                let specular_reflectance = read_value_or_texture_spectrum(
                    &mut map,
                    "specularReflectance",
                    Value::Spectrum(Spectrum::from_f32(1.0)),
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

                let nonlinear =
                    read_value(&mut map, "nonlinear", Value::Boolean(false)).as_bool()?;

                Ok(BSDF::Plastic {
                    distribution,
                    int_ior,
                    ext_ior,
                    specular_reflectance,
                    diffuse_reflectance,
                    nonlinear,
                })
            }
            "roughdiffuse" => {
                let (mut map, refs) = values_fn(event, defaults, true, f_texture)?;
                assert!(refs.is_empty());
                let reflectance = read_value_or_texture_spectrum(
                    &mut map,
                    "reflectance",
                    Value::Spectrum(Spectrum::from_f32(0.5)),
                    &textures,
                    scene,
                );
                let alpha = read_value_or_texture_f32(
                    &mut map,
                    "alpha",
                    Value::Spectrum(Spectrum::from_f32(0.1)),
                    &textures,
                    scene,
                );
                let use_fast_approx =
                    read_value(&mut map, "useFastApprox", Value::Boolean(false)).as_bool()?;

                Ok(BSDF::Roughtdiffuse {
                    reflectance,
                    alpha,
                    use_fast_approx,
                })
            }
            "mixturebsdf" => {
                // We need to parse the next element, including BSDF
                let mut bsdfs = vec![];
                let f = |events: &mut Events<R>,
                         t: &str,
                         attrs: HashMap<String, String>|
                 -> Result<bool> {
                    match t {
                        "bsdf" => {
                            let bsdf_type = attrs.get("type").unwrap();
                            bsdfs.push(BSDF::parse(events, defaults, bsdf_type, scene)?);
                        }
                        _ => panic!("Twosided encounter unexpected token {:?}", t),
                    }
                    Ok(true)
                };
                let (mut map, refs) = values_fn(event, defaults, true, f)?;
                assert!(refs.is_empty());

                // Parse the weights
                let weights = map.remove("weights").unwrap().as_string()?;
                let weights = {
                    let trimed = weights.trim();
                    let splitted = if trimed.split(",").count() > 1 {
                        trimed.split(",")
                    } else {
                        trimed.split(" ")
                    };
                    splitted
                        .map(|v| v.trim().parse::<f32>().unwrap())
                        .collect::<Vec<_>>()
                };

                if bsdfs.is_empty() {
                    // Certainly the different BSDF are with references
                    // Do the iterations based on names
                    for i in 0..weights.len() {
                        let ref_name = map.remove(&format!("mat{}", i + 1)).unwrap().as_ref()?;
                        bsdfs.push(scene.bsdfs.get(&ref_name).unwrap().clone());
                    }
                }

                Ok(BSDF::MixtureBSDF { weights, bsdfs })
            }
            "conductor" | "roughconductor" => {
                let (mut map, refs) = values_fn(event, defaults, true, f_texture)?;
                assert!(refs.is_empty());
                let distribution = if bsdf_type == "roughconductor" {
                    Some(Distribution::parse(&mut map, scene)?)
                } else {
                    None
                };

                // Load the material or eta/k
                let material_name = map
                    .remove("material")
                    .unwrap_or(Value::String("Cu".to_owned()))
                    .as_string()?;
                let (eta, k) = MATERIALS.get(&material_name).unwrap();
                let (eta, k) = (
                    match map.remove("eta") {
                        None => Spectrum { value: eta.clone() },
                        Some(v) => v.as_spectrum()?,
                    },
                    match map.remove("k") {
                        None => Spectrum { value: k.clone() },
                        Some(v) => v.as_spectrum()?,
                    },
                );

                let ext_eta =
                    read_value(&mut map, "extEta", Value::String("air".to_string())).as_ior();

                let specular_reflectance = read_value_or_texture_spectrum(
                    &mut map,
                    "specularReflectance",
                    Value::Spectrum(Spectrum::from_f32(1.0)),
                    &textures,
                    scene,
                );

                Ok(BSDF::Conductor {
                    distribution,
                    eta,
                    k,
                    ext_eta,
                    specular_reflectance,
                })
            }
            "ward" => {
                let (mut map, refs) = values_fn(event, defaults, true, f_texture)?;
                assert!(refs.is_empty());

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
                let alpha = Alpha::parse(&mut map, scene);
                let variant =
                    read_value(&mut map, "variant", Value::String("balanced".to_string()))
                        .as_string()?;
                let variant = match &variant[..] {
                    "balanced" => WardVariant::Balanced,
                    "ward" => WardVariant::Ward,
                    "ward_duer" => WardVariant::WardDuer,
                    _ => panic!("Wrong ward variant: {}", variant), // FIXME
                };

                Ok(BSDF::Ward {
                    specular_reflectance,
                    diffuse_reflectance,
                    alpha,
                    variant,
                })
            }
            _ => panic!("Unsupported material {}", bsdf_type),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextureScale {
    Texture(Box<Texture>),
    Spectrum(Spectrum),
    Float(f32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Texture {
    Bitmap {
        /// TODO: wrapMode,
        /// maxAnisotropy,
        /// cache,
        /// channel
        filename: String,
        filter_type: String,
        gamma: f32,
        offset: Vector2<f32>,
        scale: Vector2<f32>,
    },
    Checkerboard {
        color0: Spectrum, // 0.4
        color1: Spectrum, // 0.2
        offset: Vector2<f32>,
        scale: Vector2<f32>,
    },
    GridTexture {
        color0: Spectrum, // 0.4
        color1: Spectrum, // 0.2
        line_width: f32,  // 0.01
        offset: Vector2<f32>,
        scale: Vector2<f32>,
    },
    // TODO: Implement scale
    Scale {
        texture: Box<Texture>,
        value: TextureScale,
    },
}
impl Texture {
    pub fn parse<R: Read>(
        events: &mut Events<R>,
        defaults: &HashMap<String, String>,
        texture_type: &str,
    ) -> Result<Self> {
        let (mut map, refs) = values(events, defaults, true)?;
        assert!(refs.is_empty());

        // Read offset and scale
        let uoffset = read_value(&mut map, "uoffset", Value::Float(0.0)).as_float()?;
        let voffset = read_value(&mut map, "voffset", Value::Float(0.0)).as_float()?;
        let uscale = read_value(&mut map, "uscale", Value::Float(1.0)).as_float()?;
        let vscale = read_value(&mut map, "vscale", Value::Float(1.0)).as_float()?;
        let offset = Vector2::new(uoffset, voffset);
        let scale = Vector2::new(uscale, vscale);

        match texture_type {
            "bitmap" => {
                let filename = map.remove("filename").unwrap().as_string()?;
                let filter_type = read_value(
                    &mut map,
                    "filterType",
                    Value::String("trilinear".to_string()),
                )
                .as_string()?;
                let gamma = read_value(&mut map, "alpha", Value::Float(1.0)).as_float()?;
                Ok(Texture::Bitmap {
                    filename,
                    filter_type,
                    gamma,
                    offset,
                    scale,
                })
            }
            "checkerboard" => {
                let color0 =
                    read_value(&mut map, "color0", Value::Spectrum(Spectrum::from_f32(0.4)))
                        .as_spectrum()?;
                let color1 =
                    read_value(&mut map, "color1", Value::Spectrum(Spectrum::from_f32(0.2)))
                        .as_spectrum()?;
                Ok(Texture::Checkerboard {
                    color0,
                    color1,
                    offset,
                    scale,
                })
            }
            "gridtexture" => {
                let color0 =
                    read_value(&mut map, "color0", Value::Spectrum(Spectrum::from_f32(0.4)))
                        .as_spectrum()?;
                let color1 =
                    read_value(&mut map, "color1", Value::Spectrum(Spectrum::from_f32(0.2)))
                        .as_spectrum()?;
                let line_width =
                    read_value(&mut map, "lineWidth", Value::Float(0.01)).as_float()?;
                Ok(Texture::GridTexture {
                    color0,
                    color1,
                    line_width,
                    offset,
                    scale,
                })
            }
            _ => panic!("Unsupported texture type: {}", texture_type),
        }
    }
}

#[derive(Debug)]
pub struct AreaEmitter {
    pub radiance: Spectrum,
    pub sampling_weight: f32, // 1
}

#[derive(Debug)]
pub struct SunEmitterParam {
    pub scale: f32,
    pub radius_scale: f32,
}

#[derive(Debug)]
pub struct SkyEmitterParam {
    pub scale: f32,
    pub stretch: f32,     // f32 [1-2], 1.0
    pub albedo: Spectrum, // 0.15
}

#[derive(Debug)]
pub enum SunDirection {
    Vector(Vector3<f32>),
    DateAndPos {
        year: i32,      // i32 2010
        month: i32,     // i32 07
        day: i32,       // i32 10
        hour: f32,      // f32 15
        minute: f32,    // f32 0
        second: f32,    // f32 0
        latitude: f32,  // f32 35.6894
        longitude: f32, // f32 139.6917
        timezone: f32,  // f32 9
    },
}
impl SunDirection {
    pub fn parse(
        mut map: &mut HashMap<String, Value>,
        _defaults: &HashMap<String, String>,
    ) -> Result<Self> {
        let sun_direction = map.remove("sunDirection");
        if let Some(sun_direction) = sun_direction {
            Ok(SunDirection::Vector(sun_direction.as_vec()?))
        } else {
            // Date
            let year = read_value(&mut map, "year", Value::Integer(2010)).as_int()?;
            let month = read_value(&mut map, "month", Value::Integer(10)).as_int()?;
            let day = read_value(&mut map, "day", Value::Integer(10)).as_int()?;
            // Time
            let hour = read_value(&mut map, "hour", Value::Float(15.0)).as_float()?;
            let minute = read_value(&mut map, "minute", Value::Float(0.0)).as_float()?;
            let second = read_value(&mut map, "second", Value::Float(0.0)).as_float()?;
            // Pos on earth
            let latitude = read_value(&mut map, "latitude", Value::Float(35.6894)).as_float()?;
            let longitude = read_value(&mut map, "longitude", Value::Float(139.6917)).as_float()?;
            let timezone = read_value(&mut map, "timezone", Value::Float(9.0)).as_float()?;
            Ok(SunDirection::DateAndPos {
                year,
                month,
                day,
                hour,
                minute,
                second,
                latitude,
                longitude,
                timezone,
            })
        }
    }
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
    // Point light with normal
    PointNormal {
        to_world: Transform,
        position: Point3<f32>,
        normal: Vector3<f32>,
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
    // SunSky, Sun or Sky
    SunSky {
        turbidity: f32,       // f32 [1-10], default 3
        resolution: u32,      // u32 512
        sampling_weight: f32, // f32 1.0
        sun_direction: SunDirection,
        sun: Option<SunEmitterParam>,
        sky: Option<SkyEmitterParam>,
        // toWorld -> Not supported
    },
}
impl Emitter {
    pub fn parse<R: Read>(
        events: &mut Events<R>,
        defaults: &HashMap<String, String>,
        emitter_type: &str,
    ) -> Result<Self> {
        let mut to_world = Transform(Matrix4::one());
        // TODO: Texture
        let f = |events: &mut Events<R>, t: &str, _: HashMap<String, String>| -> Result<bool> {
            match t {
                "transform" => to_world = Transform::parse(events),
                _ => panic!("Unexpected token!"),
            }
            Ok(true)
        };

        let (mut map, refs) = values_fn(events, defaults, true, f)?;
        assert!(refs.is_empty());

        let sampling_weight =
            read_value(&mut map, "samplingWeight", Value::Float(1.0)).as_float()?;
        match emitter_type {
            "area" => {
                let radiance = map.remove("radiance").unwrap().as_spectrum()?;
                Ok(Emitter::Area(AreaEmitter {
                    radiance,
                    sampling_weight,
                }))
            }
            "point" => {
                let position = map.remove("position").unwrap().as_point()?;
                let intensity = read_value(
                    &mut map,
                    "intensity",
                    Value::Spectrum(Spectrum::from_f32(1.0)),
                )
                .as_spectrum()?;
                Ok(Emitter::Point {
                    to_world,
                    position,
                    intensity,
                    sampling_weight,
                })
            }
            "point-normal" => {
                let position = map.remove("position").unwrap().as_point()?;
                let normal = map.remove("normal").unwrap().as_vec()?;
                let intensity = read_value(
                    &mut map,
                    "intensity",
                    Value::Spectrum(Spectrum::from_f32(1.0)),
                )
                .as_spectrum()?;
                Ok(Emitter::PointNormal {
                    to_world,
                    position,
                    normal,
                    intensity,
                    sampling_weight,
                })
            }
            "spot" => {
                let intensity = read_value(
                    &mut map,
                    "intensity",
                    Value::Spectrum(Spectrum::from_f32(1.0)),
                )
                .as_spectrum()?;
                let cutoff_angle =
                    read_value(&mut map, "cutoffAngle", Value::Float(20.0)).as_float()?;
                let beam_width = read_value(
                    &mut map,
                    "beamWidth",
                    Value::Float(cutoff_angle * 3.0 / 4.0),
                )
                .as_float()?;
                let texture = None;
                Ok(Emitter::Spot {
                    to_world,
                    intensity,
                    cutoff_angle,
                    beam_width,
                    texture,
                    sampling_weight,
                })
            }
            "directional" => {
                let direction = map.remove("direction").unwrap().as_vec()?;
                let irradiance = read_value(
                    &mut map,
                    "irradiance",
                    Value::Spectrum(Spectrum::from_f32(1.0)),
                )
                .as_spectrum()?;
                Ok(Emitter::Directional {
                    to_world,
                    direction,
                    irradiance,
                    sampling_weight,
                })
            }
            "collimated" => {
                let power = map.remove("power").unwrap().as_spectrum()?;
                Ok(Emitter::Collimated {
                    to_world,
                    power,
                    sampling_weight,
                })
            }
            "constant" => {
                let radiance = map.remove("radiance").unwrap().as_spectrum()?;
                Ok(Emitter::Constant {
                    radiance,
                    sampling_weight,
                })
            }
            "envmap" => {
                let filename = map.remove("filename").unwrap().as_string()?;
                let scale = read_value(&mut map, "scale", Value::Float(1.0)).as_float()?;
                let gamma = match map.remove("gamma") {
                    None => None,
                    Some(v) => Some(v.as_float()?),
                };
                let cache = match map.remove("cache") {
                    None => None,
                    Some(v) => Some(v.as_bool()?),
                };
                Ok(Emitter::EnvMap {
                    to_world,
                    filename,
                    scale,
                    gamma,
                    cache,
                    sampling_weight,
                })
            }
            "sunsky" | "sun" | "sky" => {
                let turbidity = read_value(&mut map, "turbidity", Value::Float(3.0)).as_float()?;
                let resolution =
                    read_value(&mut map, "turbidity", Value::Integer(512)).as_int()? as u32;
                let sun_direction = SunDirection::parse(&mut map, defaults)?;
                let (sun, sky) = match emitter_type {
                    "sun" => {
                        let scale = read_value(&mut map, "scale", Value::Float(1.0)).as_float()?;
                        let radius_scale =
                            read_value(&mut map, "sunRadiusScale", Value::Float(1.0)).as_float()?;
                        (
                            Some(SunEmitterParam {
                                scale,
                                radius_scale,
                            }),
                            None,
                        )
                    }
                    "sky" => {
                        let scale = read_value(&mut map, "scale", Value::Float(1.0)).as_float()?;
                        let stretch =
                            read_value(&mut map, "stretch", Value::Float(1.0)).as_float()?;
                        let albedo = read_value(
                            &mut map,
                            "stretch",
                            Value::Spectrum(Spectrum::from_f32(0.15)),
                        )
                        .as_spectrum()?;
                        (
                            None,
                            Some(SkyEmitterParam {
                                scale,
                                stretch,
                                albedo,
                            }),
                        )
                    }
                    "sunsky" => {
                        // Sky
                        let sky_scale =
                            read_value(&mut map, "skyScale", Value::Float(1.0)).as_float()?;
                        let stretch =
                            read_value(&mut map, "stretch", Value::Float(1.0)).as_float()?;
                        let albedo = read_value(
                            &mut map,
                            "stretch",
                            Value::Spectrum(Spectrum::from_f32(0.15)),
                        )
                        .as_spectrum()?;
                        // Sun
                        let sun_scale =
                            read_value(&mut map, "sunScale", Value::Float(1.0)).as_float()?;
                        let radius_scale =
                            read_value(&mut map, "sunRadiusScale", Value::Float(1.0)).as_float()?;
                        (
                            Some(SunEmitterParam {
                                scale: sun_scale,
                                radius_scale,
                            }),
                            Some(SkyEmitterParam {
                                scale: sky_scale,
                                stretch,
                                albedo,
                            }),
                        )
                    }
                    _ => unimplemented!(),
                };
                Ok(Emitter::SunSky {
                    turbidity,
                    resolution,
                    sun_direction,
                    sun,
                    sky,
                    sampling_weight,
                })
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

#[derive(Debug, Clone)]
pub enum PhaseFunction {
    Isotropic,
    HG { g: f32 },
}

impl PhaseFunction {
    pub fn parse<R: Read>(
        events: &mut Events<R>,
        defaults: &HashMap<String, String>,
        phase_type: &str,
    ) -> Result<Self> {
        let (mut map, refs) = values(events, defaults, true)?;
        assert!(refs.is_empty());
        match phase_type {
            "isotropic" => Ok(PhaseFunction::Isotropic),
            "hg" => {
                let g = read_value(&mut map, "g", Value::Float(0.8)).as_float()?;
                Ok(PhaseFunction::HG { g })
            }
            _ => panic!("[ERROR] Uncovered {} phasefunction type", phase_type),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Medium {
    // TODO: Add material looking
    Homogenous {
        sigma_a: Spectrum,
        sigma_s: Spectrum,
        scale: f32,
        phase: PhaseFunction,
    }, //TODO: Add heterogenous
}

impl Medium {
    pub fn parse<R: Read>(
        events: &mut Events<R>,
        defaults: &HashMap<String, String>,
        medium_type: &str,
    ) -> Result<Self> {
        let mut phase = None;
        let f = |events: &mut Events<R>, t: &str, attrs: HashMap<String, String>| -> Result<bool> {
            match t {
                "phase" => {
                    let phase_type = attrs.get("type").unwrap();
                    phase = Some(PhaseFunction::parse(events, defaults, phase_type)?);
                }
                _ => panic!("Unexpected token!"),
            }
            Ok(true)
        };
        let (mut map, refs) = values_fn(events, defaults, true, f)?;

        assert!(refs.is_empty());
        let phase = phase.unwrap_or(PhaseFunction::Isotropic);
        match medium_type {
            "homogeneous" => {
                let sigma_a =
                    read_value(&mut map, "sigmaA", Value::Spectrum(Spectrum::from_f32(1.0)))
                        .as_spectrum()?;
                let sigma_s =
                    read_value(&mut map, "sigmaS", Value::Spectrum(Spectrum::from_f32(1.0)))
                        .as_spectrum()?;
                let scale = read_value(&mut map, "scale", Value::Float(1.0)).as_float()?;

                Ok(Medium::Homogenous {
                    sigma_a,
                    sigma_s,
                    scale,
                    phase,
                })
            }
            _ => panic!("[ERROR] Uncovered {} medium type", medium_type),
        }
    }
}

#[derive(Debug)]
pub struct ShapeOption {
    pub flip_normal: bool, // false
    pub bsdf: Option<BSDF>,
    pub to_world: Option<Transform>,
    pub emitter: Option<AreaEmitter>,
    pub interior: Option<Medium>,
    pub exterior: Option<Medium>,
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
        shape: String,
        option: ShapeOption,
    },
}
impl Shape {
    pub fn parse<R: Read>(
        events: &mut Events<R>,
        defaults: &HashMap<String, String>,
        shape_type: &str,
        scene: &mut Scene,
    ) -> Result<Self> {
        // FIXME: Can be object or references
        //  if there are references, we need to handle them
        //  differentely.
        let mut bsdf = None;
        let mut to_world = None;
        let mut emitter = None;
        let mut shapes = vec![];
        let mut shape = None; // Only for instance

        let f = |events: &mut Events<R>, t: &str, attrs: HashMap<String, String>| -> Result<bool> {
            match t {
                "bsdf" => {
                    let bsdf_type = attrs.get("type").unwrap();
                    bsdf = Some(BSDF::parse(events, defaults, bsdf_type, scene)?);
                }
                "transform" => to_world = Some(Transform::parse(events)),
                "emitter" => {
                    let emitter_type = attrs.get("type").unwrap();
                    emitter = Some(Emitter::parse(events, defaults, emitter_type)?.as_area());
                }
                "shape" => {
                    let shape_type = attrs.get("type").unwrap();
                    shapes.push(Shape::parse(events, defaults, shape_type, scene)?);
                }
                _ => panic!("Unexpected token! {}", t),
            }
            Ok(true)
        };

        let (mut map, refs) = values_fn(events, defaults, true, f)?;
        for r in refs {
            // BSDF are supported for refs without type
            if let Some(v) = scene.bsdfs.get(&r) {
                bsdf = Some(v.clone());
                continue;
            }
            // For instance, we check if the shape exists
            if scene.shapes_id.get(&r).is_some() {
                shape = Some(r);
                continue;
            }

            println!("Ignore reference for shape: {}",r);
        }

        // Try to fill things that missing
        if bsdf.is_none() {
            match map.remove("bsdf") {
                None => {}
                Some(v) => bsdf = Some(scene.bsdfs.get(&v.as_ref()?).unwrap().clone()),
            }
        }
        if emitter.is_none() {
            // TODO: Need to implement ref to emitters
            assert!(map.remove("emitter").is_none());
        }

        let flip_normal = read_value(&mut map, "flipNormal", Value::Boolean(false)).as_bool()?;

        // Convert to Medium
        let interior = map.remove("interior").map(|v| {
            let v = v.as_ref().unwrap();
            scene.medium.get(&v).unwrap().clone()
        });
        let exterior = map.remove("exterior").map(|v| {
            let v = v.as_ref().unwrap();
            scene.medium.get(&v).unwrap().clone()
        });

        // Create shape option (shared across all shapes)
        let option = ShapeOption {
            flip_normal,
            bsdf,
            to_world,
            emitter,
            interior,
            exterior,
        };

        // Read some values in advance to reduce the redundancy
        let face_normal = read_value(&mut map, "faceNormal", Value::Boolean(false)).as_bool()?;
        let max_smooth_angle = match map.remove("maxSmoothAngle") {
            None => None,
            Some(v) => Some(v.as_float()?),
        };

        match shape_type {
            "serialized" => {
                let filename = map.remove("filename").unwrap().as_string()?;
                let shape_index =
                    read_value(&mut map, "shapeIndex", Value::Integer(0)).as_int()? as u32;
                Ok(Shape::Serialized(SerializedShape {
                    filename,
                    shape_index,
                    face_normal,
                    max_smooth_angle,
                    option,
                }))
            }
            "obj" => {
                let filename = map.remove("filename").unwrap().as_string()?;
                let flip_tex_coords =
                    read_value(&mut map, "flipTexCoords", Value::Boolean(true)).as_bool()?;
                let collapse = read_value(&mut map, "collapse", Value::Boolean(false)).as_bool()?;
                Ok(Shape::Obj {
                    filename,
                    face_normal,
                    max_smooth_angle,
                    flip_tex_coords,
                    collapse,
                    option,
                })
            }
            "ply" => {
                let filename = map.remove("filename").unwrap().as_string()?;
                let srgb = read_value(&mut map, "srgb", Value::Boolean(true)).as_bool()?;
                Ok(Shape::Ply {
                    filename,
                    face_normal,
                    max_smooth_angle,
                    srgb,
                    option,
                })
            }
            "cube" => Ok(Shape::Cube { option }),
            "sphere" => {
                let center =
                    read_value(&mut map, "center", Value::Point(Point3::new(0.0, 0.0, 0.0)))
                        .as_point()?;
                let radius = read_value(&mut map, "radius", Value::Float(1.0)).as_float()?;
                Ok(Shape::Sphere {
                    center,
                    radius,
                    option,
                })
            }
            "cylinder" => {
                let p0 = read_value(&mut map, "p0", Value::Point(Point3::new(0.0, 0.0, 0.0)))
                    .as_point()?;
                let p1 = read_value(&mut map, "p1", Value::Point(Point3::new(0.0, 0.0, 1.0)))
                    .as_point()?;
                let radius = read_value(&mut map, "radius", Value::Float(1.0)).as_float()?;
                Ok(Shape::Cylinder {
                    p0,
                    p1,
                    radius,
                    option,
                })
            }
            "rectangle" => Ok(Shape::Rectangle { option }),
            "disk" => Ok(Shape::Disk { option }),
            "shapegroup" => {
                assert!(option.bsdf.is_none());
                assert!(option.to_world.is_none());
                assert!(option.emitter.is_none());
                Ok(Shape::ShapeGroup { shapes })
            }
            "instance" => {
                let shape = shape.unwrap();
                Ok(Shape::Instance { shape, option })
            }
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
    pub fn parse<R: Read>(
        events: &mut Events<R>,
        defaults: &HashMap<String, String>,
    ) -> Result<Self> {
        let (mut map, refs) = values(events, defaults, false)?;
        assert!(refs.is_empty());
        let height = map.remove("height").unwrap().as_int()? as u32;
        let width = map.remove("width").unwrap().as_int()? as u32;
        Ok(Self { height, width })
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

                        trans = trans * Matrix4::from_axis_angle(axis, Deg(-angle));
                        opened += 1;
                    }
                    "matrix" => {
                        let values = found_attrib(&attributes, "value").unwrap();
                        let values = values.trim();
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
                        #[rustfmt::skip]
                        let matrix = Matrix4::new(
                            m00, m01, m02, m03, 
                            m10, m11, m12, m13, 
                            m20, m21, m22, m23, 
                            m30, m31, m32, m33,
                        );

                        trans = trans * matrix;
                        opened += 1;
                    }
                    "lookAt" | "lookat" => {
                        let origin = found_attrib_vec(&attributes, "origin").unwrap();
                        let target = found_attrib_vec(&attributes, "target").unwrap();
                        let up = found_attrib_vec(&attributes, "up").unwrap();

                        // Conversion
                        let dir = (target - origin).normalize();
                        let left = -dir.cross(up.normalize()).normalize();
                        let new_up = dir.cross(left);

                        // use cgmath::Transform;
                        let matrix = Matrix4::new(
                            left.x, left.y, left.z, 0.0, new_up.x, new_up.y, new_up.z, 0.0, dir.x,
                            dir.y, dir.z, 0.0, origin.x, origin.y, origin.z, 1.0,
                        )
                        .transpose();

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

        Transform(trans.transpose())
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
    pub fn parse<R: Read>(
        events: &mut Events<R>,
        defaults: &HashMap<String, String>,
        sensor_type: &str,
    ) -> Result<Self> {
        assert_eq!(sensor_type, "perspective");

        // Use closure to initialize these extra stuffs
        let mut film = None;
        let mut to_world = Transform(Matrix4::one());
        let f = |events: &mut Events<R>, t: &str, _: HashMap<String, String>| -> Result<bool> {
            match t {
                "film" => film = Some(Film::parse(events, defaults)?),
                "transform" => to_world = Transform::parse(events),
                "sampler" => {
                    // Sampler is not necessary to parse
                    skipping_entry(events);
                }
                _ => panic!("Unexpected token!"),
            }
            Ok(true)
        };

        let (mut map, refs) = values_fn(events, defaults, false, f)?;
        assert!(refs.is_empty());

        let fov = map
            .remove("fov")
            .expect("Impossible to found 'fov'")
            .as_float()?;
        let fov_axis =
            read_value(&mut map, "fovAxis", Value::String("x".to_string())).as_string()?;
        let shutter_open = read_value(&mut map, "shutterOpen", Value::Float(0.0)).as_float()?;
        let shutter_close = read_value(&mut map, "shutterClose", Value::Float(0.0)).as_float()?;
        let near_clip = read_value(&mut map, "nearClip", Value::Float(0.01)).as_float()?;
        let far_clip = read_value(&mut map, "farClip", Value::Float(1000.0)).as_float()?;

        Ok(Self {
            fov,
            fov_axis,
            shutter_open,
            shutter_close,
            near_clip,
            far_clip,
            film: film.unwrap(),
            to_world,
        })
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
    pub medium: HashMap<String, Medium>,
}
impl Scene {
    // TODO:
    // pub fn shapes(&self) -> dyn Iterator<Item = &Shape> {
    //     self.shapes_id.iter().map(|(k,v)| v).chain(self.shapes_unamed.iter())
    // }
}

#[cfg(feature = "ply")]
pub mod ply;
#[cfg(feature = "serialized")]
pub mod serialized;

fn parse_scene(filename: &str, mut scene: &mut Scene) -> Result<()> {
    let file = File::open(filename).expect(&format!("Impossible to open {}", filename));
    let file = BufReader::new(file);

    let parser = EventReader::new(file);
    let mut defaults = HashMap::new();

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
                    let bsdf_type = found_attrib_or_error(&attributes, "type", "bsdf")?;
                    let bsdf_id = found_attrib_or_error(&attributes, "id", "bsdf");
                    if bsdf_id.is_err() {
                        skipping_entry(&mut iter);
                        warn!("Skipping bsdf without id {:?}", bsdf_type);
                        continue;
                    }
                    let bsdf_id = bsdf_id.unwrap();
                    let bsdf = BSDF::parse(&mut iter, &defaults, &bsdf_type, &mut scene)?;
                    scene.bsdfs.insert(bsdf_id, bsdf);
                }
                "texture" => {
                    let texture_id = found_attrib_or_error(&attributes, "id", "texture")?;
                    let texture_type = found_attrib_or_error(&attributes, "type", "texture")?;
                    let texture = Texture::parse(&mut iter, &defaults, &texture_type)?;
                    scene.textures.insert(texture_id, texture);
                }
                "sensor" => {
                    let sensor_type = found_attrib_or_error(&attributes, "type", "sensor")?;
                    let sensor = Sensor::parse(&mut iter, &defaults, &sensor_type)?;
                    scene.sensors.push(sensor);
                }
                "emitter" => {
                    let emitter_type = found_attrib_or_error(&attributes, "type", "emitter")?;
                    let emitter = Emitter::parse(&mut iter, &defaults, &emitter_type)?;
                    scene.emitters.push(emitter);
                }
                "default" => {
                    // name="faceNormalsFlag" value="false"/>
                    let name = found_attrib_or_error(&attributes, "name", "default")?;
                    let value = found_attrib_or_error(&attributes, "value", "default")?;
                    defaults.insert(name, value);
                }
                "scene" => {
                    // Nothing to do
                }
                "integrator" => {
                    // We ignoring the integrator
                    // as for scene parsing, it gives us no information
                    skipping_entry(&mut iter);
                }
                "medium" => {
                    let medium_id = found_attrib_or_error(&attributes, "id", "texture")?;
                    let medium_type = found_attrib_or_error(&attributes, "type", "texture")?;
                    let medium = Medium::parse(&mut iter, &defaults, &medium_type)?;
                    scene.medium.insert(medium_id, medium);
                }
                "include" => {
                    // Read a new file
                    let other_filename = found_attrib_or_error(&attributes, "filename", "include")?;
                    let filename = std::path::Path::new(filename)
                        .parent()
                        .unwrap()
                        .join(std::path::Path::new(&other_filename));
                    parse_scene(
                        &filename.into_os_string().into_string().unwrap(),
                        &mut scene,
                    )?;
                    skipping_entry(&mut iter);
                }
                "shape" => {
                    let shape_type = found_attrib_or_error(&attributes, "type", "shape")?;
                    let shape_id = found_attrib(&attributes, "id");
                    let shape = Shape::parse(&mut iter, &defaults, &shape_type, &mut scene)?;
                    match shape_id {
                        Some(v) => {
                            scene.shapes_id.insert(v, shape);
                        }
                        None => {
                            scene.shapes_unamed.push(shape);
                        }
                    }
                }
                "ply" => {
                    // This flag is from Mitsuba2 (exporter from blender)
                    let filename = found_attrib_or_error(&attributes, "filename", "ply")?;
                    scene.shapes_unamed.push(Shape::Ply {
                        filename,
                        face_normal: false,
                        max_smooth_angle: None,
                        srgb: true,
                        option: ShapeOption {
                            flip_normal: false,
                            bsdf: None,
                            to_world: None,
                            emitter: None,
                            interior: None,
                            exterior: None,
                        },
                    });
                }
                _ => println!("Unsupported primitive type {} {:?}", name, attributes),
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

    Ok(())
}

pub fn parse(file: &str) -> Result<Scene> {
    let mut scene = Scene {
        bsdfs: HashMap::new(),
        textures: HashMap::new(),
        shapes_id: HashMap::new(),
        shapes_unamed: Vec::new(),
        sensors: Vec::new(),
        emitters: Vec::new(),
        medium: HashMap::new(),
    };
    parse_scene(file, &mut scene)?;
    Ok(scene)
}

#[cfg(test)]
mod tests {
    fn print_scene(scene: Result<crate::Scene, crate::Error>) {
        let scene = scene.unwrap();
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

    #[test]
    fn issue_golden() {
        let s = "./data/issue_golden.xml";
        print_scene(crate::parse(s));
    }

    #[test]
    fn texture_checkboard() {
        let s = "./data/veach-door.xml";
        print_scene(crate::parse(s));
    }

    #[test]
    fn bidir() {
        let s = "./data/bidir.xml";
        print_scene(crate::parse(s));
    }

    #[test]
    fn mitsuba2() {
        let s = "./data/blender_mts2.xml";
        print_scene(crate::parse(s));
    }

    #[test]
    fn default() {
        let s = "./data/da_morton.xml";
        print_scene(crate::parse(s));
    }

    #[test]
    fn include_complex() {
        let s = "./data/necklace_include.xml";
        print_scene(crate::parse(s));
    }

    #[test]
    fn spectrum_to_rgb_failed() {
        let s = crate::Spectrum {
            value: "0.01, 0.2".to_string(),
        };
        let res = s.as_rgb();
        assert!(res.is_err())
    }

    #[test]
    fn spectrum_to_rgb_ok() {
        let s = crate::Spectrum {
            value: "0.01, 0.2, 0.3".to_string(),
        };
        let res = s.as_rgb();
        assert!(res.is_ok())
    }
}
