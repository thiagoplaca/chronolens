use crate::extractor::PhotoTimestamp;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameAction {
    pub original_path: PathBuf,
    pub new_path: PathBuf,
    pub original_name: String,
    pub new_name: String,
    pub date_str: String,
    pub has_time_suffix: bool,
}

pub struct PhotoCandidate {
    pub original_path: PathBuf,
    pub extension: String,
    pub timestamp: PhotoTimestamp,
}

/// Plans the new names for all photos safely and without collisions.
pub fn plan_renames(photos: Vec<PhotoCandidate>) -> Vec<RenameAction> {
    let mut groups: HashMap<(PathBuf, String), Vec<PhotoCandidate>> = HashMap::new();

    for photo in photos {
        let parent = photo
            .original_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let date_str = photo.timestamp.date_str.clone();
        groups.entry((parent, date_str)).or_default().push(photo);
    }

    let mut actions = Vec::new();
    let mut globally_reserved_names: HashSet<PathBuf> = HashSet::new();

    for ((parent_dir, date_str), mut group_photos) in groups {
        group_photos.sort_by(|a, b| {
            a.timestamp
                .raw_datetime
                .cmp(&b.timestamp.raw_datetime)
                .then_with(|| a.original_path.cmp(&b.original_path))
        });

        let total_in_day = group_photos.len();

        if total_in_day == 1 {
            let photo = &group_photos[0];
            let ext = &photo.extension;
            let base_name = if ext.is_empty() {
                date_str.clone()
            } else {
                format!("{}.{}", date_str, ext)
            };

            let target_path = resolve_unique_path(
                &parent_dir,
                &base_name,
                &photo.original_path,
                &mut globally_reserved_names,
            );

            actions.push(RenameAction {
                original_path: photo.original_path.clone(),
                new_path: target_path.clone(),
                original_name: photo
                    .original_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                new_name: target_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                date_str: date_str.clone(),
                has_time_suffix: false,
            });
        } else {
            for photo in group_photos {
                let ext = &photo.extension;
                let time_suffix = &photo.timestamp.time_str;
                let base_name = if ext.is_empty() {
                    format!("{}-{}", date_str, time_suffix)
                } else {
                    format!("{}-{}.{}", date_str, time_suffix, ext)
                };

                let target_path = resolve_unique_path(
                    &parent_dir,
                    &base_name,
                    &photo.original_path,
                    &mut globally_reserved_names,
                );

                actions.push(RenameAction {
                    original_path: photo.original_path.clone(),
                    new_path: target_path.clone(),
                    original_name: photo
                        .original_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    new_name: target_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    date_str: date_str.clone(),
                    has_time_suffix: true,
                });
            }
        }
    }

    actions
}

fn resolve_unique_path(
    parent: &Path,
    ideal_file_name: &str,
    current_original_path: &Path,
    reserved: &mut HashSet<PathBuf>,
) -> PathBuf {
    let mut candidate = parent.join(ideal_file_name);

    if !reserved.contains(&candidate) && (!candidate.exists() || candidate == current_original_path)
    {
        reserved.insert(candidate.clone());
        return candidate;
    }

    let (stem, ext) = split_stem_and_ext(ideal_file_name);
    let mut counter = 1;

    loop {
        let suffixed_name = if ext.is_empty() {
            format!("{}-{}", stem, counter)
        } else {
            format!("{}-{}.{}", stem, counter, ext)
        };

        candidate = parent.join(suffixed_name);

        if !reserved.contains(&candidate)
            && (!candidate.exists() || candidate == current_original_path)
        {
            reserved.insert(candidate.clone());
            return candidate;
        }

        counter += 1;
    }
}

fn split_stem_and_ext(filename: &str) -> (&str, &str) {
    match filename.rfind('.') {
        Some(pos) if pos > 0 => (&filename[..pos], &filename[pos + 1..]),
        _ => (filename, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_single_photo_in_day() {
        let date = NaiveDate::from_ymd_opt(2023, 10, 25).unwrap();
        let dt = date.and_hms_opt(10, 0, 0).unwrap();

        let photos = vec![PhotoCandidate {
            original_path: PathBuf::from("/test/img_98712.jpg"),
            extension: "jpg".to_string(),
            timestamp: PhotoTimestamp {
                date_str: "25-10-2023".to_string(),
                time_str: "10H00M00S".to_string(),
                raw_datetime: Some(dt),
            },
        }];

        let actions = plan_renames(photos);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].new_name, "25-10-2023.jpg");
        assert!(!actions[0].has_time_suffix);
    }

    #[test]
    fn test_multiple_photos_in_same_day() {
        let date = NaiveDate::from_ymd_opt(2023, 10, 25).unwrap();
        let dt1 = date.and_hms_opt(14, 30, 10).unwrap();
        let dt2 = date.and_hms_opt(18, 15, 0).unwrap();

        let photos = vec![
            PhotoCandidate {
                original_path: PathBuf::from("/test/random_a.jpg"),
                extension: "jpg".to_string(),
                timestamp: PhotoTimestamp {
                    date_str: "25-10-2023".to_string(),
                    time_str: "14H30M10S".to_string(),
                    raw_datetime: Some(dt1),
                },
            },
            PhotoCandidate {
                original_path: PathBuf::from("/test/random_b.jpg"),
                extension: "jpg".to_string(),
                timestamp: PhotoTimestamp {
                    date_str: "25-10-2023".to_string(),
                    time_str: "18H15M00S".to_string(),
                    raw_datetime: Some(dt2),
                },
            },
        ];

        let actions = plan_renames(photos);
        assert_eq!(actions.len(), 2);
        let names: Vec<String> = actions.into_iter().map(|a| a.new_name).collect();
        assert!(names.contains(&"25-10-2023-14H30M10S.jpg".to_string()));
        assert!(names.contains(&"25-10-2023-18H15M00S.jpg".to_string()));
    }

    #[test]
    fn test_collision_same_second() {
        let date = NaiveDate::from_ymd_opt(2023, 10, 25).unwrap();
        let dt = date.and_hms_opt(14, 30, 0).unwrap();

        let photos = vec![
            PhotoCandidate {
                original_path: PathBuf::from("/test/burst_1.jpg"),
                extension: "jpg".to_string(),
                timestamp: PhotoTimestamp {
                    date_str: "25-10-2023".to_string(),
                    time_str: "14H30M00S".to_string(),
                    raw_datetime: Some(dt),
                },
            },
            PhotoCandidate {
                original_path: PathBuf::from("/test/burst_2.jpg"),
                extension: "jpg".to_string(),
                timestamp: PhotoTimestamp {
                    date_str: "25-10-2023".to_string(),
                    time_str: "14H30M00S".to_string(),
                    raw_datetime: Some(dt),
                },
            },
        ];

        let actions = plan_renames(photos);
        assert_eq!(actions.len(), 2);
        let names: Vec<String> = actions.into_iter().map(|a| a.new_name).collect();
        assert!(names.contains(&"25-10-2023-14H30M00S.jpg".to_string()));
        assert!(names.contains(&"25-10-2023-14H30M00S-1.jpg".to_string()));
    }
}
