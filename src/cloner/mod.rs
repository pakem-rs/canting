pub mod api;
pub mod ci;
pub mod codegen;
pub mod introspector;
pub mod laravel;
pub mod relation_analyzer;
pub mod schema;

pub use api::{ApiCloner, CloneApiOptions};
pub use ci::{CiCloner, CloneCiOptions};
pub use laravel::{CloneLaravelOptions, LaravelCloner};

use colored::*;
use codegen::CodeGenerator;
use introspector::Introspector;
use relation_analyzer::RelationAnalyzer;
use schema::camel_to_snake;
use std::fs;
use std::path::Path;

pub struct CloneDbOptions {
    pub url: Option<String>,
    pub tables: Option<String>,
    pub dry_run: bool,
    pub with_controllers: bool,
    pub with_factories: bool,
    pub models_dir: String,
    pub migrations_dir: String,
}

pub struct DbCloner;

impl DbCloner {
    pub async fn run(options: CloneDbOptions) -> Result<(), String> {
        println!("{}", "⚡ Canting Clone DB — Database Reverse Engineering".bold().bright_magenta());
        println!("{}", "Menenun model, relasi, dan migrasi Pakem dari skema database fisik...\n".dimmed());

        // 1. Resolve URL & connect
        let (db_url, driver) = if let Some(custom_url) = &options.url {
            let drv = if custom_url.starts_with("postgres") || custom_url.starts_with("postgresql") {
                "postgresql".to_string()
            } else if custom_url.starts_with("mysql") {
                "mysql".to_string()
            } else {
                "sqlite".to_string()
            };
            (custom_url.clone(), drv)
        } else {
            crate::migrator::resolve_database_url()?
        };

        println!("  {} Driver: {} | URL: {}", "🔌".cyan(), driver.bold().green(), db_url.dimmed());

        let pool = crate::migrator::connect_db(&db_url).await?;
        let introspector = Introspector::new(pool, driver.clone());

        // 2. Baca daftar tabel
        let all_tables = introspector.list_tables().await?;
        if all_tables.is_empty() {
            println!("  {} Tidak ditemukan tabel yang dapat dikloning di database ini.", "⚠️".yellow());
            return Ok(());
        }

        // Filter tabel jika diberikan parameter --tables
        let target_tables: Vec<String> = if let Some(filter) = &options.tables {
            let set: Vec<String> = filter.split(',').map(|s| s.trim().to_string()).collect();
            all_tables.into_iter().filter(|t| set.contains(t)).collect()
        } else {
            all_tables
        };

        println!("  {} Terdeteksi {} tabel untuk dianalisis: {}\n",
            "📦".blue(),
            target_tables.len().to_string().bold().yellow(),
            target_tables.join(", ").dimmed()
        );

        // 3. Inspeksi setiap tabel
        let mut schemas = Vec::new();
        for table_name in &target_tables {
            match introspector.inspect_table(table_name).await {
                Ok(s) => schemas.push(s),
                Err(e) => eprintln!("  {} Gagal inspeksi tabel {}: {}", "❌".red(), table_name, e),
            }
        }

        // 4. Analisis Relasi Pintar (Smart Relation Inference)
        RelationAnalyzer::analyze(&mut schemas);

        // 5. Code Generation
        let models_path = Path::new(&options.models_dir);
        let migrations_path = Path::new(&options.migrations_dir);

        if !options.dry_run {
            fs::create_dir_all(models_path)
                .map_err(|e| format!("Gagal membuat folder {}: {}", models_path.display(), e))?;
            fs::create_dir_all(migrations_path)
                .map_err(|e| format!("Gagal membuat folder {}: {}", migrations_path.display(), e))?;
        }

        let mut generated_models = Vec::new();
        let mut total_relations = 0;

        for schema in &schemas {
            let model_file_name = format!("{}.rs", camel_to_snake(&schema.model_name));
            let model_file_path = models_path.join(&model_file_name);
            let model_code = CodeGenerator::generate_model(schema);
            total_relations += schema.relations.len();

            if options.dry_run {
                println!("{}", format!("--- [DRY-RUN] Model: {} ({}) ---", schema.model_name, model_file_path.display()).bold().cyan());
                println!("{}\n", model_code.dimmed());
            } else {
                fs::write(&model_file_path, &model_code)
                    .map_err(|e| format!("Gagal menulis {}: {}", model_file_path.display(), e))?;
                println!("  {} Model di-generate: {} ({})",
                    "✅".green(),
                    schema.model_name.bold().white(),
                    model_file_path.display().to_string().dimmed()
                );
                generated_models.push(camel_to_snake(&schema.model_name));
            }

            // Migrasi
            if !options.dry_run {
                let timestamp = chrono::Utc::now().format("%Y_%m_%d_%H%M%S").to_string();
                let mig_file_name = format!("{}_create_{}_table.sql", timestamp, schema.name);
                let mig_file_path = migrations_path.join(mig_file_name);
                let mig_sql = CodeGenerator::generate_migration(schema, &driver);
                fs::write(&mig_file_path, mig_sql).ok();
            }

            // Optional Controllers
            if options.with_controllers && !options.dry_run {
                let ctrl_path = Path::new("src/app/controllers");
                fs::create_dir_all(ctrl_path).ok();
                let ctrl_name = format!("{}_controller.rs", camel_to_snake(&schema.model_name));
                let ctrl_code = CodeGenerator::generate_controller(schema);
                fs::write(ctrl_path.join(ctrl_name), ctrl_code).ok();
            }

            // Optional Factories
            if options.with_factories && !options.dry_run {
                let fact_path = Path::new("database/factories");
                fs::create_dir_all(fact_path).ok();
                let fact_name = format!("{}_factory.rs", camel_to_snake(&schema.model_name));
                let fact_code = CodeGenerator::generate_factory(schema);
                fs::write(fact_path.join(fact_name), fact_code).ok();
            }
        }

        // Update src/app/models/mod.rs
        if !options.dry_run && !generated_models.is_empty() {
            CodeGenerator::update_models_mod(models_path, &generated_models)?;
        }

        println!("\n{}", "🎉 Sukses Mengkloning Skema Database!".bold().bright_green());
        println!("  • Model Dihasilkan : {}", schemas.len().to_string().bold().cyan());
        println!("  • Relasi Terdeteksi: {}", total_relations.to_string().bold().cyan());
        println!("  • Mode Operasi     : {}\n", if options.dry_run { "Simulasi (Dry-Run)".yellow() } else { "Tulis ke Disk".green() });

        Ok(())
    }
}
