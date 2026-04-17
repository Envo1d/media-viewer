use crate::core::models::{MediaType, ResolvedName};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

fn make_filename(stem: &str, ext: &str) -> String {
    if ext.is_empty() {
        stem.to_owned()
    } else {
        format!("{stem}.{ext}")
    }
}

fn dir_stems(dir: &Path) -> HashSet<String> {
    let Ok(rd) = fs::read_dir(dir) else {
        return HashSet::new();
    };
    rd.flatten()
        .filter_map(|e| {
            let p = e.path();
            if p.is_file() {
                p.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_lowercase())
            } else {
                None
            }
        })
        .collect()
}

fn find_file_by_stem(dir: &Path, stem: &str) -> Option<String> {
    let target = stem.to_lowercase();
    let Ok(rd) = fs::read_dir(dir) else {
        return None;
    };
    rd.flatten().find_map(|e| {
        let p = e.path();
        if p.is_file() {
            let s = p.file_stem()?.to_str()?.to_lowercase();
            if s == target {
                return p.file_name().map(|n| n.to_string_lossy().into_owned());
            }
        }
        None
    })
}

pub fn build_filename_stem(
    media_type: &MediaType,
    characters: &[String],
    artist: &str,
    video_title: &str,
    src_path: &Path,
    char_sep: &str,
) -> String {
    let char_part = if characters.is_empty() {
        String::new()
    } else {
        characters.join(char_sep)
    };
    let artist_part = if artist.is_empty() {
        String::new()
    } else {
        format!("[{artist}]")
    };

    match media_type {
        MediaType::Image => {
            if char_part.is_empty() {
                src_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            } else if artist_part.is_empty() {
                char_part
            } else {
                format!("{char_part} {artist_part}")
            }
        }
        MediaType::Video => {
            let title = if video_title.is_empty() {
                src_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            } else {
                sanitize_filename(video_title)
            };
            let middle = if char_part.is_empty() {
                title
            } else {
                format!("{char_part} - {title}")
            };
            if artist_part.is_empty() {
                middle
            } else {
                format!("{middle} {artist_part}")
            }
        }
    }
}

pub fn resolve_conflict(dir: &Path, stem: &str, ext: &str) -> ResolvedName {
    let stems = dir_stems(dir);
    let key = stem.to_lowercase();
    let suffix1 = format!("{stem} - 1").to_lowercase();

    if stems.contains(&key) && !stems.contains(&suffix1) {
        let existing_filename =
            find_file_by_stem(dir, stem).unwrap_or_else(|| make_filename(stem, ext));
        let existing_ext = Path::new(&existing_filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        return ResolvedName::RenameExisting {
            existing_old: existing_filename,
            existing_new: make_filename(&format!("{stem} - 1"), &existing_ext),
            new_file: make_filename(&format!("{stem} - 2"), ext),
        };
    }

    if stems.contains(&key) || stems.contains(&suffix1) {
        for n in 2u32.. {
            if !stems.contains(&format!("{stem} - {n}").to_lowercase()) {
                return ResolvedName::NextSuffix(make_filename(&format!("{stem} - {n}"), ext));
            }
        }
    }

    ResolvedName::Free(make_filename(stem, ext))
}

pub fn move_file(src: &Path, dst: &Path) -> std::io::Result<()> {
    match fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(src, dst)?;
            fs::remove_file(src)?;
            Ok(())
        }
    }
}

pub fn reveal_in_explorer(path: &str) {
    let _ = std::process::Command::new("explorer")
        .args(["/select,", path])
        .spawn();
}
