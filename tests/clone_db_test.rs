use canting::cloner::codegen::CodeGenerator;
use canting::cloner::introspector::Introspector;
use canting::cloner::relation_analyzer::RelationAnalyzer;
use canting::cloner::schema::RelationType;
use sqlx::Executor;
use std::fs;

#[tokio::test]
async fn test_clone_db_full_introspection_and_relation_inference() {
    sqlx::any::install_default_drivers();

    let test_db_path = "./test_legacy_clone.sqlite";
    let _ = fs::remove_file(test_db_path);
    fs::File::create(test_db_path).expect("Failed to create empty test db file");

    let db_url = format!("sqlite:{}", test_db_path);
    let any_pool = canting::migrator::connect_db(&db_url)
        .await
        .expect("Failed to connect any pool");

    // Skema legacy: Users, Roles, Pivot UserRoles, Posts, Comments
    let ddl = "
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT NOT NULL,
            created_at TEXT,
            updated_at TEXT,
            deleted_at TEXT
        );

        CREATE TABLE roles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            slug TEXT NOT NULL
        );

        CREATE TABLE user_roles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            role_id INTEGER NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
        );

        CREATE TABLE posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            is_published INTEGER NOT NULL DEFAULT 1,
            views INTEGER NOT NULL DEFAULT 0,
            created_at TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );

        CREATE TABLE comments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            post_id INTEGER NOT NULL,
            author_name TEXT NOT NULL,
            comment_body TEXT NOT NULL,
            created_at TEXT,
            FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE
        );
    ";

    any_pool.execute(ddl).await.expect("Failed to execute DDL on any_pool");

    let introspector = Introspector::new(any_pool, "sqlite".to_string());

    // 1. Test list_tables
    let tables = introspector.list_tables().await.expect("Failed to list tables");
    assert_eq!(tables.len(), 5);
    assert!(tables.contains(&"users".to_string()));
    assert!(tables.contains(&"roles".to_string()));
    assert!(tables.contains(&"user_roles".to_string()));
    assert!(tables.contains(&"posts".to_string()));
    assert!(tables.contains(&"comments".to_string()));

    // 2. Test inspect_table
    let mut schemas = Vec::new();
    for t in &tables {
        let schema = introspector.inspect_table(t).await.expect("Failed to inspect");
        schemas.push(schema);
    }

    // Periksa kolom tabel users
    let user_table = schemas.iter().find(|t| t.name == "users").unwrap();
    assert_eq!(user_table.model_name, "User");
    assert!(user_table.has_soft_deletes);
    let email_col = user_table.columns.iter().find(|c| c.name == "email").unwrap();
    assert_eq!(email_col.rust_type, "String");
    let del_col = user_table.columns.iter().find(|c| c.name == "deleted_at").unwrap();
    assert_eq!(del_col.rust_type, "Option<String>");

    // 3. Test RelationAnalyzer
    RelationAnalyzer::analyze(&mut schemas);

    // Cek Pivot table user_roles
    let pivot = schemas.iter().find(|t| t.name == "user_roles").unwrap();
    assert!(pivot.is_pivot, "user_roles harus dikenali sebagai pivot table!");

    // Cek relasi User
    let user = schemas.iter().find(|t| t.name == "users").unwrap();
    assert!(
        user.relations.iter().any(|r| r.relation_type == RelationType::HasMany && r.target_model == "Post"),
        "User harus memiliki has_many!(Post)!"
    );
    assert!(
        user.relations.iter().any(|r| r.relation_type == RelationType::BelongsToMany && r.target_model == "Role"),
        "User harus memiliki belongs_to_many!(Role)!"
    );

    // Cek relasi Post
    let post = schemas.iter().find(|t| t.name == "posts").unwrap();
    assert!(
        post.relations.iter().any(|r| r.relation_type == RelationType::BelongsTo && r.target_model == "User"),
        "Post harus memiliki belongs_to!(User)!"
    );
    assert!(
        post.relations.iter().any(|r| r.relation_type == RelationType::HasMany && r.target_model == "Comment"),
        "Post harus memiliki has_many!(Comment)!"
    );

    // Cek relasi Comment
    let comment = schemas.iter().find(|t| t.name == "comments").unwrap();
    assert!(
        comment.relations.iter().any(|r| r.relation_type == RelationType::BelongsTo && r.target_model == "Post"),
        "Comment harus memiliki belongs_to!(Post)!"
    );

    // 4. Test CodeGenerator
    let user_code = CodeGenerator::generate_model(user);
    assert!(user_code.contains("pub struct User {"));
    assert!(user_code.contains("#[table(\"users\")]"));
    assert!(user_code.contains("has_many!(Post, \"user_id\");"));
    assert!(user_code.contains("belongs_to_many!(Role, \"user_roles\", \"user_id\", \"role_id\");"));

    let post_code = CodeGenerator::generate_model(post);
    assert!(post_code.contains("belongs_to!(User, \"user_id\");"));
    assert!(post_code.contains("has_many!(Comment, \"post_id\");"));

    let user_mig = CodeGenerator::generate_migration(user, "sqlite");
    assert!(user_mig.contains("CREATE TABLE IF NOT EXISTS users"));

    let post_ctrl = CodeGenerator::generate_controller(post);
    assert!(post_ctrl.contains("pub struct PostController;"));
    assert!(post_ctrl.contains("Post::paginate("));

    // Bersihkan file test
    let _ = fs::remove_file(test_db_path);

    println!("✅ Semua pengujian reverse engineering & smart relation inference lulus 100%!");
}
