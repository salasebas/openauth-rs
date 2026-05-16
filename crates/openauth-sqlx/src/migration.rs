//! Structured schema migration planning for SQLx adapters.

/// Additive schema changes planned for a live database.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchemaMigrationPlan {
    pub to_be_created: Vec<TableToCreate>,
    pub to_be_added: Vec<ColumnToAdd>,
    pub indexes_to_be_created: Vec<IndexToCreate>,
    pub warnings: Vec<SchemaMigrationWarning>,
    pub statements: Vec<MigrationStatement>,
}

impl SchemaMigrationPlan {
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }

    pub fn compile(&self) -> String {
        if self.statements.is_empty() {
            return ";".to_owned();
        }

        format!(
            "{};",
            self.statements
                .iter()
                .map(|statement| statement.sql.as_str())
                .collect::<Vec<_>>()
                .join(";\n\n")
        )
    }
}

/// A table missing from the database and planned for creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableToCreate {
    pub logical_name: String,
    pub table_name: String,
}

/// A column missing from an existing table and planned for additive creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnToAdd {
    pub table_logical_name: String,
    pub table_name: String,
    pub field_logical_name: String,
    pub column_name: String,
}

/// A standalone index missing from the database and planned for creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexToCreate {
    pub table_logical_name: String,
    pub table_name: String,
    pub field_logical_name: String,
    pub column_name: String,
    pub index_name: String,
}

/// Non-executable findings discovered while planning migrations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaMigrationWarning {
    ColumnTypeMismatch {
        table_name: String,
        column_name: String,
        expected: String,
        actual: String,
    },
}

/// A SQL statement emitted by a migration plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationStatement {
    pub kind: MigrationStatementKind,
    pub sql: String,
}

/// The additive operation represented by a migration statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationStatementKind {
    CreateTable,
    AddColumn,
    CreateIndex,
}
