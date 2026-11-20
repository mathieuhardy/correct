use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

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
        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
            ctx.request_repaint();
        })
        .map_err(|e| format!("Watch du dictionnaire impossible : {e}"))?;

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
                changed |= ev.paths.iter().any(|p| p.starts_with(&self.dir));
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

        // Un fichier hors de `.correct/` ne déclenche rien.
        std::thread::sleep(Duration::from_millis(100));
        w.poll();
        std::fs::write(project.join("notes.md"), "x").unwrap();
        std::thread::sleep(Duration::from_millis(200));
        assert!(!w.poll());
    }
}
