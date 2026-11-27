use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedApiEndpoint {
    pub method: String, // "GET", "POST", "PUT", "DELETE", "PATCH"
    pub path: String,   // "/api/v1/products/{id}"
    pub operation_id: String, // "get_product_by_id"
    pub tag: String,    // "Products"
    pub summary: Option<String>,
}

pub struct ApiEndpointParser;

impl ApiEndpointParser {
    /// Mem-parsing blok `paths` dari OpenAPI / Swagger JSON
    pub fn parse_paths(spec: &Value) -> Vec<ParsedApiEndpoint> {
        let mut endpoints = Vec::new();
        let paths = if let Some(p) = spec.get("paths").and_then(|p| p.as_object()) {
            p
        } else {
            return endpoints;
        };

        for (path_str, methods_val) in paths {
            if let Some(methods_obj) = methods_val.as_object() {
                for (method_str, details_val) in methods_obj {
                    let method = method_str.to_uppercase();
                    if !["GET", "POST", "PUT", "DELETE", "PATCH"].contains(&method.as_str()) {
                        continue;
                    }

                    let tag = details_val
                        .get("tags")
                        .and_then(|t| t.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| Self::infer_tag_from_path(path_str));

                    let summary = details_val
                        .get("summary")
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string());

                    let operation_id = details_val
                        .get("operationId")
                        .and_then(|o| o.as_str())
                        .map(|s| Self::to_snake_case(s))
                        .unwrap_or_else(|| {
                            format!("{}_{}", method.to_lowercase(), Self::infer_action_from_path(path_str))
                        });

                    endpoints.push(ParsedApiEndpoint {
                        method,
                        path: path_str.clone(),
                        operation_id,
                        tag,
                        summary,
                    });
                }
            }
        }

        endpoints
    }

    /// Menghasilkan file routing routes/api.rs
    pub fn generate_rust_routes(endpoints: &[ParsedApiEndpoint]) -> String {
        let mut code = String::new();
        code.push_str("use pakem::prelude::*;\n");
        code.push_str("use crate::app::controllers::*;\n\n");
        code.push_str("/// Definisi rute RESTful API hasil kloning dari OpenAPI / Swagger\n");
        code.push_str("pub fn register_api_routes(mut app: Router) -> Router {\n");

        if endpoints.is_empty() {
            code.push_str("    // Tidak ada endpoint terdeteksi\n");
            code.push_str("    app\n");
            code.push_str("}\n");
            return code;
        }

        for ep in endpoints {
            let ctrl_mod = format!("{}_controller", Self::to_snake_case(&ep.tag));
            let method_fn = ep.method.to_lowercase();
            let action = &ep.operation_id;

            code.push_str(&format!(
                "    app = app.route(\"{}\", {}({}::{}));\n",
                ep.path, method_fn, ctrl_mod, action
            ));
        }

        code.push_str("\n    app\n");
        code.push_str("}\n");
        code
    }

    /// Menghasilkan controller-controller Rust berdasarkan pengelompokan Tag
    pub fn generate_controllers_by_tag(endpoints: &[ParsedApiEndpoint]) -> HashMap<String, String> {
        let mut groups: HashMap<String, Vec<&ParsedApiEndpoint>> = HashMap::new();
        for ep in endpoints {
            groups.entry(ep.tag.clone()).or_default().push(ep);
        }

        let mut controllers = HashMap::new();
        for (tag, eps) in groups {
            let mut code = String::new();
            code.push_str("use pakem::prelude::*;\n\n");
            code.push_str(&format!("/// API Controller untuk modul {} (OpenAPI / Swagger)\n", tag));
            code.push_str("/// Menghasilkan respons JSON standar: PakemResult.\n\n");

            for ep in eps {
                if let Some(sum) = &ep.summary {
                    code.push_str(&format!("/// {}\n", sum));
                }
                code.push_str(&format!("pub async fn {}(req: Request) -> PakemResult {{\n", ep.operation_id));
                
                if ep.path.contains("{id}") {
                    code.push_str("    let _id = req.id_or_fail()?;\n");
                }

                if ep.method == "POST" {
                    code.push_str("    req.created_with(serde_json::json!({ \"success\": true }), \"Data berhasil dibuat\")\n");
                } else if ep.method == "DELETE" {
                    code.push_str("    req.success(serde_json::json!({ \"deleted\": true }), \"Data berhasil dihapus\")\n");
                } else {
                    code.push_str("    req.ok(serde_json::json!({ \"items\": [] }))\n");
                }
                code.push_str("}\n\n");
            }

            controllers.insert(tag, code);
        }

        controllers
    }

    fn infer_tag_from_path(path: &str) -> String {
        let parts: Vec<&str> = path
            .trim_matches('/')
            .split('/')
            .filter(|p| !p.starts_with('{') && *p != "api" && *p != "v1" && *p != "v2")
            .collect();

        parts.first().unwrap_or(&"Api").to_string()
    }

    fn infer_action_from_path(path: &str) -> String {
        let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
        let last = parts.last().unwrap_or(&"item");
        if last.starts_with('{') {
            "by_id".to_string()
        } else {
            Self::to_snake_case(last)
        }
    }

    pub fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() {
                if i > 0 {
                    result.push('_');
                }
                result.push(ch.to_ascii_lowercase());
            } else if ch == '-' || ch == ' ' {
                result.push('_');
            } else {
                result.push(ch);
            }
        }
        result
    }
}
