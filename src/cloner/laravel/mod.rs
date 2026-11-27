pub mod controller_parser;
pub mod model_parser;
pub mod route_parser;
pub mod view_converter;

use colored::*;
use controller_parser::LaravelControllerParser;
use model_parser::LaravelModelParser;
use route_parser::LaravelRouteParser;
use view_converter::LaravelViewConverter;
use std::fs;
use std::path::Path;

pub struct CloneLaravelOptions {
    pub path: String,
    pub dry_run: bool,
    pub models_only: bool,
    pub routes_only: bool,
    pub views_only: bool,
    pub with_controllers: bool,
    pub output: String,
}

pub struct LaravelCloner;

impl LaravelCloner {
    pub fn run(options: CloneLaravelOptions) -> Result<(), String> {
        println!("{}", "⚡ Canting Clone Laravel — Polyglot Application Logic Cloner".bold().bright_magenta());
        println!("{}", "Mentranspilasikan logika, model, routes, dan template Blade Laravel ke Pakem Rust...\n".dimmed());

        let laravel_root = Path::new(&options.path);
        if !laravel_root.exists() {
            return Err(format!("Direktori proyek Laravel tidak ditemukan di: {}", options.path));
        }

        let output_root = Path::new(&options.output);

        let clone_all = !options.models_only && !options.routes_only && !options.views_only;

        // 1. Kloning Models
        if clone_all || options.models_only {
            Self::clone_models(laravel_root, output_root, options.dry_run)?;
        }

        // 2. Kloning Routes
        if clone_all || options.routes_only {
            Self::clone_routes(laravel_root, output_root, options.dry_run)?;
        }

        // 3. Kloning Blade Views
        if clone_all || options.views_only {
            Self::clone_views(laravel_root, output_root, options.dry_run)?;
        }

        // 4. Kloning Controllers jika diminta
        if options.with_controllers {
            Self::clone_controllers(laravel_root, output_root, options.dry_run)?;
        }

        println!("\n{}", "🎉 Kloning proyek Laravel ke Pakem Framework selesai!".bold().green());
        if options.dry_run {
            println!("{}", "ℹ️  Mode dry-run aktif: tidak ada berkas yang ditulis ke disk.".cyan());
        }

        Ok(())
    }

    /// Kloning Model-model Eloquent dari app/Models/*.php atau app/*.php
    fn clone_models(laravel_root: &Path, output_root: &Path, dry_run: bool) -> Result<(), String> {
        println!("{}", "📦 [1/4] Memindai & Mengkloning Eloquent Models...".bold().cyan());

        let mut model_dirs = vec![laravel_root.join("app").join("Models"), laravel_root.join("app")];
        let mut found_files = Vec::new();

        for dir in model_dirs.drain(..) {
            if dir.exists() && dir.is_dir() {
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("php") {
                            found_files.push(path);
                        }
                    }
                }
            }
        }

        if found_files.is_empty() {
            println!("   {} Tidak ditemukan file model PHP di app/ atau app/Models/", "⚠️".yellow());
            return Ok(());
        }

        let target_dir = output_root.join("src").join("app").join("models");
        if !dry_run {
            fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
        }

        let parser = LaravelModelParser;
        let mut model_names = Vec::new();

        for file_path in found_files {
            if let Ok(content) = fs::read_to_string(&file_path) {
                if let Some(parsed) = LaravelModelParser::parse(&content) {
                    let rust_code = parser.generate_rust_code(&parsed);
                    let snake_name = LaravelModelParser::to_snake_case(&parsed.class_name);
                    let target_file = target_dir.join(format!("{}.rs", snake_name));

                    println!(
                        "   {} Model: {} (Tabel: {}, {} relasi)",
                        "✔".green(),
                        parsed.class_name.bold().white(),
                        parsed.table_name.dimmed(),
                        parsed.relations.len().to_string().yellow()
                    );

                    if !dry_run {
                        fs::write(&target_file, rust_code).map_err(|e| e.to_string())?;
                    }
                    model_names.push(snake_name);
                }
            }
        }

        // Generate models/mod.rs
        if !model_names.is_empty() && !dry_run {
            let mod_file = target_dir.join("mod.rs");
            let mut mod_content = String::new();
            mod_content.push_str("/// Modul Model hasil kloning otomatis dari Laravel Eloquent\n");
            for name in &model_names {
                mod_content.push_str(&format!("pub mod {};\n", name));
            }
            fs::write(mod_file, mod_content).map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    /// Kloning routing dari routes/web.php dan routes/api.php
    fn clone_routes(laravel_root: &Path, output_root: &Path, dry_run: bool) -> Result<(), String> {
        println!("{}", "\n🛣️  [2/4] Memindai & Mengkloning Laravel Routes...".bold().cyan());

        let parser = LaravelRouteParser;
        let routes_dir = laravel_root.join("routes");
        let web_file = routes_dir.join("web.php");

        if web_file.exists() {
            if let Ok(content) = fs::read_to_string(&web_file) {
                let parsed_routes = LaravelRouteParser::parse(&content);
                println!(
                    "   {} routes/web.php: {} rute terdeteksi",
                    "✔".green(),
                    parsed_routes.len().to_string().yellow()
                );

                let rust_routes = parser.generate_rust_routes(&parsed_routes);
                let target_dir = output_root.join("routes");
                if !dry_run {
                    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
                    fs::write(target_dir.join("web.rs"), rust_routes).map_err(|e| e.to_string())?;
                }
            }
        } else {
            println!("   {} File routes/web.php tidak ditemukan", "⚠️".yellow());
        }

        Ok(())
    }

    /// Kloning template Blade dari resources/views/**/*.blade.php
    fn clone_views(laravel_root: &Path, output_root: &Path, dry_run: bool) -> Result<(), String> {
        println!("{}", "\n🎨 [3/4] Mengonversi Template Blade PHP ke Pakem Blade (.blade.rs)...".bold().cyan());

        let views_source = laravel_root.join("resources").join("views");
        if !views_source.exists() {
            println!("   {} Folder resources/views tidak ditemukan", "⚠️".yellow());
            return Ok(());
        }

        let target_views_dir = output_root.join("resources").join("views");
        let mut count = 0;

        Self::visit_and_convert_views(&views_source, &views_source, &target_views_dir, dry_run, &mut count)?;

        println!("   {} Berhasil mengonversi {} template Blade", "✔".green(), count.to_string().yellow());
        Ok(())
    }

    fn visit_and_convert_views(
        current_dir: &Path,
        source_root: &Path,
        target_root: &Path,
        dry_run: bool,
        count: &mut usize,
    ) -> Result<(), String> {
        if let Ok(entries) = fs::read_dir(current_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::visit_and_convert_views(&path, source_root, target_root, dry_run, count)?;
                } else if path.is_file() {
                    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
                    if filename.ends_with(".blade.php") {
                        let relative = path.strip_prefix(source_root).map_err(|e| e.to_string())?;
                        let target_filename = filename.replace(".blade.php", ".blade.rs");
                        let target_subpath = relative.with_file_name(target_filename);
                        let full_target = target_root.join(target_subpath);

                        if let Ok(content) = fs::read_to_string(&path) {
                            let converted = LaravelViewConverter::convert(&content);
                            if !dry_run {
                                if let Some(parent) = full_target.parent() {
                                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                                }
                                fs::write(&full_target, converted).map_err(|e| e.to_string())?;
                            }
                            *count += 1;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Kloning Controller dari app/Http/Controllers/*.php
    fn clone_controllers(laravel_root: &Path, output_root: &Path, dry_run: bool) -> Result<(), String> {
        println!("{}", "\n🎮 [4/4] Membangkitkan Controller Pakem Rust...".bold().cyan());

        let ctrl_source = laravel_root.join("app").join("Http").join("Controllers");
        if !ctrl_source.exists() {
            println!("   {} Folder app/Http/Controllers tidak ditemukan", "⚠️".yellow());
            return Ok(());
        }

        let target_ctrl_dir = output_root.join("src").join("app").join("controllers");
        if !dry_run {
            fs::create_dir_all(&target_ctrl_dir).map_err(|e| e.to_string())?;
        }

        let parser = LaravelControllerParser;
        let mut count = 0;

        if let Ok(entries) = fs::read_dir(ctrl_source) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("php") {
                    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
                    if filename == "Controller.php" {
                        continue;
                    }
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Some(parsed) = LaravelControllerParser::parse(&content) {
                            let rust_ctrl = parser.generate_rust_controller(&parsed);
                            let snake_name = LaravelRouteParser::to_controller_module_name(&parsed.class_name);
                            let target_file = target_ctrl_dir.join(format!("{}.rs", snake_name));

                            println!(
                                "   {} Controller: {} ({} action)",
                                "✔".green(),
                                parsed.class_name.bold().white(),
                                parsed.methods.len().to_string().yellow()
                            );

                            if !dry_run {
                                fs::write(&target_file, rust_ctrl).map_err(|e| e.to_string())?;
                            }
                            count += 1;
                        }
                    }
                }
            }
        }

        println!("   {} Berhasil membangkitkan {} Controller Pakem", "✔".green(), count.to_string().yellow());
        Ok(())
    }
}
