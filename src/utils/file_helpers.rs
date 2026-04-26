use crate::core::models::{MediaItem, MediaType, ResolvedName};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;

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

pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let al = a.to_lowercase();
    let bl = b.to_lowercase();
    let mut ar = al.as_str();
    let mut br = bl.as_str();

    loop {
        match (ar.is_empty(), br.is_empty()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }

        let a_digit = ar.starts_with(|c: char| c.is_ascii_digit());
        let b_digit = br.starts_with(|c: char| c.is_ascii_digit());

        if a_digit && b_digit {
            let an = ar.find(|c: char| !c.is_ascii_digit()).unwrap_or(ar.len());
            let bn = br.find(|c: char| !c.is_ascii_digit()).unwrap_or(br.len());
            let av: u64 = ar[..an].parse().unwrap_or(0);
            let bv: u64 = br[..bn].parse().unwrap_or(0);
            match av.cmp(&bv) {
                Ordering::Equal => {
                    ar = &ar[an..];
                    br = &br[bn..];
                }
                other => return other,
            }
        } else {
            let ac = ar.chars().next().unwrap();
            let bc = br.chars().next().unwrap();
            match ac.cmp(&bc) {
                Ordering::Equal => {
                    ar = &ar[ac.len_utf8()..];
                    br = &br[bc.len_utf8()..];
                }
                other => return other,
            }
        }
    }
}

pub fn parse_suffix_number(stem: &str) -> Option<(String, u32)> {
    const SEP: &str = " - ";

    let sep_pos = stem.rfind(SEP)?;
    let num_str = &stem[sep_pos + SEP.len()..];
    let n: u32 = num_str.parse().ok()?;
    if n == 0 {
        return None;
    }
    Some((stem[..sep_pos].to_owned(), n))
}

pub fn reindex_after_delete(deleted_path: &Path) -> Vec<(String, String, String)> {
    let stem = match deleted_path.file_stem().and_then(|s| s.to_str()) {
        Some(s) => s.to_owned(),
        None => return Vec::new(),
    };
    let ext = match deleted_path.extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_lowercase(),
        None => return Vec::new(),
    };
    let dir = match deleted_path.parent() {
        Some(d) => d,
        None => return Vec::new(),
    };

    let (base_stem, deleted_n) = match parse_suffix_number(&stem) {
        Some(t) => t,
        None => return Vec::new(),
    };

    let mut renames: Vec<(String, String, String)> = Vec::new();
    let mut n = deleted_n + 1;

    loop {
        let old_name = format!("{base_stem} - {n}.{ext}");
        let old_path = dir.join(&old_name);

        if !old_path.exists() {
            break;
        }

        let new_n = n - 1;
        let new_name = format!("{base_stem} - {new_n}.{ext}");
        let new_path = dir.join(&new_name);

        match fs::rename(&old_path, &new_path) {
            Ok(()) => {
                renames.push((
                    old_path.to_string_lossy().into_owned(),
                    new_path.to_string_lossy().into_owned(),
                    new_name,
                ));
            }
            Err(e) => {
                eprintln!(
                    "[reindex] rename '{old_name}' → '{new_n}.{ext}' failed: {e}; \
                     stopping reindex to avoid inconsistent state"
                );
                return renames;
            }
        }

        n += 1;
    }

    let suffix1_path = dir.join(format!("{base_stem} - 1.{ext}"));
    let suffix2_path = dir.join(format!("{base_stem} - 2.{ext}"));
    let base_path = dir.join(format!("{base_stem}.{ext}"));

    if suffix1_path.exists() && !suffix2_path.exists() && !base_path.exists() {
        let old_str = suffix1_path.to_string_lossy().into_owned();
        let new_str = base_path.to_string_lossy().into_owned();
        let new_name = format!("{base_stem}.{ext}");

        match fs::rename(&suffix1_path, &base_path) {
            Ok(()) => {
                let merged = renames
                    .last_mut()
                    .filter(|last| last.1 == old_str)
                    .map(|last| {
                        last.1 = new_str.clone();
                        last.2 = new_name.clone();
                    })
                    .is_some();

                if !merged {
                    renames.push((old_str, new_str, new_name));
                }
            }
            Err(e) => {
                eprintln!(
                    "[reindex] rename '{base_stem} - 1.{ext}' → '{base_stem}.{ext}' failed: {e}"
                );
            }
        }
    }

    renames
}

pub fn find_suffix_group(item: &MediaItem, all_items: &[Arc<MediaItem>]) -> Vec<Arc<MediaItem>> {
    let path = Path::new(&item.path);
    let dir = match path.parent() {
        Some(d) => d,
        None => return Vec::new(),
    };
    let stem = match path.file_stem().and_then(|s| s.to_str()) {
        Some(s) => s,
        None => return Vec::new(),
    };
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    let base_lower = if let Some((base, _)) = parse_suffix_number(stem) {
        base.to_lowercase()
    } else {
        stem.to_lowercase()
    };

    let mut group: Vec<Arc<MediaItem>> = all_items
        .iter()
        .filter(|i| {
            let p = Path::new(&i.path);
            let d = match p.parent() {
                Some(x) => x,
                None => return false,
            };
            if d != dir {
                return false;
            }
            let e = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();
            if e != ext {
                return false;
            }
            let s = match p.file_stem().and_then(|s| s.to_str()) {
                Some(x) => x,
                None => return false,
            };
            let b = if let Some((base, _)) = parse_suffix_number(s) {
                base.to_lowercase()
            } else {
                s.to_lowercase()
            };
            b == base_lower
        })
        .cloned()
        .collect();

    group.sort_by_key(|i| {
        let s = Path::new(&i.path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        parse_suffix_number(s).map(|(_, n)| n).unwrap_or(0)
    });

    if group.len() < 2 { Vec::new() } else { group }
}

pub fn apply_group_reorder(
    new_order: &[Arc<MediaItem>],
    base_stem: &str,
    ext: &str,
    dir: &Path,
) -> Result<Vec<(String, String, String, String)>, String> {
    if new_order.is_empty() {
        return Ok(Vec::new());
    }

    let target_names: Vec<String> = (0..new_order.len())
        .map(|i| {
            if new_order.len() == 1 {
                format!("{base_stem}.{ext}")
            } else {
                format!("{base_stem} - {}.{ext}", i + 1)
            }
        })
        .collect();

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let mut phase2: Vec<(String, String, String)> = Vec::new();

    for (i, item) in new_order.iter().enumerate() {
        let target = &target_names[i];
        let current_filename = Path::new(&item.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if current_filename.eq_ignore_ascii_case(target) {
            continue;
        }

        let temp_name = format!("__nexa_ro_{ts}_{i}.{ext}");
        let temp_path = dir.join(&temp_name);

        fs::rename(Path::new(&item.path), &temp_path)
            .map_err(|e| format!("Could not stage '{current_filename}' for reorder: {e}"))?;

        phase2.push((
            item.path.clone(),
            temp_path.to_string_lossy().into_owned(),
            target.clone(),
        ));
    }

    let mut result: Vec<(String, String, String, String)> = Vec::with_capacity(phase2.len());

    for (orig_path, temp_path_str, target_name) in phase2 {
        let final_path = dir.join(&target_name);

        fs::rename(Path::new(&temp_path_str), &final_path)
            .map_err(|e| format!("Could not rename to '{target_name}': {e}"))?;

        result.push((
            orig_path,
            temp_path_str,
            final_path.to_string_lossy().into_owned(),
            target_name,
        ));
    }

    Ok(result)
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
