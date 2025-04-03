use rbx_dom_weak::types::{BrickColor, CFrame, Color3, Enum, Matrix3, Ref, UDim, UDim2, Variant, Vector3};
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
    #[serde(default)]
    pub subtract: Vec<String>,  // Paths to instances that should be removed
}

#[derive(Serialize, Deserialize)]
pub struct JsonInstance {
    pub class: String,
    pub name: String,
    pub properties: HashMap<String, JsonProperty>,
    #[serde(default)]
    pub children: Vec<JsonInstance>,
    #[serde(default)]
    pub target_parent: Option<String>,
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

/// Parse a Roblox XML string into a WeakDom
pub fn parse_roblox_str(xml: &str) -> Result<WeakDom, Box<dyn Error>> {
    let place = rbx_xml::from_str_default(xml)?;
    Ok(place)
}

/// Add instances from JSON to the Roblox place
/// parent_id should be the DataModel reference for proper structure
pub fn json_to_weakdom(dom: &mut WeakDom, json: &Modification, parent_id: Ref) -> Result<(), Box<dyn Error>> {
    println!("Adding instances to Roblox place...");
    
    // Maps service names to their refs
    let mut service_refs: HashMap<String, Ref> = HashMap::new();
    
    // Get the DataModel root
    let data_model_id = parent_id;
    
    // Find or create Workspace
    let workspace_id = find_or_create_service(dom, data_model_id, "Workspace")?;
    service_refs.insert("Workspace".to_string(), workspace_id);
    
    // Define common Roblox services
    let common_services = [
        "StarterPlayer", "Lighting", "ReplicatedStorage", "ServerScriptService", 
        "ServerStorage", "SoundService", "Chat", "Teams"
    ];
    
    // Find or create common services
    for service_name in common_services.iter() {
        let service_id = find_or_create_service(dom, data_model_id, service_name)?;
        service_refs.insert(service_name.to_string(), service_id);
    }
    
    // Special case: Find or create StarterPlayerScripts under StarterPlayer
    // First, get the ref without keeping a borrow on service_refs
    let starter_player_id_opt = service_refs.get("StarterPlayer").copied();
    
    if let Some(starter_player_id) = starter_player_id_opt {
        let starter_player_scripts_id = find_or_create_service(dom, starter_player_id, "StarterPlayerScripts")?;
        service_refs.insert("StarterPlayerScripts".to_string(), starter_player_scripts_id);
        
        let starter_character_scripts_id = find_or_create_service(dom, starter_player_id, "StarterCharacterScripts")?;
        service_refs.insert("StarterCharacterScripts".to_string(), starter_character_scripts_id);
    }
    
    // Process all subtract operations first
    if !json.subtract.is_empty() {
        println!("Processing {} removal operations...", json.subtract.len());
        for path in &json.subtract {
            println!("Trying to remove instance at path: {}", path);
            if let Some(instance_id) = find_instance_by_path(dom, data_model_id, path) {
                // Remove the instance
                if let Err(e) = remove_instance(dom, instance_id) {
                    println!("Warning: Failed to remove instance at '{}': {}", path, e);
                } else {
                    println!("Successfully removed instance at path: {}", path);
                }
            } else {
                println!("Warning: Could not find instance at path '{}' to remove", path);
            }
        }
    }
    
    // Process all top-level instances
    for instance in &json.add {
        // Debug output to see what's being received
        println!("Instance: {}, target_parent: {:?}", instance.name, instance.target_parent);
        
        // Determine the parent based on target_parent, defaulting to Workspace
        let target_parent = match &instance.target_parent {
            Some(target) => {
                println!("  - Target parent specified: {}", target);
                
                // First, check if it's a direct service reference
                if service_refs.contains_key(target) {
                    println!("  - Found matching service for '{}'", target);
                    *service_refs.get(target).unwrap()
                } else {
                    // If not a service, try to find it by path
                    match find_instance_by_path(dom, data_model_id, target) {
                        Some(id) => {
                            println!("  - Found instance at path '{}'", target);
                            id
                        }
                        None => {
                            println!("  - Could not find target '{}', defaulting to Workspace", target);
                            workspace_id
                        }
                    }
                }
            }
            None => {
                println!("  - No target_parent specified, defaulting to Workspace");
                workspace_id
            }
        };
        
        // Create each instance and all its children recursively
        process_instance_with_children(dom, instance, target_parent)?;
    }
    
    println!("Successfully processed all operations!");
    Ok(())
}

/// Find a service by name or create it if it doesn't exist
fn find_or_create_service(dom: &mut WeakDom, parent_id: Ref, service_name: &str) -> Result<Ref, Box<dyn Error>> {
    // Try to find the service among the parent's children
    let parent = dom.get_by_ref(parent_id)
        .ok_or_else(|| format!("Invalid parent reference: {:?}", parent_id))?;
    for &child_id in parent.children() {
        let instance = dom.get_by_ref(child_id)
            .ok_or_else(|| format!("Invalid child reference: {:?}", child_id))?;
        if instance.name == service_name {
            println!("Found existing service: {}", service_name);
            return Ok(child_id);
        }
    }
    
    // If not found, create the service
    println!("Creating service: {}", service_name);
    let service_id = dom.insert(parent_id, InstanceBuilder::new(service_name).with_name(service_name));
    
    Ok(service_id)
}

/// Find instance by path (e.g., "Workspace/Models/House")
fn find_instance_by_path(dom: &WeakDom, start_id: Ref, path: &str) -> Option<Ref> {
    let path_parts: Vec<&str> = path.split('/').collect();
    
    // If path is empty, return the starting point
    if path_parts.is_empty() || (path_parts.len() == 1 && path_parts[0].is_empty()) {
        return Some(start_id);
    }
    
    // Start with the first part of the path
    let mut current_id = if path_parts[0] == "DataModel" {
        // If path starts with DataModel, skip it and use start_id (which should be DataModel)
        if path_parts.len() == 1 {
            return Some(start_id);
        }
        start_id
    } else {
        // Otherwise, find the first part as a direct child of start_id
        let service_name = path_parts[0];
        
        // Check if it's a service
        match find_service(dom, start_id, service_name) {
            Some(id) => id,
            None => return None,
        }
    };
    
    // Traverse the rest of the path
    for &part in &path_parts[if path_parts[0] == "DataModel" { 2 } else { 1 }..] {
        let parent = dom.get_by_ref(current_id).unwrap();
        
        let mut found = false;
        for &child_id in parent.children() {
            let child = dom.get_by_ref(child_id).unwrap();
            if child.name == part {
                current_id = child_id;
                found = true;
                break;
            }
        }
        
        if !found {
            println!("Could not find '{}' in path '{}'", part, path);
            return None;
        }
    }
    
    Some(current_id)
}

/// Find a service by name or None if it doesn't exist
fn find_service(dom: &WeakDom, parent_id: Ref, service_name: &str) -> Option<Ref> {
    let parent = dom.get_by_ref(parent_id).unwrap();
    for &child_id in parent.children() {
        let instance = dom.get_by_ref(child_id).unwrap();
        if instance.name == service_name {
            return Some(child_id);
        }
    }
    None
}

/// Process an instance and all its children recursively
fn process_instance_with_children(dom: &mut WeakDom, instance: &JsonInstance, parent_id: Ref) -> Result<Ref, Box<dyn Error>> {
    // Add the current instance
    println!("Processing instance: {} ({})", instance.name, instance.class);
    let instance_id = add_instance_to_weakdom(dom, instance, parent_id)?;
    
    // Process all children recursively
    if !instance.children.is_empty() {
        println!("Processing {} children for {}", instance.children.len(), instance.name);
        for child in &instance.children {
            process_instance_with_children(dom, child, instance_id)?;
        }
    }
    
    Ok(instance_id)
}

/// Add a single instance to WeakDom
pub fn add_instance_to_weakdom(
    dom: &mut WeakDom,
    json: &JsonInstance,
    parent_id: Ref,
) -> Result<Ref, Box<dyn Error>> {
    println!("Creating instance: {} ({})", json.name, json.class);
    let mut builder = InstanceBuilder::new(&json.class).with_name(&json.name);

    let is_script = json.class == "Script" || 
                    json.class == "LocalScript" || 
                    json.class == "ModuleScript";

    // Add properties to the instance builder
    for (prop_name, prop) in &json.properties {
        // Special case for Script Source property
        if is_script && prop_name == "Source" {
            if let Some(source) = prop.value.as_str() {
                builder = builder.with_property("Source", Variant::String(source.to_string()));
                continue;
            }
        }

        println!("  - Adding property: {}", prop_name);
        let variant = match prop.type_name.as_str() {
            "Vector3" => {
                if let Value::Array(vec) = &prop.value {
                    if vec.len() == 3 {
                        let x = vec[0].as_f64().unwrap_or(0.0) as f32;
                        let y = vec[1].as_f64().unwrap_or(0.0) as f32;
                        let z = vec[2].as_f64().unwrap_or(0.0) as f32;
                        
                        println!("    - Vector3: [{}, {}, {}]", x, y, z);
                        Variant::Vector3(Vector3::new(x, y, z))
                    } else {
                        return Err("Vector3 must have 3 components".into());
                    }
                } else if let Value::Object(obj) = &prop.value {
                    // Handle Vector3 as an object with x, y, z properties
                    let x = obj.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = obj.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let z = obj.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    
                    println!("    - Vector3 (object): [{}, {}, {}]", x, y, z);
                    Variant::Vector3(Vector3::new(x, y, z))
                } else {
                    return Err("Vector3 must be an array or object".into());
                }
            }
            "CFrame" => {
                // Create verbose debug output to diagnose the issue
                println!("    - Raw CFrame value: {:?}", prop.value);
                
                if let Value::Object(obj) = &prop.value {
                    // Try to extract position
                    if let Some(pos_val) = obj.get("position") {
                        println!("    - Position value: {:?}", pos_val);
                        
                        let pos = if let Some(pos_arr) = pos_val.as_array() {
                            if pos_arr.len() == 3 {
                                let x = pos_arr[0].as_f64().unwrap_or(0.0) as f32;
                                let y = pos_arr[1].as_f64().unwrap_or(0.0) as f32;
                                let z = pos_arr[2].as_f64().unwrap_or(0.0) as f32;
                                Vector3::new(x, y, z)
                            } else {
                                return Err("CFrame position must have 3 components".into());
                            }
                        } else if let Some(pos_obj) = pos_val.as_object() {
                            // Handle position as an object with x, y, z properties
                            let x = pos_obj.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                            let y = pos_obj.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                            let z = pos_obj.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                            Vector3::new(x, y, z)
                        } else {
                            return Err("CFrame position must be an array or object".into());
                        };

                        // Log the position to verify
                        println!("    - CFrame position: [{}, {}, {}]", pos.x, pos.y, pos.z);

                        // Extract rotation (optional)
                        let rot = if let Some(rot_val) = obj.get("rotation") {
                            println!("    - Rotation value: {:?}", rot_val);
                            
                            if let Some(rot_arr) = rot_val.as_array() {
                                if rot_arr.len() == 9 {
                                    // Convert all 9 values to f32
                                    let values: Vec<f32> = rot_arr.iter()
                                        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                                        .collect();
                                    
                                    println!("    - Using rotation matrix: {:?}", values);
                                    
                                    Matrix3::new(
                                        Vector3::new(values[0], values[1], values[2]),
                                        Vector3::new(values[3], values[4], values[5]),
                                        Vector3::new(values[6], values[7], values[8])
                                    )
                                } else if rot_arr.len() == 3 {
                                    // Handle rotation as just angles
                                    println!("    - Using rotation angles");
                                    // For simplicity, using identity matrix when only angles provided
                                    Matrix3::identity()
                                } else {
                                    // Default to identity matrix if rotation not provided correctly
                                    println!("    - Using identity matrix for rotation (incorrect length)");
                                    Matrix3::identity()
                                }
                            } else {
                                // Default to identity matrix
                                println!("    - Using identity matrix for rotation (not an array)");
                                Matrix3::identity()
                            }
                        } else {
                            // If rotation is missing, use identity matrix
                            println!("    - Using identity matrix for rotation (missing)");
                            Matrix3::identity()
                        };

                        // Create the CFrame with position and rotation
                        let cframe = CFrame::new(pos, rot);
                        println!("    - Final CFrame position: [{}, {}, {}]", 
                            cframe.position.x, cframe.position.y, cframe.position.z);
                        
                        Variant::CFrame(cframe)
                    } else {
                        return Err("CFrame missing position".into());
                    }
                } else {
                    return Err("CFrame must be an object with position and rotation".into());
                }
            }
            "String" => {
                if let Value::String(s) = &prop.value {
                    Variant::String(s.clone())
                } else {
                    // Also try to convert numbers or other types to string
                    Variant::String(prop.value.to_string())
                }
            }
            "BrickColor" => {
                if let Value::Number(n) = &prop.value {
                    // Convert to u16 as required by from_number
                    let number = n.as_u64().unwrap_or(1) as u16;
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
            "Enum" => {
                if let Value::Number(n) = &prop.value {
                    Variant::Enum(Enum::from_u32(n.as_u64().unwrap_or(1).try_into().unwrap()))
                } else {
                    return Err("Enum must be a numeric value".into());
                }
            }
            "Color3" => {
                if let Value::Array(vec) = &prop.value {
                    if vec.len() == 3 {
                        Variant::Color3(Color3::new(
                            vec[0].as_f64().unwrap_or(0.0) as f32,
                            vec[1].as_f64().unwrap_or(0.0) as f32,
                            vec[2].as_f64().unwrap_or(0.0) as f32,
                        ))
                    } else {
                        return Err("Color3 must have 3 components".into());
                    }
                } else {
                    return Err("Color3 must be an array".into());
                }
            }
            "UDim2" => {
                if let Value::Array(vec) = &prop.value {
                    if vec.len() == 4 {
                        // UDim2::new requires two UDim values (x and y)
                        // Each UDim has a scale (float) and offset (integer)
                        let x = UDim::new(
                            vec[0].as_f64().unwrap_or(0.0) as f32,
                            vec[1].as_i64().unwrap_or(0) as i32
                        );
                        let y = UDim::new(
                            vec[2].as_f64().unwrap_or(0.0) as f32,
                            vec[3].as_i64().unwrap_or(0) as i32
                        );
                        Variant::UDim2(UDim2::new(x, y))
                    } else {
                        return Err("UDim2 must have 4 components [xScale, xOffset, yScale, yOffset]".into());
                    }
                } else {
                    return Err("UDim2 must be an array".into());
                }
            }
            // Add more types as needed
            _ => continue,
        };
        builder = builder.with_property(prop_name, variant);
    }

    // Insert the instance into the DOM
    let instance_id = dom.insert(parent_id, builder);
    println!("  Created instance with ID: {:?}", instance_id);
    
    Ok(instance_id)
}

/// Remove an instance and all its children from the WeakDom
fn remove_instance(dom: &mut WeakDom, instance_id: Ref) -> Result<(), Box<dyn Error>> {
    // Get the instance name for logging
    let instance_name = match dom.get_by_ref(instance_id) {
        Some(instance) => instance.name.clone(),
        None => return Err(format!("Instance with ref {:?} not found", instance_id).into()),
    };
    
    // Remove the instance
    dom.destroy(instance_id);
    println!("Removed instance: {}", instance_name);
    
    Ok(())
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
