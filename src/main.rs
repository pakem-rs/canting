use clap::{Parser, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::Path;

mod ai_context;
mod audit;
mod cloner;
mod desktop;
mod generator;
mod key_generator;
mod migrator;
mod skeleton;
mod tinker;
mod upgrader;

#[derive(Parser)]
#[command(
    name = "canting",
    author,
    version,
    about = "⚡ Canting CLI — The Artisan of Pakem Framework. Build fast, build beautiful.",
    long_about = None
)]
struct CantingCli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Pakem project
    New {
        /// The name of the new project
        name: String,
        /// Database engine to configure: sqlite, postgres, mysql, all [default: sqlite]
        #[arg(short, long, default_value = "sqlite")]
        db: String,
        /// Force offline generation using embedded templates without fetching from GitHub
        #[arg(long)]
        offline: bool,
    },

    /// Start the application server (cargo run)
    Serve,

    /// Start the server with auto-reload on code change (cargo-watch)
    Watch,

    /// Start interactive REPL shell (Pakem Tinker)
    Tinker,

    /// Clean unused build artifacts, compiler cache, and temporary files
    Clean {
        /// Force full clean (cargo clean) instead of smart sweep
        #[arg(long)]
        purge: bool,
    },

    /// Upgrade Canting CLI toolchain (and optionally sync project dependencies)
    #[command(name = "upgrade")]
    Upgrade {
        /// Upgrade only the CLI toolchain without updating the current project
        #[arg(long)]
        cli_only: bool,
        /// Update only current project dependencies without upgrading the CLI
        #[arg(long)]
        project_only: bool,
        /// Upgrade CLI from a local directory path instead of crates.io / GitHub
        #[arg(long)]
        local: Option<String>,
    },

    /// Update current project dependencies (like `composer update`)
    #[command(name = "update")]
    Update {
        /// Specific package name to update (e.g., pakem, sqlx). Omit to update all
        package: Option<String>,
        /// Preview dependency updates without modifying Cargo.lock
        #[arg(long)]
        dry_run: bool,
        /// Update only the Pakem framework package (-p pakem)
        #[arg(long)]
        pakem_only: bool,
    },

    // ─── Generators ──────────────────────────────────────────────────────────
    /// Generate a new Controller
    #[command(name = "make:controller")]
    MakeController {
        /// The name of the controller (e.g., PostController)
        name: String,
    },

    /// Generate a new Model and its migration file
    #[command(name = "make:model")]
    MakeModel {
        /// The name of the model (e.g., User, Product)
        name: String,
    },

    /// Generate a new empty migration file
    #[command(name = "make:migration")]
    MakeMigration {
        /// Description of the migration (e.g., create_posts_table)
        name: String,
    },

    /// Generate a full CRUD setup (Model, Controller, Migration, Views)
    #[command(name = "make:crud")]
    MakeCrud {
        /// The name of the CRUD resource (e.g., Post, Article)
        name: String,
        /// Optional field definitions e.g. "title:string body:text"
        #[arg(trailing_var_arg = true)]
        fields: Vec<String>,
    },

    /// Generate a ready-to-use Authentication system (Register & Login)
    #[command(name = "make:auth")]
    MakeAuth,

    /// Install Pakem Aurora starter kit (Auth, Dashboard, Profile, Blade views & Tailwind)
    #[command(name = "aurora:install")]
    AuroraInstall,

    /// Generate a new Model Factory for testing and seeding
    #[command(name = "make:factory")]
    MakeFactory {
        /// The name of the factory (e.g., UserFactory)
        name: String,
    },

    /// Generate a new Database Seeder class
    #[command(name = "make:seeder")]
    MakeSeeder {
        /// The name of the seeder (e.g., UserSeeder, ProductSeeder)
        name: String,
    },

    /// Generate a new Blade view template (e.g., products.index)
    #[command(name = "make:view")]
    MakeView {
        /// The name of the view (e.g., products.index or auth.profile)
        name: String,
    },

    /// Generate a new Background Job class (e.g., SendWelcomeEmail)
    #[command(name = "make:job")]
    MakeJob {
        /// The name of the job (e.g., SendWelcomeEmail or ProcessInvoice)
        name: String,
    },

    /// Generate a new Pakem Live reactive component
    #[command(name = "make:live")]
    MakeLive {
        /// The name of the live component (e.g., Counter, SearchUser)
        name: String,
    },

    // ─── AI Agents & Skills ──────────────────────────────────────────────────
    /// Generate or refresh AI Agent context files (AGENTS.md, .cursorrules, .github, CLAUDE.md)
    #[command(name = "make:ai-context", alias = "ai:init")]
    MakeAiContext {
        /// Target specific agent format: all, cursor, copilot, claude, agents [default: all]
        #[arg(short, long, default_value = "all")]
        target: String,
        /// Overwrite existing context files if they already exist
        #[arg(short, long)]
        force: bool,
    },

    /// Generate an AI Skill & Tool Calling stub (Gemini, Claude, OpenAI, Antigravity)
    #[command(name = "make:ai-skill", alias = "ai:skill")]
    MakeAiSkill {
        /// Name of the skill (e.g. SearchProducts, ReverseEngineer)
        name: String,
        /// Optional fields/parameters e.g. "keyword:string category_id:i64"
        #[arg(trailing_var_arg = true)]
        fields: Vec<String>,
        /// Model provider schema to generate: all, gemini, claude, openai, antigravity [default: all]
        #[arg(short, long, default_value = "all")]
        provider: String,
    },

    // ─── Queue & Background Workers ──────────────────────────────────────────
    /// Start processing background jobs from the queue
    #[command(name = "queue:work")]
    QueueWork {
        /// The queue name to listen to (default: default)
        #[arg(short, long, default_value = "default")]
        queue: String,
    },

    // ─── Security & Auditing ──────────────────────────────────────────────────
    /// Run Pakem Shield to audit security, secrets, and environment configuration
    Audit,

    /// Set the application encryption key (APP_KEY)
    #[command(name = "key:generate", alias = "key-generate")]
    KeyGenerate {
        /// Display the key instead of modifying files
        #[arg(long)]
        show: bool,

        /// Force the operation to run when in production
        #[arg(long)]
        force: bool,

        /// Custom path to environment file (default: .env)
        #[arg(long, default_value = ".env")]
        env: String,

        /// Cipher format: base64 (default) or hex
        #[arg(long, default_value = "base64")]
        cipher: String,

        /// Also generate and set a fresh JWT_SECRET
        #[arg(long)]
        jwt: bool,
    },

    // ─── Database ─────────────────────────────────────────────────────────────
    /// Run all pending database migrations
    Migrate,

    /// Rollback the last database migration
    #[command(name = "migrate:rollback")]
    MigrateRollback,

    /// Show the status of each migration (Ran or Pending)
    #[command(name = "migrate:status")]
    MigrateStatus,

    /// Seed the database with initial dummy data or via a specific Seeder class
    #[command(name = "db:seed")]
    DbSeed {
        /// Specific seeder class to run (e.g., UserSeeder)
        #[arg(long, short)]
        class: Option<String>,
    },

    // ─── Reverse Engineering & Logic Cloning ─────────────────────────────────
    /// Reverse engineer an existing database into Pakem Models, Relations, and Migrations
    #[command(name = "clone:db", alias = "db:reverse")]
    CloneDb {
        /// Database URL (e.g., sqlite://./database.sqlite, postgres://..., mysql://...)
        /// If omitted, reads DB_CONNECTION / DATABASE_URL from .env
        #[arg(short, long)]
        url: Option<String>,

        /// Comma-separated list of specific tables to clone (e.g. users,orders,products)
        #[arg(short, long)]
        tables: Option<String>,

        /// Perform a dry-run without writing any files to disk
        #[arg(long)]
        dry_run: bool,

        /// Also generate RESTful API CRUD Controllers for cloned models
        #[arg(long)]
        with_controllers: bool,

        /// Also generate Model Factories for testing and seeding
        #[arg(long)]
        with_factories: bool,

        /// Output directory for models (default: src/app/models)
        #[arg(long, default_value = "src/app/models")]
        models_dir: String,

        /// Output directory for migrations (default: database/migrations)
        #[arg(long, default_value = "database/migrations")]
        migrations_dir: String,
    },

    /// Clone and transpile an existing Laravel project into Pakem Framework (Models, Routes, Views)
    #[command(name = "clone:laravel", alias = "laravel:clone")]
    CloneLaravel {
        /// Path to the Laravel project root directory (containing artisan, app/, routes/, etc.)
        path: String,

        /// Perform a dry-run without writing files to disk
        #[arg(long)]
        dry_run: bool,

        /// Only transpile Eloquent Models
        #[arg(long)]
        models_only: bool,

        /// Only transpile Routes
        #[arg(long)]
        routes_only: bool,

        /// Only convert Blade Views
        #[arg(long)]
        views_only: bool,

        /// Also generate Controllers based on detected Laravel controllers
        #[arg(long)]
        with_controllers: bool,

        /// Output directory (default: current directory ".")
        #[arg(short, long, default_value = ".")]
        output: String,
    },

    /// Clone and transpile an existing CodeIgniter 3 or 4 project into Pakem Framework
    #[command(name = "clone:ci", alias = "ci:clone")]
    CloneCi {
        /// Path to the CodeIgniter project root directory (containing application/ or app/)
        path: String,

        /// Perform a dry-run without writing files to disk
        #[arg(long)]
        dry_run: bool,

        /// Only transpile Models
        #[arg(long)]
        models_only: bool,

        /// Only transpile Routes
        #[arg(long)]
        routes_only: bool,

        /// Only convert Views
        #[arg(long)]
        views_only: bool,

        /// Also generate Controllers based on detected CodeIgniter controllers
        #[arg(long)]
        with_controllers: bool,

        /// Output directory (default: current directory ".")
        #[arg(short, long, default_value = ".")]
        output: String,
    },

    /// Clone DTO, routes, and controllers from an OpenAPI / Swagger JSON spec (URL or file)
    #[command(name = "clone:api", alias = "api:clone")]
    CloneApi {
        /// URL or local file path to the OpenAPI / Swagger JSON specification
        source: String,

        /// Perform a dry-run without writing files to disk
        #[arg(long)]
        dry_run: bool,

        /// Only transpile DTO schemas
        #[arg(long)]
        dto_only: bool,

        /// Only transpile Routes
        #[arg(long)]
        routes_only: bool,

        /// Also generate Controllers based on detected endpoints
        #[arg(long)]
        with_controllers: bool,

        /// Output directory (default: current directory ".")
        #[arg(short, long, default_value = ".")]
        output: String,
    },

    // ─── DevOps / Deployment ──────────────────────────────────────────────────
    /// Generate production Dockerfile, docker-compose.yml & aaPanel/Nginx reverse proxy
    #[command(name = "make:docker")]
    MakeDocker {
        /// Project / domain / service name (e.g., pos.sugandi.my.id or service_pos_event)
        name: Option<String>,
        /// Port to expose on the host (default: 8000)
        #[arg(short, long)]
        port: Option<u16>,
    },

    /// Generate production Nginx reverse proxy configuration
    #[command(name = "make:nginx")]
    MakeNginx,

    /// Generate Supervisor daemon configuration
    #[command(name = "make:supervisor")]
    MakeSupervisor,

    // ─── WebAssembly Plugins ──────────────────────────────────────────────────
    /// Generate a new WASM plugin starter crate in plugins/<name>
    #[command(name = "plugin:new")]
    PluginNew {
        /// Name of the plugin (e.g. rupiah, markdown, sensor)
        name: String,
    },

    /// Compile WASM plugin crate(s) to target wasm32 and copy to plugins/<name>.wasm
    #[command(name = "plugin:build")]
    PluginBuild {
        /// Name of the plugin (optional, builds all if omitted)
        name: Option<String>,
    },

    /// List all installed WASM plugins and Indonesian Script files
    #[command(name = "plugin:list")]
    PluginList,

    /// Show WASM sandbox performance metrics and execution stats
    #[command(name = "plugin:metrics")]
    PluginMetrics,

    /// Trigger manual zero-downtime hot-reload of all WASM plugins
    #[command(name = "plugin:reload")]
    PluginReload,

    // ─── API Documentation & Tooling ─────────────────────────────────────────
    /// Generate OpenAPI v3 specification (JSON or YAML) from application routes
    #[command(name = "route:openapi")]
    RouteOpenApi {
        /// Output file path (default: openapi.json)
        #[arg(short, long, default_value = "openapi.json")]
        output: String,
        /// Format: json or yaml (default: json)
        #[arg(short, long, default_value = "json")]
        format: String,
        /// Project API title
        #[arg(long, default_value = "Pakem Application API")]
        title: String,
        /// API version
        #[arg(long, default_value = "1.0.0")]
        version: String,
    },

    /// Export Postman collection v2.1 from application routes
    #[command(name = "export:postman")]
    ExportPostman {
        /// Output file path (default: pakem_collection.json)
        #[arg(short, long, default_value = "pakem_collection.json")]
        output: String,
        /// Name of the Postman collection
        #[arg(long, default_value = "Pakem API Collection")]
        name: String,
        /// API version
        #[arg(long, default_value = "1.0.0")]
        version: String,
    },

    // ─── Desktop Compilation Engine ──────────────────────────────────────────
    /// Initialize desktop mode configurations (desktop.toml & src/bin/desktop.rs)
    #[command(name = "desktop:init")]
    DesktopInit {
        /// Window title [default: project name]
        #[arg(short, long)]
        title: Option<String>,
        /// Window width [default: 1200]
        #[arg(long, default_value_t = 1200)]
        width: u32,
        /// Window height [default: 800]
        #[arg(long, default_value_t = 800)]
        height: u32,
    },

    /// Run the application as a native desktop GUI window
    #[command(name = "desktop:run")]
    DesktopRun {
        /// Port for local embedded server (default: 0 for dynamic loopback port)
        #[arg(short, long, default_value_t = 0)]
        port: u16,
    },

    /// Compile application into a single standalone native desktop executable
    #[command(name = "desktop:build")]
    DesktopBuild {
        /// Build with release optimizations
        #[arg(long, default_value_t = true)]
        release: bool,
    },
}

fn main() {
    upgrader::cleanup_old_binary();
    let args = CantingCli::parse();

    match args.command {
        Commands::New { name, db, offline } => {
            handle_new_project(&name, &db, offline);
        }
        Commands::Serve => {
            generator::cmd_serve();
        }
        Commands::Watch => {
            generator::cmd_watch();
        }
        Commands::Tinker => {
            tokio::runtime::Runtime::new()
                .expect("Failed to start Tokio runtime for Tinker")
                .block_on(tinker::run());
        }
        Commands::Clean { purge } => {
            generator::cmd_clean(purge);
        }
        Commands::MakeController { name } => {
            generator::generate_controller(&name);
        }
        Commands::MakeModel { name } => {
            generator::generate_model(&name);
        }
        Commands::MakeMigration { name } => {
            generator::generate_migration(&name);
        }
        Commands::MakeCrud { name, fields } => {
            generator::generate_crud(&name, &fields);
        }
        Commands::MakeAuth => {
            generator::generate_auth();
        }
        Commands::AuroraInstall => {
            generator::generate_aurora();
        }
        Commands::MakeFactory { name } => {
            generator::generate_factory(&name);
        }
        Commands::MakeSeeder { name } => {
            generator::generate_seeder(&name);
        }
        Commands::MakeView { name } => {
            generator::generate_view(&name);
        }
        Commands::MakeJob { name } => {
            generator::generate_job(&name);
        }
        Commands::MakeLive { name } => {
            generator::generate_live(&name);
        }
        Commands::MakeAiContext { target, force } => {
            generator::generate_ai_context(&target, force);
        }
        Commands::MakeAiSkill {
            name,
            fields,
            provider,
        } => {
            generator::generate_ai_skill(&name, &fields, &provider);
        }
        Commands::QueueWork { queue } => {
            tokio::runtime::Runtime::new()
                .expect("Failed to start Tokio runtime for queue worker")
                .block_on(migrator::run_queue_worker_async(&queue));
        }
        Commands::Audit => {
            audit::run_audit();
        }
        Commands::KeyGenerate {
            show,
            force,
            env,
            cipher,
            jwt,
        } => {
            if let Err(e) = key_generator::run_key_generate(show, force, &env, &cipher, jwt) {
                eprintln!("{} {}", "✖ [ERROR]".red().bold(), e);
                std::process::exit(1);
            }
        }
        Commands::Migrate => {
            tokio::runtime::Runtime::new()
                .expect("Failed to start Tokio runtime for migration")
                .block_on(migrator::run_migrate_async());
        }
        Commands::MigrateRollback => {
            tokio::runtime::Runtime::new()
                .expect("Failed to start Tokio runtime for migration")
                .block_on(migrator::run_rollback_async());
        }
        Commands::MigrateStatus => {
            tokio::runtime::Runtime::new()
                .expect("Failed to start Tokio runtime for migration")
                .block_on(migrator::run_status_async());
        }
        Commands::DbSeed { class } => {
            tokio::runtime::Runtime::new()
                .expect("Failed to start Tokio runtime for seeding")
                .block_on(migrator::run_seed_async(class.as_deref()));
        }
        Commands::CloneDb {
            url,
            tables,
            dry_run,
            with_controllers,
            with_factories,
            models_dir,
            migrations_dir,
        } => {
            let options = cloner::CloneDbOptions {
                url,
                tables,
                dry_run,
                with_controllers,
                with_factories,
                models_dir,
                migrations_dir,
            };
            tokio::runtime::Runtime::new()
                .expect("Failed to start Tokio runtime for db cloner")
                .block_on(async {
                    if let Err(e) = cloner::DbCloner::run(options).await {
                        eprintln!("{} {}", "❌ Error:".bold().red(), e);
                        std::process::exit(1);
                    }
                });
        }
        Commands::CloneLaravel {
            path,
            dry_run,
            models_only,
            routes_only,
            views_only,
            with_controllers,
            output,
        } => {
            let options = cloner::CloneLaravelOptions {
                path,
                dry_run,
                models_only,
                routes_only,
                views_only,
                with_controllers,
                output,
            };
            if let Err(e) = cloner::LaravelCloner::run(options) {
                eprintln!("{} {}", "❌ Error:".bold().red(), e);
                std::process::exit(1);
            }
        }
        Commands::CloneCi {
            path,
            dry_run,
            models_only,
            routes_only,
            views_only,
            with_controllers,
            output,
        } => {
            let options = cloner::CloneCiOptions {
                path,
                dry_run,
                models_only,
                routes_only,
                views_only,
                with_controllers,
                output,
            };
            if let Err(e) = cloner::CiCloner::run(options) {
                eprintln!("{} {}", "❌ Error:".bold().red(), e);
                std::process::exit(1);
            }
        }
        Commands::CloneApi {
            source,
            dry_run,
            dto_only,
            routes_only,
            with_controllers,
            output,
        } => {
            let options = cloner::CloneApiOptions {
                source,
                dry_run,
                dto_only,
                routes_only,
                with_controllers,
                output,
            };
            if let Err(e) = cloner::ApiCloner::run(options) {
                eprintln!("{} {}", "❌ Error:".bold().red(), e);
                std::process::exit(1);
            }
        }
        Commands::MakeDocker { name, port } => {
            generator::generate_docker(name, port);
        }
        Commands::MakeNginx => {
            generator::generate_nginx();
        }
        Commands::MakeSupervisor => {
            generator::generate_supervisor();
        }
        Commands::PluginNew { name } => {
            generator::generate_plugin_new(&name);
        }
        Commands::PluginBuild { name } => {
            generator::cmd_plugin_build(name.as_deref());
        }
        Commands::PluginList => {
            generator::cmd_plugin_list();
        }
        Commands::PluginMetrics => {
            generator::cmd_plugin_metrics();
        }
        Commands::PluginReload => {
            generator::cmd_plugin_reload();
        }
        Commands::RouteOpenApi {
            output,
            format,
            title,
            version,
        } => {
            generator::generate_openapi(&output, &format, &title, &version);
        }
        Commands::ExportPostman {
            output,
            name,
            version,
        } => {
            generator::export_postman(&output, &name, &version);
        }
        Commands::DesktopInit {
            title,
            width,
            height,
        } => {
            desktop::init_desktop_config(title.as_deref(), width, height);
        }
        Commands::DesktopRun { port } => {
            desktop::run_desktop(port);
        }
        Commands::DesktopBuild { release } => {
            desktop::build_desktop(release);
        }
        Commands::Upgrade {
            cli_only,
            project_only,
            local,
        } => {
            upgrader::run_upgrade(cli_only, project_only, local);
        }
        Commands::Update {
            package,
            dry_run,
            pakem_only,
        } => {
            upgrader::run_update(package, dry_run, pakem_only);
        }
    }
}

fn handle_new_project(name: &str, db: &str, offline: bool) {
    let db_normalized = db.to_lowercase();
    println!("{}", "✨ Welcome to Pakem Framework".bright_purple().bold());
    println!(
        "🚀 Creating new project: {} (Database: {})\n",
        name.bright_blue(),
        db_normalized.bright_yellow()
    );

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );

    pb.set_message("Preparing Pakem project...");
    pb.enable_steady_tick(std::time::Duration::from_millis(120));

    let path = Path::new(name);
    if path.exists() {
        pb.finish_with_message("❌ Error: Directory already exists!");
        std::process::exit(1);
    }

    let mut cloned_successfully = false;

    // Fetch live template from GitHub (pakem-rs/pakem) unless offline flag is set
    if !offline {
        pb.set_message("Fetching latest starter template from GitHub (pakem-rs/pakem)...");
        let git_clone_status = std::process::Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "https://github.com/pakem-rs/pakem.git",
                name,
            ])
            .output();

        if let Ok(output) = git_clone_status {
            if output.status.success() {
                // Clean up cloned .git folder
                let _ = fs::remove_dir_all(path.join(".git"));
                // Personalize Cargo.toml with project name
                let cargo_path = path.join("Cargo.toml");
                if let Ok(content) = fs::read_to_string(&cargo_path) {
                    let customized = content
                        .replace("name = \"pakem-app\"", &format!("name = \"{}\"", name))
                        .replace("name = \"pakem-app\"", &format!("name = \"{}\"", name));
                    let _ = fs::write(&cargo_path, customized);
                }
                // Initialize fresh git repo
                let _ = std::process::Command::new("git")
                    .args(["init", name])
                    .output();
                // Ensure AI context files are created
                ai_context::scaffold_ai_context(path, name).ok();
                cloned_successfully = true;
            }
        }
    }

    if !cloned_successfully {
        pb.set_message("Building project using internal offline template...");
        if let Err(e) = fs::create_dir_all(path) {
            pb.finish_with_message(format!("❌ Error: Could not create directory: {}", e));
            std::process::exit(1);
        }
        skeleton::create_full_skeleton(path, name, &db_normalized);
    }

    pb.finish_with_message("✅ Project created successfully!");

    println!("\n{}", "Next steps:".bright_green().bold());
    println!("  cd {}", name.bright_cyan());
    println!("  canting serve\n");
    println!(
        "  {} AI Agents Ready: AGENTS.md, .cursorrules, .github/copilot-instructions.md, CLAUDE.md",
        "🤖".bright_magenta()
    );
    println!(
        "{}",
        "Happy coding with Pakem & Canting! 🦀🎨".bright_yellow()
    );
}
