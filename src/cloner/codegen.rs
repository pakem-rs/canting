use super::schema::{camel_to_snake, RelationType, TableSchema};
use std::fs;
use std::path::Path;

pub struct CodeGenerator;

impl CodeGenerator {
    /// Menghasilkan kode file Model Rust Pakem (src/app/models/<name>.rs)
    pub fn generate_model(table: &TableSchema) -> String {
        let mut code = String::new();

        code.push_str("use pakem::prelude::*;\n");
        code.push_str("use serde::{Deserialize, Serialize};\n");
        code.push_str("use sqlx::FromRow;\n\n");

        code.push_str("/// Model ");
        code.push_str(&table.model_name);
        code.push_str(" — Hasil kloning skema database tabel `");
        code.push_str(&table.name);
        code.push_str("`\n");
        code.push_str("#[derive(Debug, Serialize, Deserialize, FromRow, Clone, Default, PakemModel)]\n");
        code.push_str(&format!("#[table(\"{}\")]\n", table.name));
        code.push_str(&format!("pub struct {} {{\n", table.model_name));

        for col in &table.columns {
            code.push_str(&format!("    pub {}: {},\n", col.name, col.rust_type));
        }

        code.push_str("}\n");

        // Relasi
        if !table.relations.is_empty() {
            code.push_str("\n// ─── Relasi Model ──────────────────────────────────────────\n");
            for rel in &table.relations {
                match rel.relation_type {
                    RelationType::BelongsTo => {
                        code.push_str(&format!(
                            "belongs_to!({}, \"{}\");\n",
                            rel.target_model, rel.foreign_key
                        ));
                    }
                    RelationType::HasMany => {
                        code.push_str(&format!(
                            "has_many!({}, \"{}\");\n",
                            rel.target_model, rel.foreign_key
                        ));
                    }
                    RelationType::BelongsToMany => {
                        let pivot = rel.pivot_table.as_deref().unwrap_or("pivot");
                        let local = rel.local_key.as_deref().unwrap_or("id");
                        code.push_str(&format!(
                            "belongs_to_many!({}, \"{}\", \"{}\", \"{}\");\n",
                            rel.target_model, pivot, rel.foreign_key, local
                        ));
                    }
                }
            }
        }

        code
    }

    /// Menghasilkan berkas migrasi SQLx resmi
    pub fn generate_migration(table: &TableSchema, driver: &str) -> String {
        let mut sql = String::new();
        sql.push_str(&format!("-- Migrasi untuk tabel `{}`\n", table.name));
        sql.push_str(&format!("CREATE TABLE IF NOT EXISTS {} (\n", table.name));

        let mut col_defs = Vec::new();
        for col in &table.columns {
            let mut def = format!("    {} {}", col.name, col.db_type);
            if col.is_primary_key {
                if driver == "sqlite" {
                    def.push_str(" PRIMARY KEY AUTOINCREMENT");
                } else if driver == "mysql" || driver == "mariadb" {
                    def.push_str(" AUTO_INCREMENT PRIMARY KEY");
                } else {
                    def.push_str(" PRIMARY KEY");
                }
            } else if !col.is_nullable {
                def.push_str(" NOT NULL");
            }

            if let Some(dflt) = &col.default_value {
                def.push_str(&format!(" DEFAULT {}", dflt));
            }

            col_defs.push(def);
        }

        // Foreign keys DDL
        for fk in &table.foreign_keys {
            col_defs.push(format!(
                "    FOREIGN KEY ({}) REFERENCES {}({}) ON DELETE CASCADE",
                fk.column_name, fk.foreign_table, fk.foreign_column
            ));
        }

        sql.push_str(&col_defs.join(",\n"));
        sql.push_str("\n);\n");
        sql
    }

    /// Menghasilkan file Model Factory stubs
    pub fn generate_factory(table: &TableSchema) -> String {
        let mut code = String::new();
        let factory_name = format!("{}Factory", table.model_name);

        code.push_str("use pakem::prelude::*;\n");
        code.push_str(&format!("use crate::app::models::{}::{};\n\n", camel_to_snake(&table.model_name), table.model_name));
        code.push_str(&format!("pub struct {};\n\n", factory_name));
        code.push_str(&format!("impl {} {{\n", factory_name));
        code.push_str(&format!("    pub fn make() -> {} {{\n", table.model_name));
        code.push_str(&format!("        {} {{\n", table.model_name));

        for col in &table.columns {
            let dummy_val = match col.rust_type.as_str() {
                "i64" | "i32" | "i16" => "1".to_string(),
                "bool" => "true".to_string(),
                "f64" => "100.0".to_string(),
                "String" => format!("\"Sample {}\".to_string()", col.name),
                "Option<String>" => "None".to_string(),
                _ if col.rust_type.starts_with("Option<") => "None".to_string(),
                _ => "Default::default()".to_string(),
            };
            code.push_str(&format!("            {}: {},\n", col.name, dummy_val));
        }

        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("}\n");

        code
    }

    /// Menghasilkan Controller API CRUD stubs
    pub fn generate_controller(table: &TableSchema) -> String {
        let mut code = String::new();
        let controller_name = format!("{}Controller", table.model_name);
        let model = &table.model_name;

        code.push_str("use pakem::prelude::*;\n");
        code.push_str(&format!("use crate::app::models::{}::{};\n\n", camel_to_snake(model), model));
        code.push_str(&format!("pub struct {};\n\n", controller_name));
        code.push_str(&format!("impl {} {{\n", controller_name));

        // Index
        code.push_str("    pub async fn index(req: Request) -> PakemResult {\n");
        code.push_str("        let (page, per_page) = req.pagination();\n");
        code.push_str(&format!("        let items = {}::paginate(req.db(), page, per_page).await?;\n", model));
        code.push_str("        req.paginate(items)\n");
        code.push_str("    }\n\n");

        // Show
        code.push_str("    pub async fn show(req: Request) -> PakemResult {\n");
        code.push_str("        let id = req.id_or_fail()?;\n");
        code.push_str(&format!("        let item = {}::find(req.db(), id).await?;\n", model));
        code.push_str("        match item {\n");
        code.push_str("            Some(data) => req.ok(data),\n");
        code.push_str(&format!("            None => req.not_found(\"{} tidak ditemukan\"),\n", model));
        code.push_str("        }\n");
        code.push_str("    }\n");

        code.push_str("}\n");

        code
    }

    /// Mendaftarkan modul ke `src/app/models/mod.rs`
    pub fn update_models_mod(models_dir: &Path, model_modules: &[String]) -> Result<(), String> {
        let mod_file = models_dir.join("mod.rs");
        let mut content = if mod_file.exists() {
            fs::read_to_string(&mod_file).unwrap_or_default()
        } else {
            String::new()
        };

        for m in model_modules {
            let pub_mod = format!("pub mod {};\n", m);
            if !content.contains(&pub_mod) {
                content.push_str(&pub_mod);
            }
        }

        fs::write(&mod_file, content).map_err(|e| format!("Gagal menulis {}: {}", mod_file.display(), e))
    }
}
