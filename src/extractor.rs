use chrono::{DateTime, Local, NaiveDateTime};
use exif::{In, Reader, Tag, Value};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotoTimestamp {
    pub date_str: String,
    pub time_str: String,
    pub raw_datetime: Option<NaiveDateTime>,
}

/// Extracts the photo's date/time, preferring EXIF metadata and falling back to OS filesystem metadata when needed.
pub fn extract_photo_timestamp(path: &Path) -> Option<PhotoTimestamp> {
    if let Some(ts) = read_exif_timestamp(path) {
        return Some(ts);
    }

    read_fs_timestamp(path)
}

fn read_exif_timestamp(path: &Path) -> Option<PhotoTimestamp> {
    let file = File::open(path).ok()?;
    let mut bufreader = BufReader::new(file);
    let exif = Reader::new().read_from_container(&mut bufreader).ok()?;

    let target_tags = [Tag::DateTimeOriginal, Tag::DateTimeDigitized, Tag::DateTime];

    for tag in target_tags {
        if let Some(field) = exif.get_field(tag, In::PRIMARY) {
            let str_val = match &field.value {
                Value::Ascii(ref vec) if !vec.is_empty() => std::str::from_utf8(&vec[0]).ok(),
                _ => None,
            };

            let date_str = str_val.map(|s| s.trim());

            if let Some(raw) = date_str {
                if let Some(ts) = parse_exif_datetime_string(raw) {
                    return Some(ts);
                }
            }

            let disp = field.display_value().to_string();
            if let Some(ts) = parse_exif_datetime_string(&disp) {
                return Some(ts);
            }
        }
    }

    None
}

/// Parses strings such as "2023:08:15 14:30:22" or "2023-08-15 14:30:22"
fn parse_exif_datetime_string(s: &str) -> Option<PhotoTimestamp> {
    let cleaned = s.trim().trim_matches('"');
    let formats = [
        "%Y:%m:%d %H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%Y:%m:%d %H:%M",
        "%Y-%m-%d %H:%M",
    ];

    for fmt in formats {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(cleaned, fmt) {
            return Some(PhotoTimestamp {
                date_str: ndt.format("%d-%m-%Y").to_string(),
                time_str: ndt.format("%HH%MM%SS").to_string(),
                raw_datetime: Some(ndt),
            });
        }
    }

    None
}

fn read_fs_timestamp(path: &Path) -> Option<PhotoTimestamp> {
    let meta = path.metadata().ok()?;
    let sys_time = meta.modified().or_else(|_| meta.created()).ok()?;
    let dt: DateTime<Local> = sys_time.into();
    let ndt = dt.naive_local();

    Some(PhotoTimestamp {
        date_str: ndt.format("%d-%m-%Y").to_string(),
        time_str: ndt.format("%HH%MM%SS").to_string(),
        raw_datetime: Some(ndt),
    })
}
