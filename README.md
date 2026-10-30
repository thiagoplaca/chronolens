# ChronoLens

A high-performance command-line tool written in Rust for safely batch-renaming **thousands of photos and videos** with strict guarantees of **zero corruption**, **zero data loss**, and **no overwrites**.

File names are rebuilt from the real capture date (EXIF metadata first, filesystem timestamps as a fallback), turning cluttered folders full of `IMG_98712.jpg`-style names into a clean, chronologically organized library.

## Features

- **Date-based naming** — standard `DD-MM-YYYY` pattern, with time suffixes on collisions.
- **Parallel processing** — metadata extraction is multi-threaded (Rayon), with a live progress bar.
- **Safe by default** — runs in **dry-run mode** unless explicitly told to execute.
- **Full rollback** — every real run writes a JSON manifest you can use to undo all renames.
- **Never overwrites** — destination files that already exist are never touched.
- **Photo & video support** — JPEG, PNG, HEIC, RAW formats, MP4, MOV, and more.
- **Recursive mode** — optionally scan subdirectories.

## Installation

Requires [Rust](https://www.rust-lang.org/tools/install) (stable, 1.75+ recommended).

```bash
git clone https://github.com/thiagoplaca/chronolens.git
cd chronolens
cargo build --release
```

The binary will be available at `target/release/chronolens`.

## Naming Rules

Given a file's capture date, the tool applies the following rules **per day, per directory**:

| Scenario | Pattern | Example |
|---|---|---|
| Single file on that day | `DD-MM-YYYY.<ext>` | `25-10-2023.jpg` |
| Multiple files on that day | `DD-MM-YYYY-HHHMMSS.<ext>` | `25-10-2023-14H30M22S.jpg` |
| Exact second collision (bursts) | Sequential suffix `-1`, `-2`, ... | `25-10-2023-14H30M22S-1.jpg` |

- Burst shots taken in the exact same second receive a sequential suffix: `-1`, `-2`, etc.
- Files whose date cannot be determined are **skipped** (never guessed).

## Safety Guarantees

- **Zero corruption**: file contents are **never** opened for writing. Files are only read for metadata and renamed atomically via the OS (`rename`, which does not touch bytes).
- **Overwrite protection**: before renaming, the tool checks whether the destination already exists — either on disk or within the current batch. It never overwrites anything.
- **Dry-run by default**: without `--execute`, the tool only shows a preview of the planned renames; nothing on disk changes.
- **Rollback manifest**: when running with `--execute`, a `rollback_manifest_<TIMESTAMP>.json` is written **before** any rename happens. You can undo 100% of the operation at any time.

## Supported Formats

- **Photos**: `jpg`, `jpeg`, `png`, `heic`, `heif`, `webp`, `tiff`, `tif`, `raw`, `cr2`, `cr3`, `nef`, `arw`, `dng`, `bmp`
- **Videos**: `mp4`, `mov`, `m4v`, `avi`, `mkv`, `wmv`, `flv`, `webm`, `3gp`, `mts`, `m2ts`

By default, **only photos** are processed.

## Usage

```
chronolens [OPTIONS] [DIRECTORY]

Arguments:
  [DIRECTORY]  Path to the folder containing the photos/videos

Options:
  -v, --include-videos  Include video files in the process
      --only-videos     Process ONLY video files (ignores photos)
  -r, --recursive       Scan subdirectories recursively
      --execute         Apply the actual renames on disk (without this: dry-run)
      --rollback <MANIFEST_JSON>  Undo renames from a previously generated manifest
  -h, --help            Print help
  -V, --version         Print version
```

### Examples

**1. Dry-run (default) — photos only:**
```bash
./target/release/chronolens /path/to/photos
```

**2. Dry-run including videos:**
```bash
./target/release/chronolens /path/to/photos -v
```

**3. Apply real renames:**
```bash
# Photos only:
./target/release/chronolens /path/to/photos --execute

# Photos + videos:
./target/release/chronolens /path/to/photos -v --execute
```

**4. Recursive mode (include subdirectories):**
```bash
./target/release/chronolens /path/to/photos -v -r --execute
```

**5. Undo a previous run (rollback):**
```bash
./target/release/chronolens --rollback /path/to/rollback_manifest_20241025_143012.json
```

## How It Works

The pipeline runs in five stages:

1. **Scan** — walks the target directory (recursively if requested), filtering by supported extensions.
2. **Extract** — reads each file's EXIF (`DateTimeOriginal` → `DateTimeDigitized` → `DateTime`, in that order), falling back to the filesystem's modification/creation date. This stage is parallelized with Rayon.
3. **Plan** — groups files by directory and date, sorts them chronologically, and computes collision-free target names (reserving every path globally before anything is touched).
4. **Execute** — writes the rollback manifest, then performs the renames one by one, validating before each operation.
5. **Report** — prints a final summary: renamed, unchanged, and errors/blocked.

## Development

```bash
cargo build                # development build
cargo test                 # run unit tests (planner rules)
cargo run -- /some/folder  # run a dry-run on a folder
```

## License

This project is provided as-is, without warranty of any kind. See the `LICENSE` file if present.

---

**Tip:** always run once without `--execute` first, review the dry-run output, and only then apply for real. If something looks wrong, the rollback manifest has you covered.
