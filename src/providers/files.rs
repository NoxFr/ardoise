//! Fichiers JSONL des fournisseurs : parcours des dossiers et cache des fichiers déjà analysés.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

/// Version d'un fichier : tant qu'elle ne change pas, inutile de le relire.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Stamp {
    pub modified: Option<SystemTime>,
    len: u64,
}

/// Fichiers `.jsonl` sous `roots`, récursivement, modifiés depuis `since`.
pub fn jsonl_files(roots: &[PathBuf], since: SystemTime) -> Vec<(PathBuf, Stamp)> {
    let mut out = Vec::new();
    for root in roots {
        walk(root, since, &mut out);
    }
    out
}

fn walk(dir: &Path, since: SystemTime, out: &mut Vec<(PathBuf, Stamp)>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let path = e.path();
        let Ok(meta) = e.metadata() else { continue };
        if meta.is_dir() {
            walk(&path, since, out);
        } else if path.extension().is_some_and(|x| x == "jsonl") {
            let modified = meta.modified().ok();
            if modified.is_none_or(|m| m >= since) {
                out.push((path, Stamp { modified, len: meta.len() }));
            }
        }
    }
}

/// Résultat d'analyse par fichier, réutilisé tant que le fichier ne change pas : un
/// rafraîchissement ne relit que les transcripts nouveaux ou modifiés.
pub struct FileCache<T> {
    files: Mutex<HashMap<PathBuf, (Stamp, Arc<T>)>>,
}

impl<T> Default for FileCache<T> {
    fn default() -> Self {
        FileCache { files: Mutex::default() }
    }
}

impl<T> FileCache<T> {
    /// Analyse de chaque fichier de `files`, dans l'ordre ; oublie ceux qui n'y sont plus.
    pub fn refresh(&self, files: &[(PathBuf, Stamp)], parse: impl Fn(&Path) -> T) -> Vec<Arc<T>> {
        let mut cache = self.files.lock().unwrap_or_else(PoisonError::into_inner);
        let mut kept = HashMap::with_capacity(files.len());
        let parsed = files
            .iter()
            .map(|(path, stamp)| {
                let parsed = match cache.remove(path) {
                    Some((s, parsed)) if s == *stamp => parsed,
                    _ => Arc::new(parse(path)),
                };
                kept.insert(path.clone(), (*stamp, parsed.clone()));
                parsed
            })
            .collect();
        *cache = kept;
        parsed
    }

    /// Analyse d'un seul fichier, sans oublier les autres.
    pub fn get(&self, path: &Path, stamp: Stamp, parse: impl FnOnce(&Path) -> T) -> Arc<T> {
        let mut cache = self.files.lock().unwrap_or_else(PoisonError::into_inner);
        match cache.get(path) {
            Some((s, parsed)) if *s == stamp => parsed.clone(),
            _ => {
                let parsed = Arc::new(parse(path));
                cache.insert(path.to_path_buf(), (stamp, parsed.clone()));
                parsed
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;

    fn write(dir: &TempDir, rel: &str, content: &str) {
        let path = dir.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn lists_jsonl_files_recursively() {
        let dir = TempDir::new().unwrap();
        write(&dir, "a.jsonl", "");
        write(&dir, "sub/deep/b.jsonl", "");
        write(&dir, "notes.txt", "");
        let mut names: Vec<_> =
            jsonl_files(&[dir.path().to_path_buf(), dir.path().join("absent")], SystemTime::UNIX_EPOCH)
                .into_iter()
                .map(|(p, _)| p.file_name().unwrap().to_string_lossy().into_owned())
                .collect();
        names.sort();
        assert_eq!(names, ["a.jsonl", "b.jsonl"]);
    }

    #[test]
    fn reparses_only_changed_files_and_forgets_removed_ones() {
        let dir = TempDir::new().unwrap();
        write(&dir, "a.jsonl", "1");
        write(&dir, "b.jsonl", "1");
        let cache = FileCache::default();
        let parses = AtomicUsize::new(0);
        let parse = |p: &Path| {
            parses.fetch_add(1, Ordering::SeqCst);
            fs::read_to_string(p).unwrap()
        };
        let files = || {
            let mut f = jsonl_files(&[dir.path().to_path_buf()], SystemTime::UNIX_EPOCH);
            f.sort_by(|a, b| a.0.cmp(&b.0));
            f
        };

        cache.refresh(&files(), parse);
        cache.refresh(&files(), parse);
        assert_eq!(parses.load(Ordering::SeqCst), 2, "rien n'a changé");

        write(&dir, "b.jsonl", "12");
        let parsed = cache.refresh(&files(), parse);
        assert_eq!(parses.load(Ordering::SeqCst), 3);
        assert_eq!(*parsed[1], "12");

        fs::remove_file(dir.path().join("a.jsonl")).unwrap();
        cache.refresh(&files(), parse);
        assert_eq!(cache.files.lock().unwrap().len(), 1);
    }
}
