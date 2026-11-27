pub mod controller_parser;
pub mod model_parser;
pub mod route_parser;
pub mod view_converter;

use colored::*;
use controller_parser::CiControllerParser;
use model_parser::CiModelParser;
use route_parser::CiRouteParser;
use view_converter::CiViewConverter;
use std::fs;
use std::path::Path;

pub struct CloneCiOptions {
    pub path: String,
    pub dry_run: bool,
    pub models_only: bool,
    pub routes_only: bool,
    pub views_only: bool,
    pub with_controllers: bool,
    pub output: String,
}

pub struct CiCloner;

impl CiCloner {
    pub fn run(options: CloneCiOptions) -> Result<(), String> {
        println!("{}", "⚡ Canting Clone CodeIgniter — CI3 & CI4 Application Logic Cloner".bold().bright_magenta());
        println!("{}", "Mentranspilasikan model, routes, controller, dan template view CodeIgniter ke Pakem Rust...\n".dimmed());

        let ci_root = Path::new(&options.path);
        if !ci_root.exists() {
            return Err(format!("Direktori proyek CodeIgniter tidak ditemukan di: {}", options.path));
        }

        let output_root = Path::new(&options.output);

        // Deteksi apakah CI3 (application/) atau CI4 (app/)
        let is_ci4 = ci_root.join("app").exists();
        let app_dir = if is_ci4 {
            println!("  {} Terdeteksi struktur CodeIgniter 4 (app/)", "📌".cyan());
            ci_root.join("app")
        } else if ci_root.join("application").exists() {
            println!("  {} Terdeteksi struktur CodeIgniter 3 (application/)", "📌".cyan());
            ci_root.join("application")
        } else {
            // Fallback: gunakan root langsung
            println!("  {} Memindai dari direktori root langsung", "📌".cyan());
            ci_root.to_path_buf()
        };

        let clone_all = !options.models_only && !options.routes_only && !options.views_only;

        // 1. Kloning Models
        if clone_all || options.models_only {
            Self::clone_models(&app_dir, output_root, options.dry_run)?;
        }

        // 2. Kloning Routes
        if clone_all || options.routes_only {
            Self::clone_routes(&app_dir, ci_root, output_root, options.dry_run)?;
        }

        // 3. Kloning Views
        if clone_all || options.views_only {
            Self::clone_views(&app_dir, output_root, options.dry_run)?;
        }

        // 4. Kloning Controllers jika diminta
        if options.with_controllers {
            Self::clone_controllers(&app_dir, output_root, options.dry_run)?;
        }

        println!("\n{}", "🎉 Kloning proyek CodeIgniter ke Pakem Framework selesai!".bold().green());
        if options.dry_run {
            println!("{}", "ℹ️  Mode dry-run aktif: tidak ada berkas yang ditulis ke disk.".cyan());
        }

        Ok(())
    }

    /// Kloning Model dari models/ atau Models/
    fn clone_models(app_dir: &Path, output_root: &Path, dry_run: bool) -> Result<(), String> {
        println!("{}", "📦 [1/4] Memindai & Mengkloning CodeIgniter Models...".bold().cyan());

        let candidates = vec![app_dir.join("models"), app_dir.join("Models")];
        let mut found_files = Vec::new();

        for dir in candidates {
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
            println!("   {} Tidak ditemukan file model PHP di folder models/", "⚠️".yellow());
            return Ok(());
        }

        let target_dir = output_root.join("src").join("app").join("models");
        if !dry_run {
            fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
        }

        let parser = CiModelParser;
        let mut model_names = Vec::new();

        for file_path in found_files {
            if let Ok(content) = fs::read_to_string(&file_path) {
                if let Some(parsed) = CiModelParser::parse(&content) {
                    let rust_code = parser.generate_rust_code(&parsed);
                    let clean_name = CiModelParser::clean_model_name(&parsed.class_name);
                    let snake_name = CiModelParser::to_snake_case(&clean_name);
                    let target_file = target_dir.join(format!("{}.rs", snake_name));

                    println!(
                        "   {} Model: {} (Tabel: {}, {} fields)",
                        "✔".green(),
                        clean_name.bold().white(),
                        parsed.table_name.dimmed(),
                        parsed.allowed_fields.len().to_string().yellow()
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
            mod_content.push_str("/// Modul Model hasil kloning otomatis dari CodeIgniter\n");
            for name in &model_names {
                mod_content.push_str(&format!("pub mod {};\n", name));
            }
            fs::write(mod_file, mod_content).map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    /// Kloning routes dari config/routes.php atau Config/Routes.php
    fn clone_routes(app_dir: &Path, ci_root: &Path, output_root: &Path, dry_run: bool) -> Result<(), String> {
        println!("{}", "\n🛣️  [2/4] Memindai & Mengkloning CodeIgniter Routes...".bold().cyan());

        let candidates = vec![
            app_dir.join("config").join("routes.php"),
            app_dir.join("Config").join("Routes.php"),
            ci_root.join("app").join("Config").join("Routes.php"),
            ci_root.join("application").join("config").join("routes.php"),
        ];

        let mut route_file = None;
        for c in candidates {
            if c.exists() && c.is_file() {
                route_file = Some(c);
                break;
            }
        }

        if let Some(rf) = route_file {
            if let Ok(content) = fs::read_to_string(&rf) {
                let parsed_routes = CiRouteParser::parse(&content);
                println!(
                    "   {} {}: {} rute terdeteksi",
                    "✔".green(),
                    rf.file_name().and_then(|s| s.to_str()).unwrap_or("routes.php"),
                    parsed_routes.len().to_string().yellow()
                );

                let parser = CiRouteParser;
                let rust_routes = parser.generate_rust_routes(&parsed_routes);
                let target_dir = output_root.join("routes");
                if !dry_run {
                    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
                    fs::write(target_dir.join("web.rs"), rust_routes).map_err(|e| e.to_string())?;
                }
            }
        } else {
            println!("   {} File routes.php CodeIgniter tidak ditemukan", "⚠️".yellow());
        }

        Ok(())
    }

    /// Kloning template View dari views/ atau Views/
    fn clone_views(app_dir: &Path, output_root: &Path, dry_run: bool) -> Result<(), String> {
        println!("{}", "\n🎨 [3/4] Mengonversi View CodeIgniter ke Pakem Blade (.blade.rs)...".bold().cyan());

        let candidates = vec![app_dir.join("views"), app_dir.join("Views")];
        let mut views_source = None;
        for c in candidates {
            if c.exists() && c.is_dir() {
                views_source = Some(c);
                break;
            }
        }

        if views_source.is_none() {
            println!("   {} Folder views/ CodeIgniter tidak ditemukan", "⚠️".yellow());
            return Ok(());
        }

        let src_dir = views_source.unwrap();
        let target_views_dir = output_root.join("resources").join("views");
        let mut count = 0;

        Self::visit_and_convert_views(&src_dir, &src_dir, &target_views_dir, dry_run, &mut count)?;

        println!("   {} Berhasil mengonversi {} template View CI", "✔".green(), count.to_string().yellow());
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
                    if filename.ends_with(".php") {
                        let relative = path.strip_prefix(source_root).map_err(|e| e.to_string())?;
                        let target_filename = filename.replace(".php", ".blade.rs");
                        let target_subpath = relative.with_file_name(target_filename);
                        let full_target = target_root.join(target_subpath);

                        if let Ok(content) = fs::read_to_string(&path) {
                            let converted = CiViewConverter::convert(&content);
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

    /// Kloning Controllers dari controllers/ atau Controllers/
    fn clone_controllers(app_dir: &Path, output_root: &Path, dry_run: bool) -> Result<(), String> {
        println!("{}", "\n🎮 [4/4] Membangkitkan Controller Pakem Rust dari CodeIgniter...".bold().cyan());

        let candidates = vec![app_dir.join("controllers"), app_dir.join("Controllers")];
        let mut ctrl_source = None;
        for c in candidates {
            if c.exists() && c.is_dir() {
                ctrl_source = Some(c);
                break;
            }
        }

        if ctrl_source.is_none() {
            println!("   {} Folder controllers/ CodeIgniter tidak ditemukan", "⚠️".yellow());
            return Ok(());
        }

        let src_dir = ctrl_source.unwrap();
        let target_ctrl_dir = output_root.join("src").join("app").join("controllers");
        if !dry_run {
            fs::create_dir_all(&target_ctrl_dir).map_err(|e| e.to_string())?;
        }

        let parser = CiControllerParser;
        let mut count = 0;

        if let Ok(entries) = fs::read_dir(src_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("php") {
                    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
                    if filename == "BaseController.php" || filename == "MY_Controller.php" {
                        continue;
                    }
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Some(parsed) = CiControllerParser::parse(&content) {
                            let rust_ctrl = parser.generate_rust_controller(&parsed);
                            let snake_name = CiRouteParser::to_controller_module_name(&parsed.class_name);
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
