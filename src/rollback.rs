use crate::planner::RenameAction;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::path::Path;

pub fn execute_rollback(manifest_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting rollback from: {:?}", manifest_path);

    let file = File::open(manifest_path)?;
    let actions: Vec<RenameAction> = serde_json::from_reader(file)?;

    let pb = ProgressBar::new(actions.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files restored ({eta})",
            )?
            .progress_chars("#>-"),
    );

    let mut restored = 0;
    let mut errors = 0;

    for act in actions {
        if act.original_path == act.new_path {
            pb.inc(1);
            continue;
        }

        if act.new_path.exists() {
            if act.original_path.exists() {
                eprintln!(
                    "\n[WARNING] Source already exists. Not overwriting: {:?}",
                    act.original_path
                );
                errors += 1;
            } else if let Err(e) = std::fs::rename(&act.new_path, &act.original_path) {
                eprintln!("\n[ERROR] Failed to revert {:?}: {}", act.new_path, e);
                errors += 1;
            } else {
                restored += 1;
            }
        } else {
            eprintln!(
                "\n[WARNING] File not found for reversion: {:?}",
                act.new_path
            );
            errors += 1;
        }
        pb.inc(1);
    }

    pb.finish_with_message("Rollback complete!");
    println!(
        "\nRollback Summary:\n  - Restored: {}\n  - Warnings/Errors: {}",
        restored, errors
    );

    Ok(())
}
