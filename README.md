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
font_hinting = true      # calage des glyphes sur les pixels (false = un peu plus doux)
font_gamma   = 1.0       # lissage du texte : 1.0 = traits fins et lisses (absent = défaut egui, plus gras)
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
- [x] Libération propre du watcher au switch/fermeture de fichier (drop)
- [x] Barre de statut en bas : icône rouge + message pour toute erreur applicative
- [x] Réouverture du dernier projet au démarrage
- [x] Tests unitaires (config, dictionnaire, parsing réponse LT)

### Phase 6 — Ignorés persistants
- [x] `LtMatch` : identifiant de règle et phrase (`sentence`) issus de la réponse LT
- [x] Empreinte d'un élément ignoré : règle + texte + phrase aux espaces normalisés (indépendante de la position)
- [x] `.correct/ignored.toml` : liste plate d'entrées `{ file, rule, text, sentence }`, chemins relatifs au projet, triée, écriture atomique, fichier invalide → erreur et écriture bloquée
- [x] « Ignorer » persiste l'empreinte ; un élément est ignoré si son empreinte figure dans la liste
- [x] Réconciliation après chaque analyse réussie et à jour : empreintes du fichier sans correspondance purgées (aucune purge sur erreur, annulation ou résultat périmé)
- [x] Rechargement si `ignored.toml` est modifié hors de l'application ; entrées des fichiers disparus retirées à l'ouverture du projet
- [x] Tests : empreinte qui survit à un déplacement / re-wrap, purge limitée au fichier analysé, fichiers disparus, liste vide → fichier supprimé, fichier invalide jamais écrasé

### Phase 7 — Config multi-analyseur

- [ ] Section `[grammalecte]` dans `config.rs` : `GrammalecteConfig { binary, url }`
- [ ] Enum `Analyzer { LanguageTool, Grammalecte }` sérialisé `"languagetool"` / `"grammalecte"`
- [ ] Champ `default_analyzer: Analyzer` dans `AppConfig` (défaut : `languagetool`)
- [ ] Mise à jour des configs d'exemple (`data/ubuntu_config.toml`, `data/nixos_config.toml`)
- [ ] Tests : round-trip TOML, valeurs par défaut, enum inconnu → erreur lisible

### Phase 8 — Client Grammalecte

- [ ] `src/grammalecte.rs` : `GcClient` + `GcChecker` (même pattern que `languagetool.rs`)
- [ ] Health check : `GET /get_options/fr` → 200 si opérationnel (spawn binaire si configuré, même logique que LT)
- [ ] `POST /gc_text/fr` (form-encoded : `text`, `options` JSON)
- [ ] Parser la réponse : fusionner `lGrammarErrors` + `lSpellingErrors` → `Vec<LtMatch>`
- [ ] Offsets `nStart`/`nEnd` char-based (pas de conversion UTF-16, simplification vs LT)
- [ ] `nEnd` exclusif → `length = nEnd - nStart`
- [ ] `sentence` absent dans la réponse → reconstitué depuis `sParagraph`
- [ ] `lSpellingErrors` → `misspelling = true`
- [ ] Tests unitaires : parsing JSON, cas emoji, spelling vs grammaire

### Phase 9 — Toggle UI et orchestration

- [ ] Toggle dans l'en-tête du panneau droit : `[LT] [Gc]` (radio-style)
- [ ] `AppState::active_analyzer: Analyzer` initialisé depuis `default_analyzer`
- [ ] Statut `●` indépendant pour chaque analyseur (LT peut être rouge pendant que GC est vert)
- [ ] Switch d'analyseur relance l'analyse du fichier ouvert

### Phase 10 — Options Grammalecte dans la config

- [ ] `[grammalecte.options]` : `HashMap<String, bool>` → passé en JSON dans les requêtes
- [ ] Valeurs par défaut sensées (voir analyse des options ci-dessous)
- [ ] Optionnel : panneau d'options dans l'UI

---

### Bugs trouvés
- [x] Crash de l'application au démarrage s'il n'y a pas de fichier de config ou bien s'il manque des clés dedans
- [x] Police pixelisée (lissage via `font_gamma`, hinting via `font_hinting`)

### Commandes

```shell
docker run --rm -p 8010:8010 erikvl87/languagetool
```

```shell
# Grammalecte (après installation du paquet CLI & Serveur)
python3 grammalecte-server.py -p 8080
```

---

## Analyse de faisabilité — Intégration de Grammalecte

### Architecture cible

Le principe reste identique à l'intégration LanguageTool : un module dédié expose un `Checker` clonable et `Send`, l'application le lance en arrière-plan et reçoit un `Vec<LtMatch>` (structure réutilisée telle quelle).

```
src/
  languagetool.rs    # client existant (inchangé)
  grammalecte.rs     # nouveau client, même pattern
  ui/
    app.rs           # AppState::active_analyzer: Analyzer
    lt_panel.rs      # toggle LT / Gc dans l'en-tête
```

**Enum d'orchestration** dans `app.rs` :

```rust
enum Analyzer {
    LanguageTool(LtChecker),
    Grammalecte(GcChecker),
}
```

Le `GcChecker` convertit la réponse Grammalecte en `Vec<LtMatch>` : les listes `lGrammarErrors` et `lSpellingErrors` sont fusionnées, les suggestions `aSuggestions: Vec<String>` correspondent directement aux `replacements`, et `sentence` est reconstitué depuis `sParagraph`.

### Config TOML cible

```toml
[languagetool]
binary   = "/opt/languagetool/languagetool-server"  # optionnel
url      = "http://localhost:8081"
language = "auto"

[grammalecte]
binary   = "python3 grammalecte-server.py"  # optionnel
url      = "http://localhost:8080"

[grammalecte.options]
# Toutes les options sont optionnelles ; absentes = défaut Grammalecte.
# Voir la section "Analyse des options" ci-dessous.
redon2 = true   # répétitions intra-phrase (désactivé par défaut dans GC)
md     = true   # ignorer la syntaxe Markdown

[app]
default_analyzer = "languagetool"   # ou "grammalecte"
```

### API Grammalecte vs LanguageTool

| Aspect | LanguageTool | Grammalecte |
|---|---|---|
| Endpoint analyse | `POST /v2/check` | `POST /gc_text/fr` |
| Paramètre langue | `language=fr` (form) | Encodé dans l'URL (`/fr`) |
| Paramètre options | `enabledRules`, `disabledRules` | `options` (JSON string) |
| Offsets | UTF-16 (conversion nécessaire) | Position de caractère Unicode direct |
| Borne de fin | `offset + length` | `nEnd` exclusif |
| Structure réponse | `matches[]` plat | `data[].lGrammarErrors` + `lSpellingErrors` |
| Suggestions | `replacements[].value` | `aSuggestions[]` (strings directs) |
| Champ `sentence` | Fourni | Absent → reconstituer depuis `sParagraph` |
| Health check | `GET /v2/languages` | `GET /get_options/fr` |
| Multilangue | Oui | Non (FR uniquement) |
| Runtime | Java / JVM | Python 3 |

**Point d'attention principal** : l'absence de `sentence` dans la réponse Grammalecte affecte le fingerprinting de `ignored.toml`. La solution retenue : utiliser `sParagraph` (fourni dans la réponse) pour reconstruire une phrase approximative par extraction autour de `nStart`/`nEnd`.

---

## Ce que Grammalecte apporte en plus de LanguageTool

1. **Typographie française de référence** — Les règles `apos` (apostrophe courbe), `nbsp` (espace insécable avant `?!;:«`), `typo` (guillemets `«»`, tirets cadratins) et `eleu` (élisions) sont le cœur de Grammalecte. LanguageTool les couvre partiellement ; Grammalecte est l'outil de référence pour la typographie française.

2. **Validité des dates** (`date`) — Détecte le 31 février, le 29 février hors année bissextile, les incohérences jour/mois. LanguageTool ne fait pas ce contrôle.

3. **Écriture épicène** (`eepi`) — Propose des formulations neutres ("les auteurs et autrices", "les élèves" plutôt que "les élèves et les élèves") pour les documents inclusifs. Absent dans LanguageTool.

4. **Pléonasmes exhaustifs** (`pleo`) — "Monter en haut", "prévoir à l'avance", "reculer en arrière". LanguageTool en détecte quelques-uns ; Grammalecte a une liste plus fournie pour le français.

5. **Répétitions** (`redon1`, `redon2`) — Détection des répétitions lexicales dans la même phrase (`redon2`) ou dans le même paragraphe (`redon1`). Non disponible dans LanguageTool pour le français.

6. **Endpoint `/format_text/fr`** — Applique directement les corrections typographiques au texte (apostrophes, espaces, guillemets) sans passer par des suggestions. Ouvre la voie à un mode "auto-correct typographique" en un clic.

7. **Options granulaires par requête** — Chaque règle peut être activée ou désactivée individuellement dans le champ `options` de chaque requête HTTP. LanguageTool utilise des listes `enabledRules`/`disabledRules` moins lisibles.

8. **Empreinte légère** — Python 3 contre une JVM ; démarrage plus rapide, consommation mémoire réduite pour un usage desktop.

---

## Analyse des options Grammalecte

Les options marquées `[!]` dans Grammalecte sont expérimentales ou à fort taux de faux positifs ; elles sont désactivées par défaut.

### Options essentielles (activées par défaut — à conserver)

| Option | Description | Pourquoi conserver |
|---|---|---|
| `apos` | Apostrophe typographique | Fondamental pour le français écrit |
| `conf` | Confusions et faux-amis (a/à, ou/où…) | Très fréquent, peu de faux positifs |
| `conj` | Conjugaisons | Essentiel |
| `eleu` | Élisions et euphonies | "le heure" → "l'heure", fondamental |
| `esp` | Espaces surnuméraires | Utile, peu de faux positifs |
| `gn` | Accords genre et nombre | Essentiel |
| `imp` | Impératif | Utile |
| `infi` | Infinitif | Utile |
| `inte` | Interrogatif | Utile |
| `loc` | Locutions | Utile |
| `maj` | Majuscules | Utile |
| `nbsp` | Espaces insécables | Fondamental pour typographie FR |
| `nf` | Normes françaises | Utile pour écriture formelle |
| `num` | Nombres | Utile |
| `pleo` | Pléonasmes | Intéressant, peu bruyant |
| `ppas` | Participes passés, adjectifs | Essentiel |
| `tu` | Traits d'union et soudures | Important |
| `typo` | Signes typographiques | Fondamental |
| `unit` | Espaces avant unités de mesure | Utile |
| `virg` | Virgules | Essentiel |
| `vmode` | Modes verbaux | Essentiel |

### Options à activer (actuellement désactivées par défaut)

| Option | Recommandation | Justification |
|---|---|---|
| `redon2` | **Activer** | Les répétitions dans la même phrase sont rarement intentionnelles ; peu de faux positifs |
| `md` | **Activer dans `correct`** | L'application édite du Markdown ; sans ce flag, la syntaxe `**`, `#`, `[]()` génère du bruit |

### Options situationnelles (désactivées par défaut, à exposer dans la config)

| Option | Quand activer |
|---|---|
| `eepi` | Documents officiels, communication RH, administration — irritant pour notes personnelles ou fiction |
| `redon1` | Textes formels — les répétitions paragraphe peuvent être stylistiques |
| `poncfin` | Écriture formelle (rapports, emails) — bruyant pour prise de notes fragmentée |
| `html` | Si le projet contient des fichiers `.html` |
| `latex` | Si le projet contient des fichiers `.tex` |
| `liga` | Documents typographiques soignés (ligatures fi, fl…) |

### Options à laisser désactivées

| Option | Raison |
|---|---|
| `idrule` | Affiche l'ID de règle dans le message — debug uniquement |
| `mapos` `[!]` | Faux positifs fréquents sur abréviations et sigles |
| `mc` `[!]` | Règles sur mots composés incomplètes, très bruyant |
| `neg` `[!]` | Négation partielle — trop de faux positifs sur style familier ou dialogues |
| `ocr` `[!]` | Uniquement pour textes numérisés (OCR) |
| `chim` | Chimie — trop spécifique, inutile pour l'usage général |
| `minis` | Majuscules des noms de ministères — très spécifique aux documents officiels français |
| `bs` | "Populaire" (expressions familières) — utile en relecture formelle, gênant pour notes ou fiction |
| `tab` | Tabulations — peu utile en Markdown (les tabulations ont un sens syntaxique) |
