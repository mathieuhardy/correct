use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

/// Événement susceptible d'avoir changé le contenu. Les `Access` (ouverture, fermeture
/// sans écriture) sont exclus : relire le fichier en produit, ce qui bouclerait.
fn is_change(ev: &Event) -> bool {
    !matches!(ev.kind, EventKind::Access(_))
}

fn channel_watcher(ctx: egui::Context) -> Result<(RecommendedWatcher, Receiver<notify::Result<Event>>), String> {
    let (tx, rx) = channel();
    let watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
        ctx.request_repaint();
    })
    .map_err(|e| format!("Watch impossible : {e}"))?;
    Ok((watcher, rx))
}

/// Surveille le fichier ouvert dans l'éditeur.
///
/// Comme pour le dictionnaire, on observe le dossier parent (non récursif) pour survivre
/// aux remplacements par rename. Le watch est libéré par drop.
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<notify::Result<Event>>,
    path: PathBuf,
}

impl FileWatcher {
    pub fn new(path: &Path, ctx: egui::Context) -> Result<Self, String> {
        let (mut watcher, rx) = channel_watcher(ctx)?;
        let dir = path.parent().unwrap_or(Path::new("."));
        watcher
            .watch(dir, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Watch de '{}' impossible : {e}", path.display()))?;
        Ok(Self { _watcher: watcher, rx, path: path.to_path_buf() })
    }

    /// Vide les événements reçus ; true si le fichier a pu changer depuis le dernier appel.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok(res) = self.rx.try_recv() {
            if let Ok(ev) = res {
                changed |= is_change(&ev) && ev.paths.iter().any(|p| *p == self.path);
            }
        }
        changed
    }
}

/// Surveille `<projet>/.correct/dictionary.toml`.
///
/// On observe le dossier `.correct/` plutôt que le fichier : les éditeurs (et notre propre
/// `save`) remplacent le fichier par rename, ce qui ferait perdre un watch posé sur le fichier.
/// La racine du projet est aussi observée (non récursif) pour voir apparaître `.correct/`.
/// Le watch est libéré par drop.
pub struct DictWatcher {
    watcher: RecommendedWatcher,
    rx: Receiver<notify::Result<Event>>,
    dir: PathBuf,
    dir_watched: bool,
}

impl DictWatcher {
    pub fn new(project: &Path, ctx: egui::Context) -> Result<Self, String> {
        let (mut watcher, rx) = channel_watcher(ctx)?;
        watcher
            .watch(project, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Watch du dictionnaire impossible : {e}"))?;

        let mut w = Self { watcher, rx, dir: project.join(".correct"), dir_watched: false };
        w.watch_dir();
        Ok(w)
    }

    fn watch_dir(&mut self) {
        if !self.dir_watched && self.dir.is_dir() {
            self.dir_watched = self.watcher.watch(&self.dir, RecursiveMode::NonRecursive).is_ok();
        }
    }

    /// Vide les événements reçus ; true si le dictionnaire a pu changer depuis le dernier appel.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok(res) = self.rx.try_recv() {
            if let Ok(ev) = res {
                changed |= is_change(&ev) && ev.paths.iter().any(|p| p.starts_with(&self.dir));
            }
        }
        if changed {
            // `.correct/` supprimé : le watch est perdu, il sera reposé à sa recréation.
            if !self.dir.is_dir() {
                self.dir_watched = false;
            }
            self.watch_dir();
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn wait_change(w: &mut DictWatcher) -> bool {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if w.poll() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    #[test]
    fn detects_creation_then_external_edit() {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let project = std::env::temp_dir().join(format!("correct-watch-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&project).unwrap();

        let mut w = DictWatcher::new(&project, egui::Context::default()).unwrap();
        assert!(!w.poll());

        // `.correct/` n'existe pas encore au démarrage du watch.
        let dict = crate::dictionary::Dictionary::path(&project);
        std::fs::create_dir_all(dict.parent().unwrap()).unwrap();
        assert!(wait_change(&mut w));

        std::fs::write(&dict, "words = [\"nix\"]").unwrap();
        assert!(wait_change(&mut w));

        // Relire le dictionnaire (comme le fait le rechargement) ne déclenche rien.
        std::thread::sleep(Duration::from_millis(100));
        w.poll();
        std::fs::read_to_string(&dict).unwrap();
        std::thread::sleep(Duration::from_millis(200));
        assert!(!w.poll());

        // Un fichier hors de `.correct/` ne déclenche rien.
        std::thread::sleep(Duration::from_millis(100));
        w.poll();
        std::fs::write(project.join("notes.md"), "x").unwrap();
        std::thread::sleep(Duration::from_millis(200));
        assert!(!w.poll());
    }

    #[test]
    fn file_watcher_sees_edit_and_replace_but_not_reads() {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("correct-fwatch-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("notes.md");
        std::fs::write(&file, "a").unwrap();

        let mut w = FileWatcher::new(&file, egui::Context::default()).unwrap();

        std::fs::write(&file, "b").unwrap();
        assert!(wait_change_file(&mut w));

        // Remplacement atomique (tmp + rename), comme beaucoup d'éditeurs.
        let tmp = dir.join("notes.md.tmp");
        std::fs::write(&tmp, "c").unwrap();
        std::fs::rename(&tmp, &file).unwrap();
        assert!(wait_change_file(&mut w));

        // Lecture seule et fichier voisin : rien.
        std::thread::sleep(Duration::from_millis(100));
        w.poll();
        std::fs::read_to_string(&file).unwrap();
        std::fs::write(dir.join("autre.md"), "x").unwrap();
        std::thread::sleep(Duration::from_millis(200));
        assert!(!w.poll());
    }

    fn wait_change_file(w: &mut FileWatcher) -> bool {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if w.poll() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }
}
