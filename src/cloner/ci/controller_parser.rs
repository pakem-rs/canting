use regex::Regex;

#[derive(Debug, Clone)]
pub struct ParsedCiController {
    pub class_name: String,
    pub methods: Vec<String>,
}

pub struct CiControllerParser;

impl CiControllerParser {
    /// Mem-parsing file Controller CodeIgniter (CI_Controller / BaseController)
    pub fn parse(php_content: &str) -> Option<ParsedCiController> {
        // 1. Ekstrak class name
        let class_re = Regex::new(r"class\s+([A-Za-z0-9_]+)\s+extends\s+[A-Za-z0-9_]*Controller").ok()?;
        let class_caps = class_re.captures(php_content)?;
        let class_name = class_caps.get(1)?.as_str().to_string();

        // 2. Ekstrak nama method public
        let method_re = Regex::new(r"public\s+function\s+([A-Za-z0-9_]+)\s*\(").ok()?;
        let mut methods = Vec::new();

        for cap in method_re.captures_iter(php_content) {
            if let Some(m) = cap.get(1) {
                let name = m.as_str();
                if name != "__construct" && name != "initController" {
                    methods.push(name.to_string());
                }
            }
        }

        Some(ParsedCiController {
            class_name,
            methods,
        })
    }

    /// Menghasilkan controller Rust Pakem yang mematuhi AGENTS.md
    pub fn generate_rust_controller(&self, controller: &ParsedCiController) -> String {
        let mut code = String::new();
        code.push_str("use pakem::prelude::*;\n\n");
        code.push_str(&format!("/// Controller {} hasil kloning dari CodeIgniter\n", controller.class_name));
        code.push_str("/// Menggunakan parameter tunggal req: Request sesuai standar Pakem.\n\n");

        let base_name = controller.class_name.trim_end_matches("Controller").to_lowercase();

        for method in &controller.methods {
            let fn_name = Self::to_snake_case(method);
            match method.as_str() {
                "index" => {
                    code.push_str("pub async fn index(req: Request) -> impl IntoResponse {\n");
                    code.push_str(&format!("    View::make(\"{}.index\")\n", base_name));
                    code.push_str(&format!("        .with(\"title\", \"Daftar {}\")\n", base_name));
                    code.push_str("        .render(req)\n");
                    code.push_str("        .await\n");
                    code.push_str("}\n\n");
                }
                "create" => {
                    code.push_str("pub async fn create(req: Request) -> impl IntoResponse {\n");
                    code.push_str(&format!("    View::make(\"{}.create\")\n", base_name));
                    code.push_str("        .render(req)\n");
                    code.push_str("        .await\n");
                    code.push_str("}\n\n");
                }
                "store" | "save" | "insert" => {
                    code.push_str(&format!("pub async fn {}(req: Request) -> PakemResult {{\n", fn_name));
                    code.push_str("    // Validasi data dan simpan ke database\n");
                    code.push_str("    req.created_with(serde_json::json!({ \"success\": true }), \"Data berhasil disimpan\")\n");
                    code.push_str("}\n\n");
                }
                "show" | "detail" | "view" => {
                    code.push_str(&format!("pub async fn {}(req: Request) -> impl IntoResponse {{\n", fn_name));
                    code.push_str("    let id = req.id_or_fail().unwrap_or(0);\n");
                    code.push_str(&format!("    View::make(\"{}.show\")\n", base_name));
                    code.push_str("        .with(\"id\", id)\n");
                    code.push_str("        .render(req)\n");
                    code.push_str("        .await\n");
                    code.push_str("}\n\n");
                }
                "edit" => {
                    code.push_str("pub async fn edit(req: Request) -> impl IntoResponse {\n");
                    code.push_str("    let id = req.id_or_fail().unwrap_or(0);\n");
                    code.push_str(&format!("    View::make(\"{}.edit\")\n", base_name));
                    code.push_str("        .with(\"id\", id)\n");
                    code.push_str("        .render(req)\n");
                    code.push_str("        .await\n");
                    code.push_str("}\n\n");
                }
                "update" => {
                    code.push_str("pub async fn update(req: Request) -> PakemResult {\n");
                    code.push_str("    let _id = req.id_or_fail()?;\n");
                    code.push_str("    req.success(serde_json::json!({ \"updated\": true }), \"Data berhasil diperbarui\")\n");
                    code.push_str("}\n\n");
                }
                "delete" | "destroy" | "hapus" => {
                    code.push_str(&format!("pub async fn {}(req: Request) -> PakemResult {{\n", fn_name));
                    code.push_str("    let _id = req.id_or_fail()?;\n");
                    code.push_str("    req.success(serde_json::json!({ \"deleted\": true }), \"Data berhasil dihapus\")\n");
                    code.push_str("}\n\n");
                }
                _ => {
                    code.push_str(&format!("pub async fn {}(req: Request) -> impl IntoResponse {{\n", fn_name));
                    code.push_str(&format!("    View::make(\"{}.{}\")\n", base_name, fn_name));
                    code.push_str("        .render(req)\n");
                    code.push_str("        .await\n");
                    code.push_str("}\n\n");
                }
            }
        }

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
