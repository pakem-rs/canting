use super::schema::{table_name_to_model_name, RelationDef, RelationType, TableSchema};
use std::collections::HashMap;

pub struct RelationAnalyzer;

impl RelationAnalyzer {
    /// Menganalisis seluruh tabel dalam skema dan mengisi relasi deklaratif
    pub fn analyze(tables: &mut [TableSchema]) {
        // Peta nama tabel -> nama model
        let table_to_model: HashMap<String, String> = tables
            .iter()
            .map(|t| (t.name.clone(), t.model_name.clone()))
            .collect();

        // 1. Deteksi Pivot Tables (Many-to-Many)
        let mut pivot_relations: Vec<(String, RelationDef)> = Vec::new();

        for table in tables.iter_mut() {
            // Syarat tabel pivot: memiliki tepat 2 Foreign Keys dan kolomnya relatif sedikit (<= 5)
            if table.foreign_keys.len() == 2 && table.columns.len() <= 5 {
                table.is_pivot = true;
                let fk1 = &table.foreign_keys[0];
                let fk2 = &table.foreign_keys[1];

                if let (Some(m1), Some(m2)) = (
                    table_to_model.get(&fk1.foreign_table),
                    table_to_model.get(&fk2.foreign_table),
                ) {
                    // Relasi untuk Model 1 -> Model 2
                    pivot_relations.push((
                        fk1.foreign_table.clone(),
                        RelationDef {
                            relation_type: RelationType::BelongsToMany,
                            target_model: m2.clone(),
                            foreign_key: fk1.column_name.clone(),
                            pivot_table: Some(table.name.clone()),
                            local_key: Some(fk2.column_name.clone()),
                        },
                    ));

                    // Relasi untuk Model 2 -> Model 1
                    pivot_relations.push((
                        fk2.foreign_table.clone(),
                        RelationDef {
                            relation_type: RelationType::BelongsToMany,
                            target_model: m1.clone(),
                            foreign_key: fk2.column_name.clone(),
                            pivot_table: Some(table.name.clone()),
                            local_key: Some(fk1.column_name.clone()),
                        },
                    ));
                }
            }
        }

        // Terapkan pivot relations
        for (tbl_name, rel) in pivot_relations {
            if let Some(target_table) = tables.iter_mut().find(|t| t.name == tbl_name) {
                if !target_table.relations.iter().any(|r| {
                    r.relation_type == RelationType::BelongsToMany
                        && r.target_model == rel.target_model
                        && r.pivot_table == rel.pivot_table
                }) {
                    target_table.relations.push(rel);
                }
            }
        }

        // 2. Deteksi 1-to-N (BelongsTo dan HasMany) untuk tabel non-pivot
        let mut one_to_many_pairs: Vec<(String, String, String)> = Vec::new(); // (parent_table, child_table, fk_column)

        for table in tables.iter_mut() {
            if table.is_pivot {
                continue;
            }

            for fk in &table.foreign_keys {
                let target_model = table_to_model
                    .get(&fk.foreign_table)
                    .cloned()
                    .unwrap_or_else(|| table_name_to_model_name(&fk.foreign_table));

                // Tambahkan belongs_to pada model anak (child)
                if !table.relations.iter().any(|r| {
                    r.relation_type == RelationType::BelongsTo
                        && r.target_model == target_model
                        && r.foreign_key == fk.column_name
                }) {
                    table.relations.push(RelationDef {
                        relation_type: RelationType::BelongsTo,
                        target_model,
                        foreign_key: fk.column_name.clone(),
                        pivot_table: None,
                        local_key: None,
                    });
                }

                one_to_many_pairs.push((
                    fk.foreign_table.clone(),
                    table.name.clone(),
                    fk.column_name.clone(),
                ));
            }
        }

        // Terapkan has_many pada model induk (parent)
        for (parent_tbl, child_tbl, fk_col) in one_to_many_pairs {
            let child_model = table_to_model
                .get(&child_tbl)
                .cloned()
                .unwrap_or_else(|| table_name_to_model_name(&child_tbl));

            if let Some(parent) = tables.iter_mut().find(|t| t.name == parent_tbl) {
                if !parent.relations.iter().any(|r| {
                    r.relation_type == RelationType::HasMany
                        && r.target_model == child_model
                        && r.foreign_key == fk_col
                }) {
                    parent.relations.push(RelationDef {
                        relation_type: RelationType::HasMany,
                        target_model: child_model,
                        foreign_key: fk_col,
                        pivot_table: None,
                        local_key: None,
                    });
                }
            }
        }
    }
}
