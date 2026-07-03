use std::collections::{HashMap, HashSet};

use serde_json::Value;

pub fn validate_widget_config(config_json: &str) -> Vec<String> {
    let mut errors = Vec::new();

    let config: Value = match serde_json::from_str(config_json) {
        Ok(v) => v,
        Err(e) => {
            errors.push(format!("Invalid JSON in widget config: {e}"));
            return errors;
        }
    };

    let configs = if config.is_array() {
        config.as_array().unwrap().clone()
    } else {
        vec![config]
    };

    let mut seen_config_fields = HashMap::new();

    for config_obj in configs {
        if let Some(surfaces) = config_obj.get("surfaces").and_then(|s| s.as_object()) {
            for (surface_name, surface_val) in surfaces {
                if let Some(components) = surface_val.get("components").and_then(|c| c.as_object()) {
                    for (comp_name, comp_val) in components {
                        if let Some(fields) = comp_val.get("fields").and_then(|f| f.as_object()) {
                            for (field_name, field_val) in fields {
                                let value_type = field_val
                                    .get("value_type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                if value_type == "data" {
                                    let val_name = field_val
                                        .get("value")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    let pres_type = field_val
                                        .get("presentation_type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");

                                    if val_name.is_empty() {
                                        continue;
                                    }

                                    if let Some(existing_type) = seen_config_fields.get(val_name) {
                                        if existing_type != pres_type {
                                            errors.push(format!(
                                                "Data field '{val_name}' is defined multiple times in widget configuration with conflicting presentation types ('{existing_type}' vs '{pres_type}' in {surface_name}.{comp_name}.{field_name})."
                                            ));
                                        }
                                    } else {
                                        seen_config_fields.insert(val_name.to_string(), pres_type.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    errors
}

pub fn validate_sample_data(config_json: &str, sample_data: &str) -> Vec<String> {
    let mut errors = validate_widget_config(config_json);

    let config: Value = match serde_json::from_str(config_json) {
        Ok(v) => v,
        Err(_) => {
            return errors;
        }
    };

    let sample: Value = match serde_json::from_str(sample_data) {
        Ok(v) => v,
        Err(e) => {
            errors.push(format!("Invalid JSON in sample data: {e}"));
            return errors;
        }
    };

    let dynamic_arr = match sample
        .get("data")
        .and_then(|d| d.get("dynamic"))
        .and_then(|dyn_val| dyn_val.as_array())
    {
        Some(arr) => arr,
        None => {
            errors.push("Sample data is missing the 'data.dynamic' array.".to_string());
            return errors;
        }
    };

    let mut seen_sample_names = HashSet::new();
    for entry in dynamic_arr {
        if let Some(name) = entry.get("name").and_then(|n| n.as_str()) {
            if !seen_sample_names.insert(name.to_string()) {
                errors.push(format!(
                    "Duplicate data field '{name}' found in sample data."
                ));
            }
        }
    }

    let configs = if config.is_array() {
        config.as_array().unwrap().clone()
    } else {
        vec![config]
    };

    for config_obj in configs {
        if let Some(surfaces) = config_obj.get("surfaces").and_then(|s| s.as_object()) {
            for (surface_name, surface_val) in surfaces {
                if let Some(components) = surface_val.get("components").and_then(|c| c.as_object())
                {
                    for (comp_name, comp_val) in components {
                        if let Some(fields) = comp_val.get("fields").and_then(|f| f.as_object()) {
                            for (field_name, field_val) in fields {
                                let value_type = field_val
                                    .get("value_type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                if value_type == "data" {
                                    let val_name = field_val
                                        .get("value")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    let pres_type = field_val
                                        .get("presentation_type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");

                                    if val_name.is_empty() {
                                        continue;
                                    }

                                    let found_entry = dynamic_arr.iter().find(|e| {
                                        e.get("name").and_then(|n| n.as_str()) == Some(val_name)
                                    });

                                    match found_entry {
                                        None => {
                                            errors.push(format!(
                                                "Missing dynamic field '{val_name}' (required for {surface_name}.{comp_name}.{field_name})"
                                            ));
                                        }
                                        Some(entry) => {
                                            validate_dynamic_entry(
                                                &mut errors,
                                                val_name,
                                                pres_type,
                                                entry,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    errors
}

fn validate_dynamic_entry(
    errors: &mut Vec<String>,
    val_name: &str,
    pres_type: &str,
    entry: &Value,
) {
    let type_val = entry.get("type").and_then(|t| t.as_i64());
    let value_val = entry.get("value");

    match pres_type {
        "image" => {
            if type_val != Some(3) {
                errors.push(format!(
                    "Field '{val_name}' has presentation_type 'image' so 'type' must be 3 (found {type_val:?})"
                ));
            }
            match value_val {
                Some(val_obj) if val_obj.is_object() => {
                    match val_obj.get("url").and_then(|u| u.as_str()) {
                        Some(url_str) => {
                            if reqwest::Url::parse(url_str).is_err() {
                                errors.push(format!(
                                    "Field '{val_name}' url is not a valid URL: '{url_str}'"
                                ));
                            }
                        }
                        None => {
                            errors.push(format!(
                                "Field '{val_name}' 'value' object must contain a string 'url'"
                            ));
                        }
                    }
                }
                _ => {
                    errors.push(format!("Field '{val_name}' must have 'value' as an object like {{\"url\": \"...\"}}"));
                }
            }
        }
        "text" => {
            if type_val != Some(1) {
                errors.push(format!(
                    "Field '{val_name}' has presentation_type 'text' so 'type' must be 1 (found {type_val:?})"
                ));
            }
            if value_val.map_or(true, |v| !v.is_string()) {
                errors.push(format!("Field '{val_name}' must have a string 'value'"));
            }
        }
        "number" => {
            if type_val != Some(2) {
                errors.push(format!(
                    "Field '{val_name}' has presentation_type 'number' so 'type' must be 2 (found {type_val:?})"
                ));
            }
            if value_val.map_or(true, |v| !v.is_number()) {
                errors.push(format!("Field '{val_name}' must have a numeric 'value'"));
            }
        }
        _ => {}
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct DynamicFieldSpec {
    pub name: String,
    pub presentation_type: String,
}

pub fn extract_dynamic_fields(config_json: &str) -> Vec<DynamicFieldSpec> {
    let mut specs = Vec::new();
    let config: Value = match serde_json::from_str(config_json) {
        Ok(v) => v,
        Err(_) => return specs,
    };

    let configs = if config.is_array() {
        config.as_array().unwrap().clone()
    } else {
        vec![config]
    };

    for config_obj in configs {
        if let Some(surfaces) = config_obj.get("surfaces").and_then(|s| s.as_object()) {
            for (_, surface_val) in surfaces {
                if let Some(components) = surface_val.get("components").and_then(|c| c.as_object())
                {
                    for (_, comp_val) in components {
                        if let Some(fields) = comp_val.get("fields").and_then(|f| f.as_object()) {
                            for (_, field_val) in fields {
                                let value_type = field_val
                                    .get("value_type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                if value_type == "data" {
                                    let val_name = field_val
                                        .get("value")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    let pres_type = field_val
                                        .get("presentation_type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    if !val_name.is_empty()
                                        && !specs
                                            .iter()
                                            .any(|s: &DynamicFieldSpec| s.name == val_name)
                                    {
                                        specs.push(DynamicFieldSpec {
                                            name: val_name.to_string(),
                                            presentation_type: pres_type.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    specs
}

pub fn get_field_value(sample_data: &str, name: &str, pres_type: &str) -> String {
    let sample: Value = match serde_json::from_str(sample_data) {
        Ok(v) => v,
        Err(_) => return String::new(),
    };

    if let Some(dynamic_arr) = sample
        .get("data")
        .and_then(|d| d.get("dynamic"))
        .and_then(|dyn_val| dyn_val.as_array())
    {
        if let Some(entry) = dynamic_arr
            .iter()
            .find(|e| e.get("name").and_then(|n| n.as_str()) == Some(name))
        {
            if pres_type == "image" {
                return entry
                    .get("value")
                    .and_then(|v| v.get("url"))
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();
            } else if pres_type == "number" {
                match entry.get("value") {
                    Some(Value::Number(n)) => return n.to_string(),
                    Some(Value::String(s)) => return s.clone(),
                    _ => return String::new(),
                }
            } else {
                return entry
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
            }
        }
    }

    String::new()
}

pub fn update_sample_data(sample_data: &mut String, name: &str, pres_type: &str, new_val: &str) {
    let mut sample: Value = serde_json::from_str(sample_data).unwrap_or_else(|_| {
        serde_json::json!({
            "data": {
                "dynamic": []
            }
        })
    });

    if !sample.is_object() {
        sample = serde_json::json!({ "data": { "dynamic": [] } });
    }

    if sample.get("data").map_or(true, |d| !d.is_object()) {
        sample["data"] = serde_json::json!({ "dynamic": [] });
    }

    if sample["data"]
        .get("dynamic")
        .map_or(true, |arr| !arr.is_array())
    {
        sample["data"]["dynamic"] = serde_json::json!([]);
    }

    let arr = sample["data"]["dynamic"].as_array_mut().unwrap();

    let mut found_idx = None;
    for (i, e) in arr.iter().enumerate() {
        if e.get("name").and_then(|n| n.as_str()) == Some(name) {
            found_idx = Some(i);
            break;
        }
    }

    let idx = match found_idx {
        Some(i) => i,
        None => {
            let new_entry = match pres_type {
                "image" => serde_json::json!({ "name": name, "type": 3, "value": { "url": "" } }),
                "number" => serde_json::json!({ "name": name, "type": 2, "value": 0 }),
                _ => serde_json::json!({ "name": name, "type": 1, "value": "" }),
            };
            arr.push(new_entry);
            arr.len() - 1
        }
    };

    let target_entry = &mut arr[idx];

    match pres_type {
        "image" => {
            target_entry["type"] = serde_json::json!(3);
            if target_entry.get("value").map_or(true, |v| !v.is_object()) {
                target_entry["value"] = serde_json::json!({ "url": new_val });
            } else {
                target_entry["value"]["url"] = serde_json::Value::String(new_val.to_string());
            }
        }
        "number" => {
            target_entry["type"] = serde_json::json!(2);
            if let Ok(num) = new_val.parse::<f64>() {
                if num.fract() == 0.0
                    && let Ok(i) = new_val.parse::<i64>()
                {
                    target_entry["value"] = serde_json::Value::Number(i.into());
                } else if let Some(n) = serde_json::Number::from_f64(num) {
                    target_entry["value"] = serde_json::Value::Number(n);
                } else {
                    target_entry["value"] = serde_json::Value::String(new_val.to_string());
                }
            } else {
                target_entry["value"] = serde_json::Value::String(new_val.to_string());
            }
        }
        _ => {
            target_entry["type"] = serde_json::json!(1);
            target_entry["value"] = serde_json::Value::String(new_val.to_string());
        }
    }

    if let Ok(pretty) = serde_json::to_string_pretty(&sample) {
        *sample_data = pretty;
    }
}

pub fn get_widget_config_info(config_json: &str) -> Option<(String, String)> {
    let config: Value = serde_json::from_str(config_json).ok()?;
    let config_obj = if config.is_array() {
        config.as_array()?.first()?.as_object()?
    } else {
        config.as_object()?
    };
    let config_id = config_obj.get("config_id")?.as_str()?.to_string();
    let status = config_obj.get("status")?.as_str()?.to_string();
    Some((config_id, status))
}
