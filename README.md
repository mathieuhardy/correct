# correct

Correcteur orthographique et grammatical de bureau, basé sur [LanguageTool](https://languagetool.org/).
Interface minimaliste à trois panneaux, thème sombre par défaut.

---

## Interface

```
┌──────────────┬──────────────────────────────┬──────────────────────┐
│ Explorer [⊕] │ notes.md              ✎       │ LanguageTool  ● [☀]  │
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
├──────────────┴───────────────────────────────┴──────────────────────┤
│ ● Erreur : impossible de joindre LanguageTool (http://localhost:8081)│
└──────────────────────────────────────────────────────────────────────┘
```

- **Panneau gauche** — explorateur de fichiers du projet ouvert ; **[⊕]** ouvre un nouveau projet (aussi Ctrl+O)
- **Panneau central** — un seul fichier ouvert à la fois, contenu avec erreurs soulignées
- **Panneau droit** — résultats LanguageTool (liste collapsée), boutons d'analyse
- **●** — statut de connexion LanguageTool (vert = connecté, rouge = inaccessible)
- **[☀/☾]** — bascule thème clair/sombre
- **Barre de statut** (bas) — icône rouge + message descriptif en cas d'erreur

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

- **Ouvrir un projet** via le bouton [⊕] dans le panneau gauche ou **Ctrl+O** (sélecteur de dossier) ; ce projet devient le dernier ouvert et sera restauré au prochain démarrage
- Ouvre automatiquement le dernier projet au démarrage
- **Un seul fichier ouvert à la fois** dans le panneau central ; ouvrir un autre fichier ferme le précédent sans fuite mémoire (le watcher est libéré par drop)
- Analyse orthographique et grammaticale via LanguageTool
- Soulignement des erreurs dans le texte
- Accepter une suggestion → applique le remplacement et écrit immédiatement sur disque
- Ignorer → masque l'erreur pour la session en cours
- Ajouter au dictionnaire → persiste dans `.correct/dictionary.toml`, retire immédiatement les occurrences
- Dictionnaire local par projet (`.correct/dictionary.toml`)
- **Watch filesystem** : à l'ouverture d'un fichier, un watcher est posé dessus ; toute modification externe recharge le contenu et relance automatiquement l'analyse (panel droit nettoyé avant)
- **Écriture immédiate** sur disque à chaque modification effectuée dans l'éditeur
- Watch retiré proprement à la fermeture ou au remplacement du fichier
- Thème sombre par défaut, bascule vers le thème clair
- **Barre de statut en bas** : icône rouge + message descriptif pour toute erreur (LT inaccessible, fichier illisible, écriture impossible…)

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
font_size    = 14.0
font_family  = ""        # chemin vers un fichier .ttf/.otf ou nom de famille (optionnel)
line_height  = 1.0       # multiplicateur d'interligne dans l'éditeur (1.5 = 150 %)
editor_max_width = 720   # largeur max du texte en pixels pour le mode centré (optionnel)
accent_color_bg = ""     # fond des en-têtes de panneaux en hex HTML, ex. "#5277C3" (optionnel)
accent_color_fg = ""     # texte des en-têtes de panneaux en hex HTML, ex. "#ffffff" (optionnel)
app_color_bg       = ""  # fond de l'application en hex HTML (optionnel)
app_color_fg       = ""  # texte de l'application en hex HTML (optionnel)
selection_color_bg = ""  # fond du fichier actif dans l'arbre (optionnel)
selection_color_fg = ""  # texte du fichier actif dans l'arbre (optionnel)
accept_color_bg     = "" # survol des suggestions dans le popup (défaut vert #66BB6A)
accept_color_fg     = "" # (défaut #1E1E1E)
ignore_color_bg     = "" # survol de « Ignorer » (défaut rouge #E57373)
ignore_color_fg     = "" # (défaut #1E1E1E)
dictionary_color_bg = "" # survol de « Ajouter au dictionnaire » (défaut bleu #64B5F6)
dictionary_color_fg = "" # (défaut #1E1E1E)
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
  watcher.rs         # watch filesystem sur le fichier ouvert (notify)
  ui/
    mod.rs
    app.rs           # AppState, boucle principale egui
    file_tree.rs     # panneau explorateur
    editor.rs        # panneau central — texte + soulignements
    lt_panel.rs      # panneau droit — résultats + boutons
    popup.rs         # modal accepter/ignorer/dictionnaire
    status_bar.rs    # barre de statut en bas (erreurs)
```

### Stack technique

| Composant | Choix |
|---|---|
| Langage | Rust 2024 |
| GUI | [egui](https://github.com/emilk/egui) 0.37 / eframe 0.37 |
| Config & dictionnaire | TOML (`serde` + `toml`) |
| HTTP | `reqwest` + `tokio` |
| Watch filesystem | `notify` |
| Env de dev | Nix flake (`flake.nix`) |

---

## Feuille de route

### Phase 0 — Scaffolding
- [x] `cargo init`
- [x] `flake.nix` avec devShell (Rust toolchain + dépendances GUI)
- [x] Skeletons des modules

### Phase 1 — Config & dictionnaire
- [x] Lecture/écriture `config.toml` (créé avec valeurs par défaut si absent)
- [x] Chargement `<projet>/.correct/dictionary.toml` à l'ouverture du projet
- [x] Persistance de `last_project`
- [x] `font_size` et `font_family` chargés au démarrage et appliqués à l'interface
- [x] `accent_color_bg/fg` et `app_color_bg/fg` pour le contrôle complet des couleurs

### Phase 2 — Client LanguageTool
- [x] Logique connect-first / spawn-on-failure
- [x] Indicateur de statut (cercle vert/rouge)
- [x] `check(text, lang) -> Vec<LtMatch>`
- [x] Filtrage par dictionnaire local

### Phase 3 — Squelette GUI
- [x] Fenêtre 3 panneaux (`egui::SidePanel` + `CentralPanel`)
- [x] Thème sombre par défaut (bascule supprimée — couleurs pilotées par config)
- [x] Bouton [⊕] + raccourci Ctrl+O → sélecteur de dossier (`rfd`) → ouvre le projet, persiste `last_project`
- [x] Arbre de fichiers (panneau gauche) avec expand/collapse
- [x] Éditeur texte défilable (panneau central) — `TextEdit` monospace, édition directe
- [x] Statut LanguageTool ● + bouton [▶ Analyser] (panneau droit)

### Phase 4 — Affichage des erreurs & interactions
- [x] Soulignements via `egui::Painter`
- [x] Détection de clic sur les spans d'erreur
- [x] Items collapsés dans le panneau droit, clic pour développer
- [x] Popup modal : accepter / ignorer / ajouter au dictionnaire

### Phase 4b — Dictionnaire
- [x] `dictionary.toml` illisible ou mal formé : erreur dans la barre de statut, sauvegarde bloquée pour ne pas écraser le fichier
- [x] Watch de `.correct/dictionary.toml` : rechargement et re-filtrage des résultats si modifié hors de l'application
- [x] Liste des mots du dictionnaire avec suppression depuis l'interface
- [x] « Ajouter au dictionnaire » proposé uniquement pour les fautes d'orthographe (catégorie de règle LT), pas pour les erreurs de grammaire multi-mots
- [x] Offsets LT en unités UTF-16 → conversion en index de caractères (mot extrait, filtrage et soulignement faux après un emoji)

### Phase 5 — Polish & persistance
- [x] Écriture immédiate sur disque à chaque modification (éditeur en lecture seule : à chaque suggestion acceptée)
- [x] Watch filesystem sur le fichier ouvert (`notify`) : reload + relance analyse automatique
- [ ] Libération propre du watcher au switch/fermeture de fichier (drop)
- [ ] Barre de statut en bas : icône rouge + message pour toute erreur applicative
- [ ] Réouverture du dernier projet au démarrage
- [ ] Tests unitaires (config, dictionnaire, parsing réponse LT)

### Bugs trouvés
- [ ] Crash de l'application au démarrage s'il n'y a pas de fichier de config ou bien s'il manque des clés dedans
- [ ] Police pixelisée

### Commandes

```shell
docker run --rm -p 8010:8010 erikvl87/languagetool
```
