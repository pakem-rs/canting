use colored::*;
use pakem::database::connection::DatabasePool;
use pakem::tinker::{Tinker, TinkerResult};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::path::Path;
use std::sync::Arc;

/// Menjalankan sesi terminal interaktif Pakem Tinker REPL
pub async fn run() {
    dotenv::dotenv().ok();

    println!();
    println!("{}", "=========================================================".cyan());
    println!("{}", "   ⚡ Pakem Framework — Interactive Tinker Shell (REPL)   ".cyan().bold());
    println!("{}", "=========================================================".cyan());

    // Coba hubungkan ke basis data aktif dari .env (mendukung DATABASE_URL maupun DB_* ala Laravel)
    let pool = match crate::migrator::resolve_database_url() {
        Ok((db_url, _)) => match DatabasePool::connect(&db_url).await {
            Ok(p) => {
                let kind_str = match p.kind {
                    pakem::database::connection::DatabaseKind::Sqlite => "SQLite",
                    pakem::database::connection::DatabaseKind::Postgres => "PostgreSQL",
                    pakem::database::connection::DatabaseKind::MySql => "MySQL",
                    pakem::database::connection::DatabaseKind::Mssql => "SQL Server",
                };
                println!("{} Terhubung ke basis data: {}", "📦 [Database]".green().bold(), kind_str.yellow());
                Some(Arc::new(p))
            }
            Err(e) => {
                println!("{} Gagal menghubungkan ke database: {}", "⚠️  [Database]".yellow().bold(), e);
                None
            }
        },
        Err(_) => {
            println!("{} Konfigurasi basis data tidak ditemukan di .env (mode offline tanpa db).", "ℹ️  [Database]".blue());
            None
        }
    };

    println!("{}", "Ketik 'help' untuk daftar perintah, 'exit' atau Ctrl+D untuk keluar.\n".dimmed());

    let mut rl = match DefaultEditor::new() {
        Ok(editor) => editor,
        Err(e) => {
            eprintln!("Gagal menginisialisasi editor readline: {}", e);
            return;
        }
    };

    let history_file = ".pakem_tinker_history";
    if Path::new(history_file).exists() {
        let _ = rl.load_history(history_file);
    }

    loop {
        let prompt = format!("{} ", "pakem >".cyan().bold());
        let readline = rl.readline(&prompt);

        match readline {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(&line);

                let res = Tinker::eval(&line, pool.clone()).await;
                match res {
                    TinkerResult::Exit => {
                        println!("{}", "👋 Goodbye! Leaving Pakem Tinker.\n".green());
                        break;
                    }
                    _ => {
                        print!("{}", res.render());
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", "^C (Ketik 'exit' untuk keluar)".dimmed());
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("{}", "👋 Goodbye! Leaving Pakem Tinker.\n".green());
                break;
            }
            Err(err) => {
                eprintln!("Error membaca baris: {:?}", err);
                break;
            }
        }
    }

    let _ = rl.save_history(history_file);
}
