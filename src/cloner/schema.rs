use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub name: String,
    pub model_name: String,
    pub columns: Vec<ColumnSchema>,
    pub primary_keys: Vec<String>,
    pub foreign_keys: Vec<ForeignKeySchema>,
    pub relations: Vec<RelationDef>,
    pub is_pivot: bool,
    pub has_soft_deletes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    pub name: String,
    pub db_type: String,
    pub rust_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeySchema {
    pub column_name: String,
    pub foreign_table: String,
    pub foreign_column: String,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RelationType {
    BelongsTo,
    HasMany,
    BelongsToMany,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationDef {
    pub relation_type: RelationType,
    pub target_model: String,
    pub foreign_key: String,
    pub pivot_table: Option<String>,
    pub local_key: Option<String>,
}

impl TableSchema {
    pub fn new(name: &str) -> Self {
        let model_name = table_name_to_model_name(name);
        Self {
            name: name.to_string(),
            model_name,
            columns: Vec::new(),
            primary_keys: Vec::new(),
            foreign_keys: Vec::new(),
            relations: Vec::new(),
            is_pivot: false,
            has_soft_deletes: false,
        }
    }
}

/// Mengubah nama tabel plural (snake_case) menjadi nama Model CamelCase singular.
/// Contoh: "users" -> "User", "order_items" -> "OrderItem", "categories" -> "Category"
pub fn table_name_to_model_name(table_name: &str) -> String {
    let singular = singularize(table_name);
    snake_to_camel(&singular)
}

/// Helper singularize sederhana untuk konvensi tabel bahasa Inggris
pub fn singularize(word: &str) -> String {
    let lower = word.to_lowercase();
    if lower.ends_with("ies") && lower.len() > 3 {
        format!("{}y", &lower[..lower.len() - 3])
    } else if lower.ends_with("ses") && lower.len() > 3 {
        lower[..lower.len() - 2].to_string()
    } else if lower.ends_with("xes") && lower.len() > 3 {
        lower[..lower.len() - 2].to_string()
    } else if lower.ends_with("ches") && lower.len() > 4 {
        lower[..lower.len() - 2].to_string()
    } else if lower.ends_with("shes") && lower.len() > 4 {
        lower[..lower.len() - 2].to_string()
    } else if lower.ends_with('s') && !lower.ends_with("ss") && lower.len() > 1 {
        lower[..lower.len() - 1].to_string()
    } else {
        lower
    }
}

/// Mengubah snake_case ke CamelCase
pub fn snake_to_camel(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' || c == '-' || c == ' ' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    if result.is_empty() {
        s.to_string()
    } else {
        result
    }
}

/// Mengubah CamelCase ke snake_case
pub fn camel_to_snake(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}
