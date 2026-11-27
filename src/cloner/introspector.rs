use super::schema::{ColumnSchema, ForeignKeySchema, TableSchema};
use sqlx::{Any, Pool, Row};

pub struct Introspector {
    pub pool: Pool<Any>,
    pub driver: String,
}

impl Introspector {
    pub fn new(pool: Pool<Any>, driver: String) -> Self {
        Self { pool, driver }
    }

    /// Mengambil daftar semua tabel yang ada di database (selain tabel internal framework)
    pub async fn list_tables(&self) -> Result<Vec<String>, String> {
        let tables = match self.driver.as_str() {
            "sqlite" => {
                let rows = sqlx::query(
                    "SELECT name FROM sqlite_master WHERE type='table' \
                     AND name NOT LIKE 'sqlite_%' \
                     AND name NOT LIKE '_pakem_%' \
                     ORDER BY name"
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| format!("Gagal membaca daftar tabel SQLite: {}", e))?;

                rows.into_iter()
                    .filter_map(|r| r.try_get::<String, _>("name").ok())
                    .collect()
            }
            "postgres" | "postgresql" => {
                let rows = sqlx::query(
                    "SELECT table_name FROM information_schema.tables \
                     WHERE table_schema = 'public' \
                     AND table_type = 'BASE TABLE' \
                     AND table_name NOT LIKE '_pakem_%' \
                     ORDER BY table_name"
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| format!("Gagal membaca daftar tabel PostgreSQL: {}", e))?;

                rows.into_iter()
                    .filter_map(|r| r.try_get::<String, _>("table_name").ok())
                    .collect()
            }
            "mysql" | "mariadb" => {
                let rows = sqlx::query(
                    "SELECT table_name FROM information_schema.tables \
                     WHERE table_schema = DATABASE() \
                     AND table_type = 'BASE TABLE' \
                     AND table_name NOT LIKE '_pakem_%' \
                     ORDER BY table_name"
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| format!("Gagal membaca daftar tabel MySQL: {}", e))?;

                rows.into_iter()
                    .filter_map(|r| r.try_get::<String, _>("table_name").ok())
                    .collect()
            }
            _ => return Err(format!("Driver database '{}' belum didukung untuk introspeksi", self.driver)),
        };

        Ok(tables)
    }

    /// Membaca struktur lengkap satu tabel: kolom, tipe data, primary key, dan foreign key
    pub async fn inspect_table(&self, table_name: &str) -> Result<TableSchema, String> {
        let mut schema = TableSchema::new(table_name);

        match self.driver.as_str() {
            "sqlite" => {
                self.inspect_sqlite_table(&mut schema).await?;
            }
            "postgres" | "postgresql" => {
                self.inspect_postgres_table(&mut schema).await?;
            }
            "mysql" | "mariadb" => {
                self.inspect_mysql_table(&mut schema).await?;
            }
            _ => return Err(format!("Driver '{}' tidak didukung", self.driver)),
        }

        // Cek apakah tabel memiliki soft deletes
        schema.has_soft_deletes = schema.columns.iter().any(|c| c.name == "deleted_at");

        Ok(schema)
    }

    async fn inspect_sqlite_table(&self, schema: &mut TableSchema) -> Result<(), String> {
        // 1. Kolom & Primary Keys via PRAGMA table_info
        let query_info = format!("PRAGMA table_info('{}')", schema.name);
        let rows = sqlx::query(&query_info)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("Gagal query PRAGMA table_info('{}'): {}", schema.name, e))?;

        for row in rows {
            let col_name: String = row.try_get("name").map_err(|e| e.to_string())?;
            let col_type: String = row.try_get("type").unwrap_or_else(|_| "TEXT".to_string());
            let notnull: i32 = row.try_get("notnull").unwrap_or(0);
            let pk: i32 = row.try_get("pk").unwrap_or(0);
            let dflt_val: Option<String> = row.try_get("dflt_value").ok();

            let is_nullable = notnull == 0 && pk == 0;
            let is_primary_key = pk > 0;
            if is_primary_key {
                schema.primary_keys.push(col_name.clone());
            }

            let rust_type = map_sql_type_to_rust(&col_type, is_nullable);

            schema.columns.push(ColumnSchema {
                name: col_name,
                db_type: col_type,
                rust_type,
                is_nullable,
                is_primary_key,
                default_value: dflt_val,
            });
        }

        // 2. Foreign Keys via PRAGMA foreign_key_list
        let query_fk = format!("PRAGMA foreign_key_list('{}')", schema.name);
        if let Ok(fk_rows) = sqlx::query(&query_fk).fetch_all(&self.pool).await {
            for row in fk_rows {
                let from_col: String = row.try_get("from").unwrap_or_default();
                let to_table: String = row.try_get("table").unwrap_or_default();
                let to_col: String = row.try_get("to").unwrap_or_else(|_| "id".to_string());
                let on_update: Option<String> = row.try_get("on_update").ok();
                let on_delete: Option<String> = row.try_get("on_delete").ok();

                if !from_col.is_empty() && !to_table.is_empty() {
                    schema.foreign_keys.push(ForeignKeySchema {
                        column_name: from_col,
                        foreign_table: to_table,
                        foreign_column: to_col,
                        on_delete,
                        on_update,
                    });
                }
            }
        }

        Ok(())
    }

    async fn inspect_postgres_table(&self, schema: &mut TableSchema) -> Result<(), String> {
        // 1. Kolom via information_schema.columns
        let rows = sqlx::query(
            "SELECT column_name, data_type, udt_name, is_nullable, column_default \
             FROM information_schema.columns \
             WHERE table_schema = 'public' AND table_name = $1 \
             ORDER BY ordinal_position"
        )
        .bind(&schema.name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Gagal membaca kolom postgres: {}", e))?;

        for row in rows {
            let col_name: String = row.try_get("column_name").map_err(|e| e.to_string())?;
            let data_type: String = row.try_get("data_type").unwrap_or_default();
            let udt_name: String = row.try_get("udt_name").unwrap_or_default();
            let is_nullable_str: String = row.try_get("is_nullable").unwrap_or_else(|_| "YES".to_string());
            let dflt_val: Option<String> = row.try_get("column_default").ok();

            let is_nullable = is_nullable_str.to_uppercase() == "YES";
            let effective_type = if !udt_name.is_empty() { udt_name } else { data_type };
            let rust_type = map_sql_type_to_rust(&effective_type, is_nullable);

            schema.columns.push(ColumnSchema {
                name: col_name,
                db_type: effective_type,
                rust_type,
                is_nullable,
                is_primary_key: false, // akan diupdate via constraint
                default_value: dflt_val,
            });
        }

        // 2. Primary Keys & Foreign Keys via pg_constraint
        let pk_rows = sqlx::query(
            "SELECT kcu.column_name \
             FROM information_schema.table_constraints tc \
             JOIN information_schema.key_column_usage kcu \
               ON tc.constraint_name = kcu.constraint_name \
               AND tc.table_schema = kcu.table_schema \
             WHERE tc.constraint_type = 'PRIMARY KEY' \
               AND tc.table_schema = 'public' \
               AND tc.table_name = $1"
        )
        .bind(&schema.name)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        for r in pk_rows {
            if let Ok(pk_col) = r.try_get::<String, _>("column_name") {
                schema.primary_keys.push(pk_col.clone());
                for c in &mut schema.columns {
                    if c.name == pk_col {
                        c.is_primary_key = true;
                    }
                }
            }
        }

        // Foreign keys
        let fk_rows = sqlx::query(
            "SELECT \
                kcu.column_name, \
                ccu.table_name AS foreign_table_name, \
                ccu.column_name AS foreign_column_name \
             FROM information_schema.table_constraints AS tc \
             JOIN information_schema.key_column_usage AS kcu \
               ON tc.constraint_name = kcu.constraint_name \
               AND tc.table_schema = kcu.table_schema \
             JOIN information_schema.constraint_column_usage AS ccu \
               ON ccu.constraint_name = tc.constraint_name \
               AND ccu.table_schema = tc.table_schema \
             WHERE tc.constraint_type = 'FOREIGN KEY' \
               AND tc.table_schema = 'public' \
               AND tc.table_name = $1"
        )
        .bind(&schema.name)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        for r in fk_rows {
            let col: String = r.try_get("column_name").unwrap_or_default();
            let f_table: String = r.try_get("foreign_table_name").unwrap_or_default();
            let f_col: String = r.try_get("foreign_column_name").unwrap_or_default();

            if !col.is_empty() && !f_table.is_empty() {
                schema.foreign_keys.push(ForeignKeySchema {
                    column_name: col,
                    foreign_table: f_table,
                    foreign_column: f_col,
                    on_delete: None,
                    on_update: None,
                });
            }
        }

        Ok(())
    }

    async fn inspect_mysql_table(&self, schema: &mut TableSchema) -> Result<(), String> {
        let rows = sqlx::query(
            "SELECT column_name, data_type, is_nullable, column_default, column_key \
             FROM information_schema.columns \
             WHERE table_schema = DATABASE() AND table_name = ? \
             ORDER BY ordinal_position"
        )
        .bind(&schema.name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Gagal membaca kolom mysql: {}", e))?;

        for row in rows {
            let col_name: String = row.try_get("column_name").map_err(|e| e.to_string())?;
            let data_type: String = row.try_get("data_type").unwrap_or_default();
            let is_nullable_str: String = row.try_get("is_nullable").unwrap_or_else(|_| "YES".to_string());
            let dflt_val: Option<String> = row.try_get("column_default").ok();
            let col_key: String = row.try_get("column_key").unwrap_or_default();

            let is_nullable = is_nullable_str.to_uppercase() == "YES";
            let is_primary_key = col_key.to_uppercase() == "PRI";

            if is_primary_key {
                schema.primary_keys.push(col_name.clone());
            }

            let rust_type = map_sql_type_to_rust(&data_type, is_nullable);

            schema.columns.push(ColumnSchema {
                name: col_name,
                db_type: data_type,
                rust_type,
                is_nullable,
                is_primary_key,
                default_value: dflt_val,
            });
        }

        // Foreign keys
        let fk_rows = sqlx::query(
            "SELECT column_name, referenced_table_name, referenced_column_name \
             FROM information_schema.key_column_usage \
             WHERE table_schema = DATABASE() \
               AND table_name = ? \
               AND referenced_table_name IS NOT NULL"
        )
        .bind(&schema.name)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        for r in fk_rows {
            let col: String = r.try_get("column_name").unwrap_or_default();
            let f_table: String = r.try_get("referenced_table_name").unwrap_or_default();
            let f_col: String = r.try_get("referenced_column_name").unwrap_or_default();

            if !col.is_empty() && !f_table.is_empty() {
                schema.foreign_keys.push(ForeignKeySchema {
                    column_name: col,
                    foreign_table: f_table,
                    foreign_column: f_col,
                    on_delete: None,
                    on_update: None,
                });
            }
        }

        Ok(())
    }
}

/// Memetakan tipe SQL mentah menjadi tipe data Rust idiomatik
pub fn map_sql_type_to_rust(sql_type: &str, is_nullable: bool) -> String {
    let lower = sql_type.to_lowercase();
    let base_type = if lower.contains("bigint") || lower.contains("int8") || lower.contains("bigserial") {
        "i64"
    } else if lower.contains("tinyint(1)") || lower.contains("bool") || lower.contains("boolean") {
        "bool"
    } else if lower.contains("tinyint") || lower.contains("int2") || lower.contains("smallint") {
        "i16"
    } else if lower.contains("int") || lower.contains("serial") || lower.contains("integer") {
        "i64" // Standar default primary key id di Pakem & Laravel
    } else if lower.contains("decimal") || lower.contains("numeric") || lower.contains("float") || lower.contains("double") || lower.contains("real") {
        "f64"
    } else if lower.contains("json") {
        "serde_json::Value"
    } else {
        // varchar, text, char, enum, uuid, datetime, timestamp
        "String"
    };

    if is_nullable {
        format!("Option<{}>", base_type)
    } else {
        base_type.to_string()
    }
}
