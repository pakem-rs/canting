use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ParsedDtoField {
    pub name: String,
    pub rust_type: String,
    #[allow(dead_code)]
    pub is_optional: bool,
}

#[derive(Debug, Clone)]
pub struct ParsedDtoModel {
    pub name: String,
    pub fields: Vec<ParsedDtoField>,
}

pub struct ApiSchemaParser;

impl ApiSchemaParser {
    /// Mem-parsing `definitions` (OpenAPI v2) atau `components.schemas` (OpenAPI v3)
    pub fn parse_schemas(spec: &Value) -> HashMap<String, ParsedDtoModel> {
        let mut models = HashMap::new();

        // 1. Coba baca components.schemas (OpenAPI 3.x)
        let schemas = if let Some(components) = spec.get("components") {
            components.get("schemas").and_then(|s| s.as_object())
        } else {
            None
        };

        // 2. Fallback baca definitions (Swagger 2.0)
        let definitions = if schemas.is_none() {
            spec.get("definitions").and_then(|d| d.as_object())
        } else {
            None
        };

        let target_schemas = schemas.or(definitions);

        if let Some(schemas_obj) = target_schemas {
            for (model_name, schema_val) in schemas_obj {
                if let Some(model) = Self::parse_single_schema(model_name, schema_val) {
                    models.insert(model_name.clone(), model);
                }
            }
        }

        models
    }

    fn parse_single_schema(name: &str, schema: &Value) -> Option<ParsedDtoModel> {
        let props = schema.get("properties").and_then(|p| p.as_object())?;
        let required_fields: Vec<String> = schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let mut fields = Vec::new();

        for (prop_name, prop_val) in props {
            let rust_type_raw = Self::map_json_type_to_rust(prop_val);
            let is_required = required_fields.contains(prop_name);

            let (rust_type, is_optional) = if is_required {
                (rust_type_raw, false)
            } else {
                (format!("Option<{}>", rust_type_raw), true)
            };

            fields.push(ParsedDtoField {
                name: prop_name.clone(),
                rust_type,
                is_optional,
            });
        }

        Some(ParsedDtoModel {
            name: Self::clean_dto_name(name),
            fields,
        })
    }

    fn map_json_type_to_rust(prop_val: &Value) -> String {
        let json_type = prop_val
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("string");
        let format = prop_val.get("format").and_then(|f| f.as_str());

        match json_type {
            "integer" => {
                if format == Some("int32") {
                    "i32".to_string()
                } else {
                    "i64".to_string()
                }
            }
            "number" => {
                if format == Some("float") {
                    "f32".to_string()
                } else {
                    "f64".to_string()
                }
            }
            "boolean" => "bool".to_string(),
            "array" => {
                if let Some(items) = prop_val.get("items") {
                    let item_type = Self::map_json_type_to_rust(items);
                    format!("Vec<{}>", item_type)
                } else {
                    "Vec<serde_json::Value>".to_string()
                }
            }
            "object" => "serde_json::Value".to_string(),
            _ => {
                if let Some(ref_val) = prop_val.get("$ref").and_then(|r| r.as_str()) {
                    let clean = ref_val.split('/').last().unwrap_or("Object");
                    Self::clean_dto_name(clean)
                } else {
                    "String".to_string()
                }
            }
        }
    }

    pub fn clean_dto_name(name: &str) -> String {
        let clean = name.replace("Dto", "").replace("DTO", "").replace("Model", "");
        let mut res = String::new();
        let mut capitalize_next = true;
        for c in clean.chars() {
            if c.is_alphanumeric() {
                if capitalize_next {
                    res.push(c.to_ascii_uppercase());
                    capitalize_next = false;
                } else {
                    res.push(c);
                }
            } else {
                capitalize_next = true;
            }
        }
        if res.is_empty() {
            "DtoObject".to_string()
        } else {
            res
        }
    }

    /// Menghasilkan kode DTO Rust
    pub fn generate_rust_dto(&self, model: &ParsedDtoModel) -> String {
        let mut code = String::new();
        code.push_str("use serde::{Deserialize, Serialize};\n\n");
        code.push_str(&format!("/// DTO {} hasil kloning dari OpenAPI / Swagger\n", model.name));
        code.push_str("#[derive(Debug, Serialize, Deserialize, Clone, Default)]\n");
        code.push_str(&format!("pub struct {} {{\n", model.name));

        for f in &model.fields {
            let snake_name = Self::to_snake_case(&f.name);
            if snake_name != f.name {
                code.push_str(&format!("    #[serde(rename = \"{}\")]\n", f.name));
            }
            code.push_str(&format!("    pub {}: {},\n", snake_name, f.rust_type));
        }

        code.push_str("}\n");
        code
    }

    pub fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() {
                if i > 0 {
                    result.push('_');
                }
                result.push(ch.to_ascii_lowercase());
            } else {
                result.push(ch);
            }
        }
        result
    }
}
