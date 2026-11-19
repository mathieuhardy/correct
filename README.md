# correct

Correcteur orthographique et grammatical de bureau, basé sur [LanguageTool](https://languagetool.org/).
Interface minimaliste à trois panneaux, thème sombre par défaut.

---

## Interface

```
┌──────────────┬──────────────────────────────┬──────────────────────┐
│ Explorer     │ notes.md              ✎       │ LanguageTool  ● [☀]  │
├──────────────┤                               ├──────────────────────┤
│ ▼ project/   │ # Réunion du 15/11            │  [▶ Analyser] [■]    │
│   ▶ docs/    │                               ├──────────────────────┤
│     notes.md │ Le dossier est tres           │ ⚠ 2 problèmes        │
│     README   │               ~~~~            │                      │
│   ▶ src/     │ importent pour nous.          │ ▸ "tres" → très      │
│     main.rs  │ ~~~~~~~~~                     │   Accent manquant    │
│              │                               │                      │
│              │                               │ ▸ "importent" →      │
│              │                               │   important          │
└──────────────┴───────────────────────────────┴──────────────────────┘
```

- **Panneau gauche** — explorateur de fichiers du projet ouvert
- **Panneau central** — contenu du fichier, erreurs soulignées
- **Panneau droit** — résultats LanguageTool (liste collapsée), boutons d'analyse
- **●** — statut de connexion LanguageTool (vert = connecté, rouge = inaccessible)
- **[☀/☾]** — bascule thème clair/sombre

Cliquer sur un soulignement ou un élément du panneau droit ouvre un popup :

```
┌──────────────────────────────┐
│ "importent"                  │
│ Suggestion : important       │
│ ──────────────────────────── │
│ [✓ Accepter] [✗ Ignorer]     │
│ [+ Ajouter au dictionnaire]  │
└──────────────────────────────┘
```

---

## Fonctionnalités

- Ouvre le dernier projet au démarrage
- Analyse orthographique et grammaticale via LanguageTool
- Soulignement des erreurs dans le texte
- Accepter une suggestion → applique le remplacement dans le buffer
- Ignorer → masque l'erreur pour la session en cours
- Ajouter au dictionnaire → persiste dans `.correct/dictionary.toml`, retire immédiatement les occurrences
- Dictionnaire local par projet (`.correct/dictionary.toml`)
- Thème sombre par défaut, bascule vers le thème clair
- Sauvegarde du fichier (Ctrl+S)

---

## Prérequis

- [LanguageTool](https://dev.languagetool.org/http-server) — binaire natif (`languagetool-server` ou équivalent)
- [Nix](https://nixos.org/) (optionnel, recommandé pour l'environnement de développement)

---

## Configuration

Fichier global : `~/.config/correct/config.toml`

```toml
[languagetool]
binary   = "/opt/languagetool/languagetool-server"  # optionnel
url      = "http://localhost:8081"
language = "auto"   # ou "fr", "en-US", etc.

[app]
last_project = ""
theme        = "dark"   # "dark" | "light"
```

Le champ `binary` est optionnel. Comportement au démarrage :

1. L'application tente de se connecter à `url`
2. Si accessible → statut vert, rien de plus
3. Si inaccessible et `binary` configuré → le binaire est lancé directement, puis connexion retentée
4. Si toujours inaccessible → statut rouge, message d'erreur

### Dictionnaire

Chaque projet possède son propre dictionnaire : `<dossier-projet>/.correct/dictionary.toml`

```toml
words = ["Qwant", "flake", "nix"]
```

Créé automatiquement à la première utilisation de « Ajouter au dictionnaire ».
Les mots correspondent sans tenir compte de la casse.

---

## Développement

```sh
nix develop    # environnement avec toolchain Rust + dépendances GUI
cargo build
cargo test
```

### Structure des sources

```
src/
  main.rs            # point d'entrée eframe
  config.rs          # lecture/écriture config.toml
  dictionary.rs      # chargement/sauvegarde dictionnaire par projet
  languagetool.rs    # client HTTP + spawn/kill du processus LT
  ui/
    mod.rs
    app.rs           # AppState, boucle principale egui
    file_tree.rs     # panneau explorateur
    editor.rs        # panneau central — texte + soulignements
    lt_panel.rs      # panneau droit — résultats + boutons
    popup.rs         # modal accepter/ignorer/dictionnaire
```

### Stack technique

| Composant | Choix |
|---|---|
| Langage | Rust 2024 |
| GUI | [egui](https://github.com/emilk/egui) 0.31 / eframe 0.31 |
| Config & dictionnaire | TOML (`serde` + `toml`) |
| HTTP | `reqwest` + `tokio` |
| Env de dev | Nix flake (`flake.nix`) |

---

## Feuille de route

### Phase 0 — Scaffolding
- [x] `cargo init`
- [ ] `flake.nix` avec devShell (Rust toolchain + dépendances GUI)
- [ ] Skeletons des modules

### Phase 1 — Config & dictionnaire
- [ ] Lecture/écriture `config.toml` (créé avec valeurs par défaut si absent)
- [ ] Chargement `<projet>/.correct/dictionary.toml` à l'ouverture du projet
- [ ] Persistance de `last_project` et `theme`

### Phase 2 — Client LanguageTool
- [ ] Logique connect-first / spawn-on-failure
- [ ] Indicateur de statut (cercle vert/rouge)
- [ ] `check(text, lang) -> Vec<LtMatch>`
- [ ] Filtrage par dictionnaire local

### Phase 3 — Squelette GUI
- [ ] Fenêtre 3 panneaux (`egui::SidePanel` + `CentralPanel`)
- [ ] Thème sombre par défaut + bascule clair/sombre
- [ ] Arbre de fichiers (panneau gauche)
- [ ] Affichage texte défilable (panneau central)
- [ ] Boutons + liste placeholder (panneau droit)

### Phase 4 — Affichage des erreurs & interactions
- [ ] Soulignements via `egui::Painter`
- [ ] Détection de clic sur les spans d'erreur
- [ ] Items collapsés dans le panneau droit, clic pour développer
- [ ] Popup modal : accepter / ignorer / ajouter au dictionnaire

### Phase 5 — Polish & persistance
- [ ] Ctrl+S sauvegarde le fichier
- [ ] Réouverture du dernier projet au démarrage
- [ ] Gestion des erreurs (LT inaccessible, fichier illisible, binaire introuvable)
- [ ] Tests unitaires (config, dictionnaire, parsing réponse LT)
