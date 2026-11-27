use regex::Regex;

#[derive(Debug, Clone)]
pub struct ParsedCiModel {
    pub class_name: String,
    pub table_name: String,
    pub primary_key: String,
    pub allowed_fields: Vec<String>,
    pub has_timestamps: bool,
}

pub struct CiModelParser;

impl CiModelParser {
    /// Mem-parsing konten file model CodeIgniter 3 (CI_Model) atau CodeIgniter 4 (CodeIgniter\Model)
    pub fn parse(php_content: &str) -> Option<ParsedCiModel> {
        // 1. Ekstrak nama Class (class User_model extends CI_Model ATAU class UserModel extends Model)
        let class_re = Regex::new(r"class\s+([A-Za-z0-9_]+)\s+extends\s+[A-Za-z0-9_\\]*Model").ok()?;
        let class_caps = class_re.captures(php_content)?;
        let class_name = class_caps.get(1)?.as_str().to_string();

        // 2. Ekstrak protected $table = 'nama_tabel';
        let table_re = Regex::new(r#"(?:protected|public|var)\s+\$table\s*=\s*['"]([A-Za-z0-9_]+)['"]"#).ok();
        let table_name = if let Some(re) = table_re {
            if let Some(caps) = re.captures(php_content) {
                caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_else(|| Self::infer_table_from_class(&class_name))
            } else {
                Self::infer_table_from_class(&class_name)
            }
        } else {
            Self::infer_table_from_class(&class_name)
        };

        // 3. Ekstrak primaryKey
        let pk_re = Regex::new(r#"(?:protected|public)\s+\$primaryKey\s*=\s*['"]([A-Za-z0-9_]+)['"]"#).ok();
        let primary_key = pk_re
            .and_then(|re| re.captures(php_content))
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "id".to_string());

        // 4. Ekstrak allowedFields (CI4) atau field dari method insert/update (CI3)
        let allowed_fields = Self::extract_allowed_fields(php_content);

        // 5. Timestamps
        let timestamps_re = Regex::new(r#"(?:protected|public)\s+\$useTimestamps\s*=\s*true"#).ok();
        let has_timestamps = timestamps_re.map(|re| re.is_match(php_content)).unwrap_or(true);

        Some(ParsedCiModel {
            class_name,
            table_name,
            primary_key,
            allowed_fields,
            has_timestamps,
        })
    }

    /// Ekstraksi allowedFields dari CI4 ($allowedFields = ['name', 'email'])
    fn extract_allowed_fields(content: &str) -> Vec<String> {
        let fields_re = Regex::new(r#"\$allowedFields\s*=\s*\[([\s\S]*?)\];"#).ok();
        let mut results = Vec::new();

        if let Some(re) = fields_re {
            if let Some(caps) = re.captures(content) {
                if let Some(body) = caps.get(1) {
                    let item_re = Regex::new(r#"['"]([A-Za-z0-9_]+)['"]"#).unwrap();
                    for item_cap in item_re.captures_iter(body.as_str()) {
                        if let Some(name) = item_cap.get(1) {
                            results.push(name.as_str().to_string());
                        }
                    }
                }
            }
        }

        // Jika CI3 (tidak ada $allowedFields eksplisit), coba cari kolom dari $this->db->insert('table', $data)
        if results.is_empty() {
            let data_keys_re = Regex::new(r#"['"]([A-Za-z0-9_]+)['"]\s*=>"#).unwrap();
            for cap in data_keys_re.captures_iter(content) {
                if let Some(k) = cap.get(1) {
                    let field = k.as_str().to_string();
                    if !results.contains(&field) && field != "id" {
                        results.push(field);
                    }
                }
            }
        }

        results
    }

    /// Menghasilkan nama struct Rust yang bersih dari nama class CI
    /// Contoh: "User_model" -> "User", "UserModel" -> "User", "M_Barang" -> "Barang"
    pub fn clean_model_name(class_name: &str) -> String {
        let mut name = class_name.to_string();
        if name.starts_with("M_") || name.starts_with("m_") {
            name = name[2..].to_string();
        }
        if name.ends_with("_model") || name.ends_with("_Model") {
            name = name[..name.len() - 6].to_string();
        } else if name.ends_with("Model") {
            name = name[..name.len() - 5].to_string();
        }
        name
    }

    /// Menghasilkan kode Rust Model Pakem yang valid
    pub fn generate_rust_code(&self, model: &ParsedCiModel) -> String {
        let clean_name = Self::clean_model_name(&model.class_name);
        let mut code = String::new();
        code.push_str("use serde::{Deserialize, Serialize};\n");
        code.push_str("use sqlx::FromRow;\n");
        code.push_str("use pakem::prelude::*;\n\n");

        code.push_str(&format!("/// Model {} hasil kloning otomatis dari CodeIgniter ({})\n", clean_name, model.class_name));
        code.push_str("#[derive(Debug, Serialize, Deserialize, FromRow, Clone, Default, PakemModel)]\n");
        code.push_str(&format!("#[table(\"{}\")]\n", model.table_name));
        code.push_str(&format!("pub struct {} {{\n", clean_name));

        // Primary key
        code.push_str(&format!("    pub {}: i64,\n", model.primary_key));

        // Generate kolom dari allowed_fields
        for field in &model.allowed_fields {
            if field == &model.primary_key {
                continue;
            }
            let rust_type = Self::infer_rust_type(field);
            code.push_str(&format!("    pub {}: {},\n", field, rust_type));
        }

        // Timestamps
        if model.has_timestamps {
            code.push_str("    pub created_at: Option<String>,\n");
            code.push_str("    pub updated_at: Option<String>,\n");
        }

        code.push_str("}\n");
        code
    }

    fn infer_table_from_class(class_name: &str) -> String {
        let clean = Self::clean_model_name(class_name);
        let snake = Self::to_snake_case(&clean);
        if snake.ends_with('y') && !snake.ends_with("ey") && !snake.ends_with("ay") {
            format!("{}ies", &snake[..snake.len() - 1])
        } else if snake.ends_with('s') || snake.ends_with('x') || snake.ends_with("ch") || snake.ends_with("sh") {
            format!("{}es", snake)
        } else {
            format!("{}s", snake)
        }
    }

    fn infer_rust_type(field: &str) -> String {
        if field.ends_with("_id") || field == "id" {
            "i64".to_string()
        } else if field.starts_with("is_") || field.starts_with("has_") || field.starts_with("status_") {
            "bool".to_string()
        } else if field.ends_with("_count") || field.ends_with("_qty") || field == "quantity" || field == "stock" {
            "i64".to_string()
        } else if field.ends_with("_price") || field.ends_with("_amount") || field == "price" || field == "total" {
            "f64".to_string()
        } else if field == "content" || field == "description" || field == "body" || field == "note" || field == "notes" || field == "address" {
            "Option<String>".to_string()
        } else {
            "String".to_string()
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
            } else {
                result.push(ch);
            }
        }
        result
    }
}
