pub mod endpoint_parser;
pub mod schema_parser;

use colored::*;
use endpoint_parser::ApiEndpointParser;
use schema_parser::ApiSchemaParser;
use serde_json::Value;
use std::fs;
use std::path::Path;

pub struct CloneApiOptions {
    pub source: String, // URL (http://...) atau file lokal (/path/to/swagger.json)
    pub dry_run: bool,
    pub dto_only: bool,
    pub routes_only: bool,
    pub with_controllers: bool,
    pub output: String,
}

pub struct ApiCloner;

impl ApiCloner {
    pub fn run(options: CloneApiOptions) -> Result<(), String> {
        println!("{}", "⚡ Canting Clone API — Universal OpenAPI & Swagger Contract Cloner".bold().bright_magenta());
        println!("{}", "Mengkloning DTO, routes, dan API controller dari .NET, Golang, atau Java ke Pakem Rust...\n".dimmed());

        // 1. Ambil spesifikasi JSON (dari URL atau file lokal)
        let spec = Self::fetch_spec(&options.source)?;

        let title = spec
            .get("info")
            .and_then(|i| i.get("title"))
            .and_then(|t| t.as_str())
            .unwrap_or("OpenAPI Service");
        let version = spec
            .get("info")
            .and_then(|i| i.get("version"))
            .and_then(|v| v.as_str())
            .unwrap_or("1.0.0");

        println!(
            "  {} Terdeteksi Kontrak API: {} (Versi: {})",
            "📌".cyan(),
            title.bold().white(),
            version.yellow()
        );

        let output_root = Path::new(&options.output);
        let clone_all = !options.dto_only && !options.routes_only;

        // 2. Kloning DTO Schemas
        if clone_all || options.dto_only {
            Self::clone_dtos(&spec, output_root, options.dry_run)?;
        }

        // 3. Kloning Endpoints & Routes
        if clone_all || options.routes_only {
            Self::clone_endpoints(&spec, output_root, options.with_controllers, options.dry_run)?;
        }

        println!("\n{}", "🎉 Kloning kontrak API ke Pakem Framework selesai!".bold().green());
        if options.dry_run {
            println!("{}", "ℹ️  Mode dry-run aktif: tidak ada berkas yang ditulis ke disk.".cyan());
        }

        Ok(())
    }

    /// Mengambil konten JSON spesifikasi dari URL atau file lokal
    fn fetch_spec(source: &str) -> Result<Value, String> {
        if source.starts_with("http://") || source.starts_with("https://") {
            println!("  {} Mengunduh spesifikasi dari: {}", "🌐".cyan(), source.dimmed());
            let resp = reqwest::blocking::get(source)
                .map_err(|e| format!("Gagal menghubungi URL OpenAPI {}: {}", source, e))?;
            resp.json::<Value>()
                .map_err(|e| format!("Gagal mem-parsing JSON dari URL {}: {}", source, e))
        } else {
            let path = Path::new(source);
            if !path.exists() {
                return Err(format!("Berkas spesifikasi lokal tidak ditemukan di: {}", source));
            }
            let content = fs::read_to_string(path)
                .map_err(|e| format!("Gagal membaca berkas {}: {}", source, e))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("Berkas bukan format JSON yang valid: {}", e))
        }
    }

    /// Kloning DTO struct dari definitions atau components.schemas
    fn clone_dtos(spec: &Value, output_root: &Path, dry_run: bool) -> Result<(), String> {
        println!("{}", "\n📦 [1/2] Memindai & Mengkloning DTO Schemas...".bold().cyan());

        let schemas = ApiSchemaParser::parse_schemas(spec);
        if schemas.is_empty() {
            println!("   {} Tidak ditemukan skema DTO pada spesifikasi ini", "⚠️".yellow());
            return Ok(());
        }

        let target_dir = output_root.join("src").join("app").join("dto");
        if !dry_run {
            fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
        }

        let parser = ApiSchemaParser;
        let mut dto_modules = Vec::new();

        for (raw_name, model) in &schemas {
            let rust_code = parser.generate_rust_dto(model);
            let snake_name = ApiSchemaParser::to_snake_case(&model.name);
            let target_file = target_dir.join(format!("{}.rs", snake_name));

            println!(
                "   {} DTO: {} (dari '{}', {} field)",
                "✔".green(),
                model.name.bold().white(),
                raw_name.dimmed(),
                model.fields.len().to_string().yellow()
            );

            if !dry_run {
                fs::write(&target_file, rust_code).map_err(|e| e.to_string())?;
            }
            dto_modules.push(snake_name);
        }

        // Generate dto/mod.rs
        if !dto_modules.is_empty() && !dry_run {
            let mod_file = target_dir.join("mod.rs");
            let mut mod_content = String::new();
            mod_content.push_str("/// Modul DTO hasil kloning otomatis dari OpenAPI / Swagger\n");
            for name in &dto_modules {
                mod_content.push_str(&format!("pub mod {};\n", name));
            }
            fs::write(mod_file, mod_content).map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    /// Kloning Endpoints ke routes/api.rs dan Controller Handlers
    fn clone_endpoints(spec: &Value, output_root: &Path, with_controllers: bool, dry_run: bool) -> Result<(), String> {
        println!("{}", "\n🛣️  [2/2] Memindai & Mengkloning API Endpoints...".bold().cyan());

        let endpoints = ApiEndpointParser::parse_paths(spec);
        if endpoints.is_empty() {
            println!("   {} Tidak ditemukan endpoint paths pada spesifikasi ini", "⚠️".yellow());
            return Ok(());
        }

        println!("   {} Berhasil mendeteksi {} RESTful endpoint", "✔".green(), endpoints.len().to_string().yellow());

        // 1. Bangkitkan routes/api.rs
        let rust_routes = ApiEndpointParser::generate_rust_routes(&endpoints);
        let target_routes_dir = output_root.join("routes");
        if !dry_run {
            fs::create_dir_all(&target_routes_dir).map_err(|e| e.to_string())?;
            fs::write(target_routes_dir.join("api.rs"), rust_routes).map_err(|e| e.to_string())?;
        }

        // 2. Bangkitkan Controller jika diminta
        if with_controllers {
            let controllers = ApiEndpointParser::generate_controllers_by_tag(&endpoints);
            let target_ctrl_dir = output_root.join("src").join("app").join("controllers");
            if !dry_run {
                fs::create_dir_all(&target_ctrl_dir).map_err(|e| e.to_string())?;
            }

            for (tag, code) in controllers {
                let snake_ctrl = format!("{}_controller", ApiEndpointParser::to_snake_case(&tag));
                let target_file = target_ctrl_dir.join(format!("{}.rs", snake_ctrl));

                println!("   {} Controller: {}", "✔".green(), snake_ctrl.bold().white());
                if !dry_run {
                    fs::write(&target_file, code).map_err(|e| e.to_string())?;
                }
            }
        }

        Ok(())
    }
}
