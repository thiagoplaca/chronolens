use crate::planner::RenameAction;
use chrono::Local;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::path::{Path, PathBuf};

pub fn execute_plan(
    actions: &[RenameAction],
    target_dir: &Path,
    dry_run: bool,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let to_rename_count = actions
        .iter()
        .filter(|a| a.original_path != a.new_path)
        .count();

    if to_rename_count == 0 {
        println!("All files already have the correct names! No changes needed.");
        return Ok(None);
    }

    if dry_run {
        println!("\n=== SIMULATION MODE (DRY-RUN) ===");
        println!("No files were modified on disk.");
        println!("Total files to rename: {}", to_rename_count);
        println!("\nSample of the first operations:");
        for act in actions
            .iter()
            .filter(|a| a.original_path != a.new_path)
            .take(15)
        {
            println!("  [DRY-RUN] {} -> {}", act.original_name, act.new_name);
        }
        if to_rename_count > 15 {
            println!("  ... and {} more files.", to_rename_count - 15);
        }
        println!("\nTo apply the real changes safely, run again with the --execute flag.");
        return Ok(None);
    }

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let manifest_name = format!("rollback_manifest_{}.json", timestamp);
    let manifest_path = target_dir.join(&manifest_name);

    {
        let manifest_file = File::create(&manifest_path)?;
        serde_json::to_writer_pretty(manifest_file, actions)?;
    }
    println!("\n[SAFETY] Rollback manifest written to:");
    println!("  {:?}", manifest_path);
    println!(
        "If you need to undo the operation, use: --rollback {:?}",
        manifest_path
    );

    let pb = ProgressBar::new(actions.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "[{elapsed_precise}] [{bar:40.green/white}] {pos}/{len} files processed ({eta})",
            )?
            .progress_chars("=>-"),
    );

    let mut success_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;

    for act in actions {
        if act.original_path == act.new_path {
            skipped_count += 1;
            pb.inc(1);
            continue;
        }

        if !act.original_path.exists() {
            eprintln!(
                "\n[WARNING] Original file not found: {:?}",
                act.original_path
            );
            error_count += 1;
            pb.inc(1);
            continue;
        }

        if act.new_path.exists() {
            eprintln!(
                "\n[SAFETY WARNING] Destination already exists, operation aborted to protect the file: {:?}",
                act.new_path
            );
            error_count += 1;
            pb.inc(1);
            continue;
        }

        match std::fs::rename(&act.original_path, &act.new_path) {
            Ok(_) => {
                success_count += 1;
            }
            Err(e) => {
                eprintln!(
                    "\n[ERROR] Failed to rename {:?} to {:?}: {}",
                    act.original_path, act.new_path, e
                );
                error_count += 1;
            }
        }
        pb.inc(1);
    }

    pb.finish_with_message("Finished!");

    println!("\n=== FINAL REPORT ===");
    println!("  - Renamed successfully: {}", success_count);
    println!("  - Unchanged (already correct): {}", skipped_count);
    println!("  - Errors / safety blocks: {}", error_count);

    Ok(Some(manifest_path))
}
