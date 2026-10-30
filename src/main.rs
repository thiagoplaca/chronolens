mod executor;
mod extractor;
mod planner;
mod rollback;

use clap::Parser;
use extractor::extract_photo_timestamp;
use indicatif::{ProgressBar, ProgressStyle};
use planner::{plan_renames, PhotoCandidate};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "heic", "heif", "webp", "tiff", "tif", "raw", "cr2", "cr3", "nef", "arw",
    "dng", "bmp",
];

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "m4v", "avi", "mkv", "wmv", "flv", "webm", "3gp", "mts", "m2ts",
];

#[derive(Parser, Debug)]
#[command(
    name = "chronolens",
    about = "Safely batch-rename thousands of photos and videos to the 'DD-MM-YYYY' pattern, falling back to 'DD-MM-YYYY-HHhMMmSSs' when multiple files share the same day.",
    version = "0.2.0"
)]
struct Args {
    /// Path to the folder containing the photos/videos
    #[arg(value_name = "DIRECTORY")]
    directory: Option<PathBuf>,

    /// Applies the actual renames on disk. If omitted, runs in dry-run mode.
    #[arg(long, default_value_t = false)]
    execute: bool,

    /// Scan subdirectories recursively
    #[arg(short, long, default_value_t = false)]
    recursive: bool,

    /// Include video files (mp4, mov, mkv, avi, webm, etc.) in the process
    #[arg(short = 'v', long, default_value_t = false)]
    include_videos: bool,

    /// Process ONLY video files (ignores photos)
    #[arg(long, default_value_t = false)]
    only_videos: bool,

    /// Run a rollback using a previously generated manifest file
    #[arg(long, value_name = "MANIFEST_JSON")]
    rollback: Option<PathBuf>,
}

fn is_target_file(path: &Path, include_videos: bool, only_videos: bool) -> bool {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_lowercase(),
        None => return false,
    };

    let is_img = IMAGE_EXTENSIONS.contains(&ext.as_str());
    let is_vid = VIDEO_EXTENSIONS.contains(&ext.as_str());

    if only_videos {
        is_vid
    } else if include_videos {
        is_img || is_vid
    } else {
        is_img
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if let Some(ref manifest) = args.rollback {
        return rollback::execute_rollback(manifest);
    }

    let target_dir = match args.directory {
        Some(dir) => dir,
        None => {
            eprintln!("Error: Provide a folder path or use --rollback <manifest.json>");
            std::process::exit(1);
        }
    };

    if !target_dir.is_dir() {
        eprintln!(
            "Error: The specified path is not a valid directory: {:?}",
            target_dir
        );
        std::process::exit(1);
    }

    let media_mode_desc = if args.only_videos {
        "Videos only"
    } else if args.include_videos {
        "Photos and videos"
    } else {
        "Photos only (use -v or --include-videos to include videos)"
    };

    println!("==================================================");
    println!("       MEDIA RENAMER (BULK PHOTOS & VIDEOS)       ");
    println!("==================================================");
    println!("Working directory:    {:?}", target_dir);
    println!("Files to include:     {}", media_mode_desc);
    println!(
        "Recursive mode:       {}",
        if args.recursive { "Yes" } else { "No" }
    );
    println!(
        "Execution mode:       {}",
        if args.execute {
            "REAL (MODIFYING FILES)"
        } else {
            "SIMULATION (SAFE DRY-RUN)"
        }
    );
    println!("--------------------------------------------------");

    println!("Scanning files in the folder...");
    let mut walker = WalkDir::new(&target_dir);
    if !args.recursive {
        walker = walker.max_depth(1);
    }

    let entries: Vec<PathBuf> = walker
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| is_target_file(p, args.include_videos, args.only_videos))
        .collect();

    let total_files = entries.len();
    println!("Found {} matching files.", total_files);

    if total_files == 0 {
        println!("No matching files found in the directory with the current filters.");
        return Ok(());
    }

    println!("Extracting EXIF/filesystem metadata (multi-threaded)...");
    let pb = ProgressBar::new(total_files as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "[{elapsed_precise}] [{bar:40.yellow/blue}] {pos}/{len} metadata read ({eta})",
            )?
            .progress_chars("=>-"),
    );

    let candidates: Vec<PhotoCandidate> = entries
        .par_iter()
        .filter_map(|path| {
            let res = extract_photo_timestamp(path).map(|ts| {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_string();

                PhotoCandidate {
                    original_path: path.clone(),
                    extension: ext,
                    timestamp: ts,
                }
            });
            pb.inc(1);
            res
        })
        .collect();

    pb.finish_with_message("Done reading!");

    let extracted_count = candidates.len();
    if extracted_count < total_files {
        println!(
            "[WARNING] Could not extract a date from {} files (skipped for safety).",
            total_files - extracted_count
        );
    }

    println!("Computing new names and resolving possible collisions...");
    let actions = plan_renames(candidates);

    executor::execute_plan(&actions, &target_dir, !args.execute)?;

    Ok(())
}
