# SXM Mail

Client mail de bureau pour Linux (Tauri 2 + Rust). Cible principale : Fedora, paquet `.rpm`.

## Compiler sur Fedora

```bash
# Dépendances système
sudo dnf install -y webkit2gtk4.1-devel openssl-devel gtk3-devel libappindicator-gtk3-devel \
  librsvg2-devel libxdo-devel rpm-build gcc gcc-c++ pkgconf-pkg-config curl wget file

# Rust (si absent)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Outil de build Tauri
cargo install tauri-cli --version "^2" --locked

# Mode développement (fenêtre avec rechargement)
cd sxm-mail && cargo tauri dev

# Paquet RPM
cargo tauri build
# -> src-tauri/target/release/bundle/rpm/SXM Mail-0.1.0-1.x86_64.rpm
sudo dnf install ./src-tauri/target/release/bundle/rpm/*.rpm
```

La première compilation est longue (plusieurs minutes), les suivantes sont rapides.

## Version macOS (compilée par GitHub)

Le dépôt contient `.github/workflows/macos.yml` : GitHub compile l'application sur ses propres Mac.
1. Onglet **Actions** du dépôt → **Build macOS** → **Run workflow**.
2. À la fin (≈ 15 à 25 min la première fois), ouvre l'exécution → section **Artifacts** → télécharge **SXM-Mail-macOS** (un zip contenant le `.dmg`).
3. Sur le Mac : ouvre le `.dmg`, glisse SXM Mail dans Applications.
4. Premier lancement : l'app n'est pas signée par Apple → **Réglages Système → Confidentialité et sécurité → Ouvrir quand même**.
5. Le fichier `config.json` va dans `~/Library/Application Support/sxm-mail/config.json` (même contenu que sous Linux). Le jeton est rangé dans le Trousseau d'accès.

Le binaire est « universel » : il fonctionne sur les Mac Intel et Apple Silicon.

## Connecter Gmail

Voir `docs/GOOGLE.md` (création du Client ID Google, ~10 min), puis « + Ajouter un compte » dans l'application.
Le trousseau système (GNOME Keyring / KWallet) doit être actif pour stocker le jeton.

## Connecter Office 365 / Hotmail

Voir `docs/MICROSOFT.md` (enregistrement de l'application dans Azure, ~10 min).

## Tests du cœur (sans interface)

```bash
cd core && cargo test
```

## Aperçu rapide sans Rust

Ouvre `src/index.html` dans un navigateur : l'interface fonctionne avec des données de démonstration.

## Raccourcis

`j` / `k` message suivant / précédent · `e` archiver · `s` épingler · `h` rappeler plus tard · `#` supprimer · `c` nouveau message · `/` recherche

## Icônes

Les icônes fournies sont des icônes temporaires. Pour générer le jeu complet à partir de ton propre logo (PNG carré 1024×1024) : `cargo tauri icon mon-logo.png`.

Voir `PROGRESS.md` pour l'avancement et les prochaines étapes.
