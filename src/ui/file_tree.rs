use std::path::{Path, PathBuf};

pub struct FileTree {
    nodes: Vec<TreeNode>,
}

struct TreeNode {
    name: String,
    path: PathBuf,
    is_dir: bool,
    children: Vec<TreeNode>,
}

impl FileTree {
    pub fn empty() -> Self {
        Self { nodes: vec![] }
    }

    pub fn load(root: &Path) -> Self {
        Self { nodes: build_children(root) }
    }

    /// Affiche l'arbre et retourne le chemin du fichier cliqué, le cas échéant.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        open_path: Option<&Path>,
        selection_bg: Option<egui::Color32>,
        selection_fg: Option<egui::Color32>,
    ) -> Option<PathBuf> {
        let mut clicked = None;
        for node in &mut self.nodes {
            if let Some(p) = show_node(ui, node, open_path, selection_bg, selection_fg) {
                clicked = Some(p);
            }
        }
        clicked
    }
}

fn build_children(dir: &Path) -> Vec<TreeNode> {
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
    let mut nodes: Vec<TreeNode> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                return None;
            }
            let is_dir = path.is_dir();
            let children = if is_dir { build_children(&path) } else { vec![] };
            Some(TreeNode { name, path, is_dir, children })
        })
        .collect();
    // Dossiers en premier, puis fichiers, alphabétiquement dans chaque groupe.
    nodes.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    nodes
}

fn show_node(
    ui: &mut egui::Ui,
    node: &mut TreeNode,
    open_path: Option<&Path>,
    selection_bg: Option<egui::Color32>,
    selection_fg: Option<egui::Color32>,
) -> Option<PathBuf> {
    if node.is_dir {
        let mut clicked = None;
        egui::CollapsingHeader::new(&node.name)
            .default_open(false)
            .show(ui, |ui| {
                for child in &mut node.children {
                    if let Some(p) = show_node(ui, child, open_path, selection_bg, selection_fg) {
                        clicked = Some(p);
                    }
                }
            });
        clicked
    } else {
        let selected = open_path.map(|p| p == node.path).unwrap_or(false);
        let clicked = ui.scope(|ui| {
            if selected {
                if let Some(bg) = selection_bg {
                    ui.visuals_mut().selection.bg_fill = bg;
                }
                if let Some(fg) = selection_fg {
                    ui.visuals_mut().override_text_color = Some(fg);
                }
            }
            ui.selectable_label(selected, &node.name).clicked()
        }).inner;
        if clicked { Some(node.path.clone()) } else { None }
    }
}
