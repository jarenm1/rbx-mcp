use rbx_dom_weak::types::{BrickColor, CFrame, Matrix3, Ref, Variant, Vector3};
use rbx_dom_weak::{InstanceBuilder, WeakDom};
use serde_json::Value;
use serde::{Serialize, Deserialize};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct Modification {
    pub add: Vec<JsonInstance>,
}

#[derive(Serialize, Deserialize)]
pub struct JsonInstance {
    pub class: String,
    pub name: String,
    pub properties: HashMap<String, JsonProperty>,
    #[serde(default)]
    pub children: Vec<JsonInstance>,
}

#[derive(Serialize, Deserialize)]
pub struct JsonProperty {
    #[serde(rename = "type")]
    pub type_name: String,
    pub value: Value,
}

/// Parse a Roblox XML file into a WeakDom
pub fn parse_roblox_file(path: impl AsRef<Path>) -> Result<WeakDom, Box<dyn Error>> {
    let file = BufReader::new(File::open(path)?);
    let place = rbx_xml::from_reader_default(file)?;
    Ok(place)
}

pub fn json_to_weakdom(dom: &mut WeakDom, json: &Modification, parent_id: Ref) -> Result<(), Box<dyn Error>> {
    for instance in &json.add {
        let instance_id = add_instance_to_weakdom(dom, instance, parent_id)?;
        for children in &instance.children {
            add_instance_to_weakdom(dom, children, instance_id)?;
        }
    }
    Ok(())
}

/// Add a single instance to WeakDom
pub fn add_instance_to_weakdom(
    dom: &mut WeakDom,
    json: &JsonInstance,
    parent_id: Ref,
) -> Result<Ref, Box<dyn Error>> {
    let mut builder = InstanceBuilder::new(&json.class).with_name(&json.name);

    for (prop_name, prop) in &json.properties {
        let variant = match prop.type_name.as_str() {
            "Vector3" => {
                if let Value::Array(vec) = &prop.value {
                    if vec.len() == 3 {
                        Variant::Vector3(Vector3::new(
                            vec[0].as_f64().unwrap_or(0.0) as f32,
                            vec[1].as_f64().unwrap_or(0.0) as f32,
                            vec[2].as_f64().unwrap_or(0.0) as f32,
                        ))
                    } else {
                        return Err("Vector3 must have 3 components".into());
                    }
                } else {
                    return Err("Vector3 must be an array".into());
                }
            }
            "CFrame" => {
                if let Value::Object(obj) = &prop.value {
                    let pos = obj.get("position").and_then(|v| v.as_array()).ok_or("CFrame missing position")?;
                    let rot = obj.get("rotation").and_then(|v| v.as_array()).ok_or("CFrame missing rotation")?;
                    if pos.len() == 3 && rot.len() == 9 {
                        // Matrix3 expects three Vector3 values for x, y, and z axes
                        Variant::CFrame(CFrame::new(
                            Vector3::new(
                                pos[0].as_f64().unwrap_or(0.0) as f32,
                                pos[1].as_f64().unwrap_or(0.0) as f32,
                                pos[2].as_f64().unwrap_or(0.0) as f32,
                            ),
                            Matrix3::new(
                                Vector3::new(
                                    rot[0].as_f64().unwrap_or(0.0) as f32,
                                    rot[1].as_f64().unwrap_or(0.0) as f32,
                                    rot[2].as_f64().unwrap_or(0.0) as f32,
                                ),
                                Vector3::new(
                                    rot[3].as_f64().unwrap_or(0.0) as f32,
                                    rot[4].as_f64().unwrap_or(0.0) as f32,
                                    rot[5].as_f64().unwrap_or(0.0) as f32,
                                ),
                                Vector3::new(
                                    rot[6].as_f64().unwrap_or(0.0) as f32,
                                    rot[7].as_f64().unwrap_or(0.0) as f32,
                                    rot[8].as_f64().unwrap_or(0.0) as f32,
                                ),
                            ),
                        ))
                    } else {
                        return Err("CFrame position must have 3 components, rotation 9".into());
                    }
                } else {
                    return Err("CFrame must be an object with position and rotation".into());
                }
            }
            "String" => {
                if let Value::String(s) = &prop.value {
                    Variant::String(s.clone())
                } else {
                    return Err("String must be a string value".into());
                }
            }
            "BrickColor" => {
                if let Value::Number(n) = &prop.value {
                    // Convert to u16 as required by from_number
                    let number = n.as_u64().unwrap_or(0) as u16;
                    match BrickColor::from_number(number) {
                        Some(color) => Variant::BrickColor(color),
                        None => return Err(format!("Invalid BrickColor number: {}", number).into())
                    }
                } else {
                    return Err("BrickColor must be a number".into());
                }
            }
            "Bool" => {
                if let Value::Bool(b) = &prop.value {
                    Variant::Bool(*b)
                } else {
                    return Err("Bool must be a boolean".into());
                }
            }
            "Number" | "Float" | "Float32" => {
                if let Value::Number(n) = &prop.value {
                    Variant::Float32(n.as_f64().unwrap_or(0.0) as f32)
                } else {
                    return Err("Number must be a numeric value".into());
                }
            }
            "Int" | "Int32" => {
                if let Value::Number(n) = &prop.value {
                    Variant::Int32(n.as_i64().unwrap_or(0) as i32)
                } else {
                    return Err("Int must be a numeric value".into());
                }
            }
            // Add more types as needed
            _ => continue,
        };
        builder = builder.with_property(prop_name, variant);
    }

    Ok(dom.insert(parent_id, builder))
}

/// Write a Roblox WeakDom to a file
pub fn write_roblox_file(
    path: impl AsRef<Path>,
    model: &WeakDom,
) -> Result<(), Box<dyn Error>> {
    let file = BufWriter::new(File::create(path)?);
    rbx_xml::to_writer_default(file, model, model.root().children())?;
    Ok(())
}
