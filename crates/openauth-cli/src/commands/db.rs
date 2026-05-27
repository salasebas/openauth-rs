use std::path::Path;

use crate::app::{AppContext, AppError, GenerateArgs, MigrateArgs, StatusArgs};
use crate::db::{self, DbCliError, MigrationOutput};
use crate::output::print_json;
use crate::prompt::confirm;
use crate::schema::{dialect_from_provider, dialect_name};
use serde_json::{Map, Value};

pub async fn status(context: &AppContext, args: StatusArgs) -> Result<(), AppError> {
    let config = context.load_config()?;
    let planned = db::plan_with_base(&config, false, Some(context.cwd())).await?;
    let summary = planned.summary();
    if args.json {
        print_json(&summary)?;
    } else {
        print_plan(&planned);
    }
    if args.check && !planned.plan.is_empty() {
        return Err(AppError::SilentExit { code: 1 });
    }
    Ok(())
}

pub async fn generate(context: &AppContext, args: GenerateArgs) -> Result<(), AppError> {
    let config = context.load_config()?;
    let planned = match db::plan_with_base(&config, args.from_empty, Some(context.cwd())).await {
        Ok(planned) => planned,
        Err(error @ DbCliError::UnsupportedAdapter(_)) => {
            crate::telemetry::publish_generate_with_extra(
                &config,
                "unsupported_adapter",
                unsupported_adapter_payload(&config),
            )
            .await;
            return Err(error.into());
        }
        Err(error @ DbCliError::UnsupportedProvider(_)) => {
            crate::telemetry::publish_generate_with_extra(
                &config,
                "unsupported_database",
                unsupported_adapter_payload(&config),
            )
            .await;
            return Err(error.into());
        }
        Err(error) => return Err(error.into()),
    };
    if planned.plan.is_empty() {
        println!("Schema is already up to date.");
        crate::telemetry::publish_generate(&config, "no_changes").await;
        return Ok(());
    }
    let output = migration_output(
        context,
        &config,
        args.output.as_deref(),
        args.output_dir.as_deref(),
    )?;
    let path = db::write_migration_output(&config, &planned, output, args.force)?;
    println!("Generated migration: {}", path.display());
    crate::telemetry::publish_generate(&config, "generated").await;
    Ok(())
}

pub async fn migrate(context: &AppContext, args: MigrateArgs) -> Result<(), AppError> {
    let config = context.load_config()?;
    let planned = match db::plan_with_base(&config, false, Some(context.cwd())).await {
        Ok(planned) => planned,
        Err(error @ DbCliError::UnsupportedAdapter(_)) => {
            crate::telemetry::publish_migrate_with_extra(
                &config,
                "unsupported_adapter",
                unsupported_adapter_payload(&config),
            )
            .await;
            return Err(error.into());
        }
        Err(error @ DbCliError::UnsupportedProvider(_)) => {
            crate::telemetry::publish_migrate_with_extra(
                &config,
                "unsupported_database",
                unsupported_adapter_payload(&config),
            )
            .await;
            return Err(error.into());
        }
        Err(error) => return Err(error.into()),
    };
    if planned.plan.is_empty() {
        println!("No migrations needed.");
        crate::telemetry::publish_migrate(&config, "no_changes").await;
        return Ok(());
    }
    print_plan(&planned);
    if args.dry_run {
        println!("Dry run complete; no changes were applied.");
        crate::telemetry::publish_migrate(&config, "dry_run").await;
        return Ok(());
    }
    if !confirm("Apply these migrations?", args.yes)? {
        println!("Migration cancelled.");
        crate::telemetry::publish_migrate(&config, "aborted").await;
        return Ok(());
    }
    db::migrate_with_base(&config, Some(context.cwd())).await?;
    println!("Migration completed successfully.");
    crate::telemetry::publish_migrate(&config, "migrated").await;
    Ok(())
}

fn unsupported_adapter_payload(config: &crate::config::CliConfig) -> Map<String, Value> {
    let mut payload = Map::new();
    payload.insert(
        "adapter".to_owned(),
        Value::String(config.database.adapter.clone()),
    );
    if let Some(provider) = &config.database.provider {
        payload.insert("database".to_owned(), Value::String(provider.clone()));
    }
    payload
}

fn migration_output(
    context: &AppContext,
    config: &crate::config::CliConfig,
    output: Option<&Path>,
    output_dir: Option<&Path>,
) -> Result<MigrationOutput, AppError> {
    match (output, output_dir) {
        (Some(_), Some(_)) => Err(AppError::Message(
            "Use only one of --output or --output-dir.".to_owned(),
        )),
        (Some(path), None) => {
            let resolved = context.resolve_project_path(path);
            if resolved
                .extension()
                .and_then(|extension| extension.to_str())
                == Some("sql")
            {
                Ok(MigrationOutput::File(resolved))
            } else {
                eprintln!(
                    "warning: --output without a .sql extension is treated as a directory; prefer --output-dir"
                );
                Ok(MigrationOutput::Directory(resolved))
            }
        }
        (None, Some(path)) => {
            let resolved = context.resolve_project_path(path);
            Ok(MigrationOutput::Directory(resolved))
        }
        (None, None) => Ok(MigrationOutput::Directory(
            context.cwd().join(&config.database.migrations_dir),
        )),
    }
}

fn print_plan(planned: &db::PlannedMigration) {
    let dialect = dialect_from_provider(&planned.provider)
        .map(dialect_name)
        .unwrap_or("unknown");
    println!("OpenAuth schema plan ({dialect})");
    println!("Tables to create: {}", planned.plan.to_be_created.len());
    for table in &planned.plan.to_be_created {
        println!("  - {}", table.table_name);
    }
    println!("Columns to add: {}", planned.plan.to_be_added.len());
    for column in &planned.plan.to_be_added {
        println!("  - {}.{}", column.table_name, column.column_name);
    }
    println!(
        "Indexes to create: {}",
        planned.plan.indexes_to_be_created.len()
    );
    for index in &planned.plan.indexes_to_be_created {
        println!("  - {}", index.index_name);
    }
    for warning in &planned.plan.warnings {
        println!("WARNING: {warning:?}");
    }
}
