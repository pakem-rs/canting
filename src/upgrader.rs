use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Default)]
struct StreamSummary {
    success: bool,
    downloaded: usize,
    download_crates: Vec<String>,
    compiled: usize,
    updated: usize,
    added: usize,
    removed: usize,
    upgraded_packages: Vec<(String, String)>,
    error_lines: Vec<String>,
}

fn new_spinner(initial_msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
            .template("{spinner:.green.bold} {msg}")
            .unwrap(),
    );
    pb.set_message(initial_msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

fn print_cli_header(title: &str, subtitle: &str) {
    println!();
    println!("{}", "╭────────────────────────────────────────────────────────────────────────╮".bright_purple());
    println!("│ {:<70} │", format!("⚡ {}", title).bright_white().bold());
    println!("│ {:<70} │", subtitle.dimmed());
    println!("{}", "╰────────────────────────────────────────────────────────────────────────╯".bright_purple());
}

fn format_arrow(text: &str) -> String {
    text.replace("->", "➜")
}

/// Menjalankan perintah cargo dengan streaming stdout/stderr secara real-time dan interaktif
fn run_cargo_streaming(args: &[&str], spinner_label: &str, composer_mode: bool) -> StreamSummary {
    let mut summary = StreamSummary::default();
    let pb = new_spinner(spinner_label);
    let start_time = Instant::now();

    let mut child = match Command::new("cargo")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            pb.finish_with_message(format!("✖ Gagal menjalankan cargo: {}", e).red().to_string());
            return summary;
        }
    };

    let (tx, rx) = mpsc::channel::<String>();

    if let Some(stdout) = child.stdout.take() {
        let tx_out = tx.clone();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let _ = tx_out.send(line);
            }
        });
    }

    if let Some(stderr) = child.stderr.take() {
        let tx_err = tx.clone();
        thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let _ = tx_err.send(line);
            }
        });
    }
    drop(tx);

    for raw in rx {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        if composer_mode {
            if let Some(rest) = trimmed.strip_prefix("Updating ") {
                if rest.contains("->") {
                    summary.updated += 1;
                    let parts: Vec<&str> = rest.split("->").map(|s| s.trim()).collect();
                    if parts.len() == 2 {
                        summary.upgraded_packages.push((parts[0].to_string(), parts[1].to_string()));
                    }
                    pb.println(format!("  {} {}", "↑ Upgrading".green().bold(), format_arrow(rest).bright_cyan()));
                } else {
                    pb.println(format!("  {} Mengindeks registry: {}", "↻".cyan().bold(), rest.white()));
                    pb.set_message(format!("Mengindeks {}", rest));
                }
            } else if let Some(rest) = trimmed.strip_prefix("Adding ") {
                summary.added += 1;
                pb.println(format!("  {} {}", "+ Installing".cyan().bold(), rest.white().bold()));
            } else if let Some(rest) = trimmed.strip_prefix("Removing ") {
                summary.removed += 1;
                pb.println(format!("  {} {}", "- Removing".red().bold(), rest.dimmed()));
            } else if let Some(rest) = trimmed.strip_prefix("Downgrading ") {
                summary.updated += 1;
                pb.println(format!("  {} {}", "↓ Downgrading".yellow().bold(), format_arrow(rest).yellow()));
            } else if trimmed.starts_with("Locking ") {
                pb.set_message(format!("Menulis pembaruan {}", trimmed));
            } else if trimmed.starts_with("error") || trimmed.starts_with("Caused by:") {
                summary.error_lines.push(trimmed.to_string());
            }
        } else {
            // CLI Toolchain Upgrade Streaming Mode
            if trimmed.starts_with("Updating ") {
                pb.println(format!("  {} {}", "↻ Registry".cyan().bold(), trimmed.dimmed()));
                pb.set_message("Memperbarui crates.io index...");
            } else if let Some(crate_info) = trimmed.strip_prefix("Downloading ") {
                summary.download_crates.push(crate_info.to_string());
                pb.println(format!("  {} {}", "⬇ Downloading".bright_blue().bold(), crate_info.white()));
                pb.set_message(format!("Mengunduh: {}", crate_info));
            } else if trimmed.starts_with("Downloaded ") {
                summary.downloaded += 1;
            } else if let Some(compiled_info) = trimmed.strip_prefix("Compiling ") {
                summary.compiled += 1;
                pb.set_message(format!("Mengompilasi [{}]: {}", summary.compiled, compiled_info));
                if compiled_info.contains("pakem")
                    || compiled_info.contains("canting")
                    || compiled_info.contains("sqlx")
                    || compiled_info.contains("axum")
                    || compiled_info.contains("tokio")
                {
                    pb.println(format!("  {} {}", "⚙ Building".yellow().bold(), compiled_info.bright_yellow()));
                }
            } else if trimmed.starts_with("Installing ") || trimmed.starts_with("Replacing ") {
                pb.println(format!("  {} {}", "★ Linking Binary".bright_magenta().bold(), trimmed.white().bold()));
                pb.set_message("Menginstal binary ke sistem...");
            } else if trimmed.starts_with("Finished ") {
                pb.println(format!("  {} {}", "✔ Compiled".bright_green().bold(), trimmed.bright_green()));
            } else if trimmed.starts_with("error") || trimmed.starts_with("Caused by:") {
                summary.error_lines.push(trimmed.to_string());
            }
        }
    }

    match child.wait() {
        Ok(status) => {
            summary.success = status.success();
            let elapsed = start_time.elapsed();
            if summary.success {
                pb.finish_and_clear();
                println!("  {} Selesai dalam {:.2} detik.", "⚡".green(), elapsed.as_secs_f32());
            } else {
                pb.finish_with_message("✖ Proses cargo gagal dieksekusi.".red().to_string());
            }
        }
        Err(e) => {
            pb.finish_with_message(format!("✖ Gagal menunggu proses cargo: {}", e).red().to_string());
        }
    }

    summary
}

/// Eksekusi perintah: canting upgrade (fokus CLI toolchain + opsional sync proyek)
pub fn run_upgrade(cli_only: bool, project_only: bool, local_path: Option<String>) {
    print_cli_header(
        "CANTING TOOLCHAIN UPGRADE ENGINE",
        "Look like Laravel, Run at Rust Speed — Self-Updater & Tooling Suite",
    );

    println!("  • Current CLI Version : {}", format!("v{}", env!("CARGO_PKG_VERSION")).bright_cyan().bold());

    let in_project = is_pakem_project();

    // 1. Upgrade CLI Toolchain
    if !project_only {
        let source_label = if let Some(ref local) = local_path {
            format!("Local Path: {}", local)
        } else {
            "crates.io (Official Registry) ➔ Fallback: GitHub (pakem-rs/framework)".to_string()
        };
        println!("  • Target Source       : {}", source_label.bright_yellow());
        println!("  • Target Binary       : {}", "canting (Global CLI)".white().bold());
        println!();
        println!("{} Memulai proses peningkatan Canting CLI...", "▸".bright_purple().bold());

        #[cfg(windows)]
        prepare_windows_binary_for_replacement();

        let summary = if let Some(local) = local_path {
            let cli_folder = Path::new(&local).join("canting");
            let target = if cli_folder.exists() {
                cli_folder
            } else {
                Path::new(&local).to_path_buf()
            };
            let target_str = target.to_string_lossy().to_string();
            run_cargo_streaming(
                &["install", "--path", &target_str, "--force"],
                "Mempersiapkan kompilasi CLI dari direktori lokal...",
                false,
            )
        } else {
            let res = run_cargo_streaming(
                &["install", "canting", "--force"],
                "Menghubungi crates.io untuk memeriksa versi terbaru...",
                false,
            );
            if res.success {
                res
            } else {
                println!("\n  {} Crates.io tidak merespons atau build gagal. Mencoba rilis GitHub upstream...", "↻ [FALLBACK]".yellow().bold());
                run_cargo_streaming(
                    &["install", "--git", "https://github.com/pakem-rs/framework.git", "canting", "--force"],
                    "Mengunduh & mengompilasi canting dari GitHub pakem-rs/framework...",
                    false,
                )
            }
        };

        if summary.success {
            println!();
            println!("  {}", "╔══════════════════════════════════════════════════════════════╗".bright_green());
            println!("  {} {:<60} {}", "║".bright_green(), "✔ CANTING CLI BERHASIL DIPERBARUI KE VERSI TERBARU!".green().bold(), "║".bright_green());
            println!("  {} {:<60} {}", "║".bright_green(), format!("Total Crate Diunduh  : {}", summary.downloaded).white(), "║".bright_green());
            println!("  {} {:<60} {}", "║".bright_green(), format!("Total Unit Dikompilasi: {}", summary.compiled).white(), "║".bright_green());
            println!("  {}", "╚══════════════════════════════════════════════════════════════╝".bright_green());
        } else {
            println!("\n  {} Gagal memperbarui Canting CLI toolchain.", "✖ [UPGRADE GAGAL]".red().bold());
            for line in summary.error_lines.iter().take(8) {
                println!("    {}", line.dimmed());
            }
            println!("\n  {} Anda dapat mencoba instalasi manual:", "Saran:".cyan().bold());
            println!("    cargo install canting --force");
            println!("    cargo install --git https://github.com/pakem-rs/framework.git canting --force");
        }
    }

    // 2. Opsional sinkronisasi dependensi proyek Pakem
    if !cli_only {
        if in_project {
            println!();
            println!("{} Menyinkronkan dependensi proyek Pakem lokal...", "▸".bright_cyan().bold());
            run_update(None, false, true);
        } else if project_only {
            println!("\n{} Tidak ditemukan berkas Cargo.toml proyek Pakem di direktori aktif.", "ℹ [INFO]".blue());
        }
    }

    println!();
    println!("{}", "────────────────────────────────────────────────────────────────────────".dimmed());
    println!("✨ Proses selesai. Menenun mahakarya perangkat lunak berkecepatan Rust! 🦀");
}

/// Eksekusi perintah: canting update (layaknya `composer update` untuk dependensi proyek)
pub fn run_update(package: Option<String>, dry_run: bool, pakem_only: bool) {
    let cargo_path = Path::new("Cargo.toml");
    if !cargo_path.exists() {
        println!("{} Tidak ditemukan Cargo.toml di direktori aktif.", "✖ [ERROR]".red().bold());
        return;
    }

    let project_name = read_project_name(cargo_path).unwrap_or_else(|| "pakem-project".to_string());
    let target_desc = if let Some(ref pkg) = package {
        format!("Spesifik Package ({})", pkg)
    } else if pakem_only {
        "Pakem Core Framework Package (-p pakem)".to_string()
    } else {
        "Semua Pustaka Ekosistem (Cargo.lock)".to_string()
    };
    let mode_desc = if dry_run {
        "Dry-Run (Simulasi - Tanpa Menulis Lockfile)".yellow().bold().to_string()
    } else {
        "Live Sync (Sinkronisasi Cargo.lock)".bright_green().bold().to_string()
    };

    print_cli_header(
        "PAKEM DEPENDENCY UPDATER (canting update)",
        "Automated Lockfile Resolution, SemVer Upgrade & Package Streaming",
    );

    println!("  • Proyek Aktif : {}", project_name.bright_white().bold());
    println!("  • Target Crate : {}", target_desc.bright_yellow());
    println!("  • Mode Operasi : {}", mode_desc);
    println!();
    println!("{} Memindai indeks crates.io dan menganalisis graph dependensi...", "▸".cyan().bold());

    let mut args: Vec<String> = vec!["update".to_string()];
    if let Some(ref pkg) = package {
        args.push("-p".to_string());
        args.push(pkg.clone());
    } else if pakem_only {
        args.push("-p".to_string());
        args.push("pakem".to_string());
    }
    if dry_run {
        args.push("--dry-run".to_string());
    }

    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let summary = run_cargo_streaming(&arg_refs, "Menyinkronkan dan memecahkan konflik versi paket...", true);

    if summary.success {
        println!();
        if summary.updated == 0 && summary.added == 0 && summary.removed == 0 {
            println!("  {} Nothing to install, update or remove — seluruh dependensi sudah mutakhir.", "✔".bright_green().bold());
        } else {
            println!("  {}", "╭──────────────────────────────────────────────────────────────╮".bright_cyan());
            println!("  │ {:<60} │", "✔ RINGKASAN PEMBARUAN DEPENDENSI (CARGO.LOCK)".bright_white().bold());
            println!("  │                                                              │");
            println!("  │  • Ditingkatkan (Upgraded) : {:<31} │", summary.updated.to_string().green().bold());
            println!("  │  • Dipasang Baru (Installed): {:<31} │", summary.added.to_string().cyan().bold());
            println!("  │  • Dihapus (Removed)        : {:<31} │", summary.removed.to_string().red().bold());
            println!("  {}", "╰──────────────────────────────────────────────────────────────╯".bright_cyan());

            if !summary.upgraded_packages.is_empty() {
                println!("\n  {} Paket Utama yang Diperbarui:", "Daftar:".white().bold());
                for (from_pkg, to_ver) in summary.upgraded_packages.iter().take(12) {
                    println!("    {} {} ➜ {}", "•".cyan(), from_pkg.white(), to_ver.bright_green().bold());
                }
                if summary.upgraded_packages.len() > 12 {
                    println!("    {} ... dan {} paket lainnya.", "•".dimmed(), summary.upgraded_packages.len() - 12);
                }
            }
        }
        if dry_run {
            println!("\n  {} Mode simulasi (--dry-run) aktif: Berkas Cargo.lock tidak diubah secara fisik.", "ℹ [SIMULASI]".bright_yellow().bold());
        }
    } else {
        println!("\n  {} Gagal memperbarui dependensi proyek Pakem.", "✖ [UPDATE GAGAL]".red().bold());
        for line in summary.error_lines.iter().take(6) {
            println!("    {}", line.dimmed());
        }
    }
}

fn read_project_name(cargo_path: &Path) -> Option<String> {
    let content = fs::read_to_string(cargo_path).ok()?;
    for line in content.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("name") {
            let rest = rest.trim();
            if let Some(val) = rest.strip_prefix('=') {
                return Some(val.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

fn is_pakem_project() -> bool {
    let cargo_path = Path::new("Cargo.toml");
    if !cargo_path.exists() {
        return false;
    }
    if let Ok(content) = fs::read_to_string(cargo_path) {
        return content.contains("pakem");
    }
    false
}

/// Menyiapkan executable Windows yang sedang aktif untuk di-overwrite oleh cargo install
/// dengan me-rename ke .old (karena Windows mengizinkan rename running exe).
#[cfg(windows)]
fn prepare_windows_binary_for_replacement() {
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let old_exe = parent.join("canting.exe.old");
            let _ = fs::remove_file(&old_exe);
            let _ = fs::rename(&current_exe, &old_exe);
        }
    }
}

/// Menghapus sisa canting.exe.old jika ada saat aplikasi dijalankan
pub fn cleanup_old_binary() {
    #[cfg(windows)]
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let old_exe = parent.join("canting.exe.old");
            if old_exe.exists() {
                let _ = fs::remove_file(&old_exe);
            }
        }
    }
}
