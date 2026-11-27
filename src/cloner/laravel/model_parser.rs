use regex::Regex;
use std::collections::HashMap;

/// Definisi sebuah relasi Eloquent hasil ekstraksi file PHP
#[derive(Debug, Clone, PartialEq)]
pub enum LaravelRelation {
    HasMany {
        related_model: String,
        foreign_key: String,
    },
    BelongsTo {
        related_model: String,
        foreign_key: String,
    },
    BelongsToMany {
        related_model: String,
        pivot_table: String,
        foreign_pivot_key: String,
        related_pivot_key: String,
    },
    HasOne {
        related_model: String,
        foreign_key: String,
    },
}

/// Struktur Model Laravel yang berhasil diparsing
#[derive(Debug, Clone)]
pub struct ParsedLaravelModel {
    pub class_name: String,
    pub table_name: String,
    pub primary_key: String,
    pub fillable: Vec<String>,
    pub casts: HashMap<String, String>,
    pub has_timestamps: bool,
    pub has_soft_deletes: bool,
    pub relations: Vec<LaravelRelation>,
}

pub struct LaravelModelParser;

impl LaravelModelParser {
    /// Mem-parsing konten file model PHP Laravel menjadi struct ParsedLaravelModel
    pub fn parse(php_content: &str) -> Option<ParsedLaravelModel> {
        // 1. Ekstrak nama Class (class Post extends Model)
        let class_re = Regex::new(r"class\s+([A-Za-z0-9_]+)\s+extends\s+[A-Za-z0-9_]*Model").ok()?;
        let class_caps = class_re.captures(php_content)?;
        let class_name = class_caps.get(1)?.as_str().to_string();

        // 2. Ekstrak $table jika ada, jika tidak otomatis plural snake_case
        let table_re = Regex::new(r#"(?:protected|public)\s+\$table\s*=\s*['"]([A-Za-z0-9_]+)['"]"#).ok()?;
        let table_name = if let Some(caps) = table_re.captures(php_content) {
            caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_else(|| Self::pluralize_snake_case(&class_name))
        } else {
            Self::pluralize_snake_case(&class_name)
        };

        // 3. Ekstrak $primaryKey (default: "id")
        let pk_re = Regex::new(r#"(?:protected|public)\s+\$primaryKey\s*=\s*['"]([A-Za-z0-9_]+)['"]"#).ok();
        let primary_key = pk_re
            .and_then(|re| re.captures(php_content))
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "id".to_string());

        // 4. Deteksi Timestamps (public $timestamps = false;)
        let timestamps_re = Regex::new(r#"(?:public|protected)\s+\$timestamps\s*=\s*false"#).ok();
        let has_timestamps = !timestamps_re.map(|re| re.is_match(php_content)).unwrap_or(false);

        // 5. Deteksi SoftDeletes (use SoftDeletes;)
        let soft_deletes_re = Regex::new(r#"use\s+.*SoftDeletes"#).ok();
        let has_soft_deletes = soft_deletes_re.map(|re| re.is_match(php_content)).unwrap_or(false);

        // 6. Ekstrak $fillable = ['title', 'content', ...];
        let fillable = Self::extract_fillable(php_content);

        // 7. Ekstrak $casts = ['is_active' => 'boolean', ...];
        let casts = Self::extract_casts(php_content);

        // 8. Ekstrak Relasi Eloquent (hasMany, belongsTo, belongsToMany, hasOne)
        let relations = Self::extract_relations(php_content, &class_name);

        Some(ParsedLaravelModel {
            class_name,
            table_name,
            primary_key,
            fillable,
            casts,
            has_timestamps,
            has_soft_deletes,
            relations,
        })
    }

    /// Ekstraksi array $fillable
    fn extract_fillable(content: &str) -> Vec<String> {
        let fillable_re = Regex::new(r#"\$fillable\s*=\s*\[([\s\S]*?)\];"#).ok();
        let mut results = Vec::new();

        if let Some(re) = fillable_re {
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
        results
    }

    /// Ekstraksi array $casts
    fn extract_casts(content: &str) -> HashMap<String, String> {
        let casts_re = Regex::new(r#"\$casts\s*=\s*\[([\s\S]*?)\];"#).ok();
        let mut map = HashMap::new();

        if let Some(re) = casts_re {
            if let Some(caps) = re.captures(content) {
                if let Some(body) = caps.get(1) {
                    let pair_re = Regex::new(r#"['"]([A-Za-z0-9_]+)['"]\s*=>\s*['"]([A-Za-z0-9_:]+)['"]"#).unwrap();
                    for pair_cap in pair_re.captures_iter(body.as_str()) {
                        if let (Some(k), Some(v)) = (pair_cap.get(1), pair_cap.get(2)) {
                            map.insert(k.as_str().to_string(), v.as_str().to_lowercase());
                        }
                    }
                }
            }
        }
        map
    }

    /// Ekstraksi relasi Eloquent dalam method PHP
    fn extract_relations(content: &str, current_class: &str) -> Vec<LaravelRelation> {
        let mut relations = Vec::new();

        // 1. hasMany(Comment::class, 'post_id')
        let has_many_re = Regex::new(
            r#"\$this->hasMany\(\s*([A-Za-z0-9_\\]+)::class(?:\s*,\s*['"]([A-Za-z0-9_]+)['"])?"#
        ).unwrap();
        for cap in has_many_re.captures_iter(content) {
            if let Some(model_match) = cap.get(1) {
                let related_model = Self::clean_class_name(model_match.as_str());
                let foreign_key = cap.get(2)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| format!("{}_id", Self::to_snake_case(current_class)));
                relations.push(LaravelRelation::HasMany { related_model, foreign_key });
            }
        }

        // 2. belongsTo(User::class, 'user_id')
        let belongs_to_re = Regex::new(
            r#"\$this->belongsTo\(\s*([A-Za-z0-9_\\]+)::class(?:\s*,\s*['"]([A-Za-z0-9_]+)['"])?"#
        ).unwrap();
        for cap in belongs_to_re.captures_iter(content) {
            if let Some(model_match) = cap.get(1) {
                let related_model = Self::clean_class_name(model_match.as_str());
                let foreign_key = cap.get(2)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| format!("{}_id", Self::to_snake_case(&related_model)));
                relations.push(LaravelRelation::BelongsTo { related_model, foreign_key });
            }
        }

        // 3. belongsToMany(Role::class, 'role_user', 'user_id', 'role_id')
        let belongs_to_many_re = Regex::new(
            r#"\$this->belongsToMany\(\s*([A-Za-z0-9_\\]+)::class(?:\s*,\s*['"]([A-Za-z0-9_]+)['"])?(?:\s*,\s*['"]([A-Za-z0-9_]+)['"])?(?:\s*,\s*['"]([A-Za-z0-9_]+)['"])?"#
        ).unwrap();
        for cap in belongs_to_many_re.captures_iter(content) {
            if let Some(model_match) = cap.get(1) {
                let related_model = Self::clean_class_name(model_match.as_str());
                let default_pivot = {
                    let mut pair = vec![Self::to_snake_case(current_class), Self::to_snake_case(&related_model)];
                    pair.sort();
                    pair.join("_")
                };
                let pivot_table = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or(default_pivot);
                let foreign_pivot_key = cap.get(3).map(|m| m.as_str().to_string()).unwrap_or_else(|| format!("{}_id", Self::to_snake_case(current_class)));
                let related_pivot_key = cap.get(4).map(|m| m.as_str().to_string()).unwrap_or_else(|| format!("{}_id", Self::to_snake_case(&related_model)));
                relations.push(LaravelRelation::BelongsToMany {
                    related_model,
                    pivot_table,
                    foreign_pivot_key,
                    related_pivot_key,
                });
            }
        }

        // 4. hasOne(Profile::class, 'user_id')
        let has_one_re = Regex::new(
            r#"\$this->hasOne\(\s*([A-Za-z0-9_\\]+)::class(?:\s*,\s*['"]([A-Za-z0-9_]+)['"])?"#
        ).unwrap();
        for cap in has_one_re.captures_iter(content) {
            if let Some(model_match) = cap.get(1) {
                let related_model = Self::clean_class_name(model_match.as_str());
                let foreign_key = cap.get(2)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| format!("{}_id", Self::to_snake_case(current_class)));
                relations.push(LaravelRelation::HasOne { related_model, foreign_key });
            }
        }

        relations
    }

    /// Menghasilkan kode Rust Model Pakem yang valid dan sesuai kaidah AGENTS.md
    pub fn generate_rust_code(&self, model: &ParsedLaravelModel) -> String {
        let mut code = String::new();
        code.push_str("use serde::{Deserialize, Serialize};\n");
        code.push_str("use sqlx::FromRow;\n");
        code.push_str("use pakem::prelude::*;\n\n");

        code.push_str("/// Model hasil kloning otomatis dari Laravel Eloquent\n");
        code.push_str("#[derive(Debug, Serialize, Deserialize, FromRow, Clone, Default, PakemModel)]\n");
        code.push_str(&format!("#[table(\"{}\")]\n", model.table_name));
        code.push_str(&format!("pub struct {} {{\n", model.class_name));

        // Primary key (selalu ada)
        code.push_str(&format!("    pub {}: i64,\n", model.primary_key));

        // Generate kolom dari $fillable
        for field in &model.fillable {
            if field == &model.primary_key {
                continue;
            }
            let rust_type = Self::map_field_to_rust_type(field, &model.casts);
            code.push_str(&format!("    pub {}: {},\n", field, rust_type));
        }

        // Timestamps
        if model.has_timestamps {
            code.push_str("    pub created_at: Option<String>,\n");
            code.push_str("    pub updated_at: Option<String>,\n");
        }

        // SoftDeletes
        if model.has_soft_deletes {
            code.push_str("    pub deleted_at: Option<String>,\n");
        }

        code.push_str("}\n\n");

        // Relasi macros
        if !model.relations.is_empty() {
            code.push_str(&format!("// ─── Relasi Eloquent untuk {} ────────────────────────────\n", model.class_name));
            for rel in &model.relations {
                match rel {
                    LaravelRelation::HasMany { related_model, foreign_key } => {
                        code.push_str(&format!("has_many!({}, \"{}\");\n", related_model, foreign_key));
                    }
                    LaravelRelation::BelongsTo { related_model, foreign_key } => {
                        code.push_str(&format!("belongs_to!({}, \"{}\");\n", related_model, foreign_key));
                    }
                    LaravelRelation::BelongsToMany { related_model, pivot_table, foreign_pivot_key, related_pivot_key } => {
                        code.push_str(&format!(
                            "belongs_to_many!({}, \"{}\", \"{}\", \"{}\");\n",
                            related_model, pivot_table, foreign_pivot_key, related_pivot_key
                        ));
                    }
                    LaravelRelation::HasOne { related_model, foreign_key } => {
                        code.push_str(&format!("has_one!({}, \"{}\");\n", related_model, foreign_key));
                    }
                }
            }
        }

        code
    }

    /// Mapping field dan casts Laravel ke Rust Type
    fn map_field_to_rust_type(field: &str, casts: &HashMap<String, String>) -> String {
        if let Some(cast) = casts.get(field) {
            match cast.as_str() {
                "boolean" | "bool" => return "bool".to_string(),
                "integer" | "int" => return "i64".to_string(),
                "float" | "double" | "real" => return "f64".to_string(),
                s if s.starts_with("decimal") => return "f64".to_string(),
                "array" | "json" | "collection" => return "Option<serde_json::Value>".to_string(),
                "datetime" | "date" | "timestamp" => return "Option<String>".to_string(),
                _ => return "Option<String>".to_string(),
            }
        }

        // Heuristik nama kolom
        if field.ends_with("_id") || field == "id" {
            "i64".to_string()
        } else if field.starts_with("is_") || field.starts_with("has_") {
            "bool".to_string()
        } else if field.ends_with("_count") || field.ends_with("_qty") || field == "quantity" {
            "i64".to_string()
        } else if field.ends_with("_price") || field.ends_with("_amount") || field == "price" {
            "f64".to_string()
        } else if field == "content" || field == "description" || field == "body" || field == "note" || field == "notes" {
            "Option<String>".to_string()
        } else {
            "String".to_string()
        }
    }

    fn clean_class_name(raw: &str) -> String {
        raw.split('\\').last().unwrap_or(raw).trim().to_string()
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

    pub fn pluralize_snake_case(class_name: &str) -> String {
        let snake = Self::to_snake_case(class_name);
        if snake.ends_with('y') && !snake.ends_with("ey") && !snake.ends_with("ay") {
            format!("{}ies", &snake[..snake.len() - 1])
        } else if snake.ends_with('s') || snake.ends_with('x') || snake.ends_with("ch") || snake.ends_with("sh") {
            format!("{}es", snake)
        } else {
            format!("{}s", snake)
        }
    }
}
