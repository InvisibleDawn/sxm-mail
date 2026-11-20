# SXM Mail — avancement

Client mail de bureau (Tauri 2 + Rust), cible : Fedora (MATE), paquet `.rpm`.
Compte visé : **Gmail uniquement** (décision du 2026-11-02, voir Décisions). Tout est local, aucun serveur.

## Règle de travail
Ne rien compiler sans le feu vert explicite de Laurent.
Le code réseau est dans `core/` (crate `sxm-core`, sans Tauri) pour pouvoir être testé sans WebKitGTK.
Livraison : archive zip à décompresser par-dessus `~/Downloads/sxm-mail` (`unzip -o`), puis `cargo tauri dev`.

## Test réel sur le Fedora de Laurent (2026-11-04)
- OK : `cargo test` du core, `cargo tauri dev` (compilation Tauri complète), connexion Gmail OAuth, liste de l'Inbox, ouverture d'un mail.
- Bug corrigé : `tauri.conf.json` avait `license` sous `bundle.linux.rpm` (refusé par Tauri 2) → déplacé dans `bundle.license`.
- Piège rencontré : à la 1re connexion, Google n'a accordé que l'e-mail (accès Gmail non accordé) → IMAP « Invalid credentials ». Résolu en retirant le compte et en le ré-ajoutant (Google redemande l'accès Gmail). Amélioration possible : vérifier le scope accordé et afficher un message clair.
- OK aussi : `cargo tauri build` → .rpm de 3 Mo installé (après 2 corrections de tauri.conf.json : licence, catégorie « Email » → « Productivity »).
- Pas encore testé : envoi, réponse, affichage HTML. Retour de Laurent : lenteur (à préciser : lancement et/ou ouverture d'un mail) → pistes : connexion IMAP persistante, ne télécharger que le texte, cache.
- v0.3.8 (2026-11-20) : notifications du bureau (tauri-plugin-notification, commande `notify`) pour les nouveaux mails Prioritaire non lus, fenêtre non active ; bouton corbeille au survol de chaque mail de la liste (déplacement sans ouverture, sans passer au suivant ; suppression définitive avec confirmation dans corbeille/indésirables).
- v0.3.7 (2026-11-20) : logo officiel = mascotte facteur du visuel de Laurent (« SXM Mail — Your friendly mail app », `design/visuel-sxm-mail.png`), écusson découpé (`design/mascotte-facteur.png`) sur fond bleu ciel arrondi. Bas de l'écusson coupé à plat (texte du visuel) : à refaire si Laurent fournit la mascotte seule en PNG transparent.
- v0.3.6 (2026-11-20) : logo définitif = mascotte SXM (manchot des visuels SXM Linux / SXM Forge) en facteur avec une lettre, fond bleu ciel (`design/logo-sxm-mail.svg`, aussi `src/logo.svg` pour l'en-tête). Pose « SXM Forge » (lettre levée + pouce) écartée pour l'icône : moins lisible en petit ; gardée comme idée d'illustration pour la page du site.
- v0.3.5 (2026-11-20) : police Inter intégrée (`src/fonts`, woff2 variable latin + latin-ext, licence OFL) — sur le Fedora 45 réinstallé de Laurent, Inter absente → police de remplacement jugée moins jolie. Fedora 45 réinstallé : `~/.config/sxm-mail/config.json` à recréer (identifiants Google).
- v0.3.4 (2026-11-20) : Fedora 45 = OpenSSL 4 (libssl.so.4) → la 0.3.1–0.3.3 ne démarrait pas (libssl.so.3 manquant). Correctif : `openssl` « vendored » (lié en statique, Linux) ; scripts rpm post-install/post-remove (gtk-update-icon-cache, update-desktop-database). Contournement immédiat : `sudo dnf install openssl3-libs`.
- v0.3.3 (2026-11-19) : logo provisoire « boîte aux lettres, drapeau rouge, fond bleu ciel » (source `design/logo-sxm-mail.svg`, icônes PNG/ICNS/ICO générées) — Laurent veut une version plus stylée plus tard ; pistes écartées : enveloppe en V (trop Gmail/Spike), fond violet.
- v0.3.2 (2026-11-19) : éditeur `contenteditable` (texte brut forcé au collage/dépôt, gras/italique bloqués) ; images collées → HTML `multipart/alternative` (texte + `multipart/related` avec images `cid:`), sinon texte seul comme avant ; copie depuis un mail reçu en texte brut.
- v0.3.1 (2026-11-19) : pièces jointes à l'envoi (sélecteur de fichiers → base64, ou glisser-déposer Tauri → chemin lu côté Rust ; multipart/mixed via lettre ; 25 Mo max) ; bouton mises à jour dans la barre du haut (pastille si nouvelle version).
- v0.3.0 (2026-11-19) — compilé par GitHub, pas encore testé en réel : couleurs vives façon SXM PDF (tuiles colorées par dossier/action, dégradés, avatars variés) ; dossiers Gmail Archives (recherche `X-GM-RAW -in:inbox`), Courrier indésirable (\\Junk), Corbeille (\\Trash) chargés au 1er clic ; « Mes dossiers » = libellés Gmail (LIST, CREATE, noms en UTF-7 modifié) ; bouton Classer (V) + règle « Toujours classer les mails de X ici » appliquée à chaque actualisation (déplacement côté Gmail) ; Indésirable (!), Pas un spam, Restaurer (Z), Supprimer définitivement (seulement depuis corbeille/indésirables, confirmation), Marquer non lu. Mises à jour intégrées (`core/src/update.rs`) : API Releases, SHA256 obligatoire, Fedora `pkexec dnf install`, macOS remplacement du .app depuis le .dmg (hdiutil + ditto) puis redémarrage. Workflow unique `.github/workflows/build.yml` : .dmg + .rpm + publication avec notes tirées de `CHANGELOG.md`.
- v0.1.2 (v8) : style inspiré d'Outlook (liste en cartes espacées, lecture dans une carte).
- v0.1.3 (v9) : dossier Envoyés relié à Gmail (dossier trouvé par l'attribut \\Sent, chargé au 1er clic, 40 derniers), dossier Archives retiré, contenu HTML forcé à gauche + plus aéré, tables larges limitées à la largeur. Envoi réel : pas encore confirmé par Laurent.
- v0.2.1 : dépôt public github.com/InvisibleDawn/sxm-mail ; compilation macOS GitHub Actions **réussie** le 2026-11-07 (runner `macos-latest`, `macos-14` retiré par GitHub ; se relance à chaque push sur main). App Mac pas encore testée sur le MacBook. Version macOS préparée — trousseau macOS (keyring apple-native, dépendance par système), ouverture du navigateur via `open` sur Mac, workflow GitHub Actions `.github/workflows/macos.yml` (remplacé en 0.3.0 par `build.yml`). Dépôt GitHub de Laurent : compte `InvisibleDawn`. Config Mac : `~/Library/Application Support/sxm-mail/config.json`.
- v0.2.0 (v12) — non testé en réel, `src-tauri` non compilé ici :
  - Cache local `~/.cache/sxm-mail/messages.json` (droits 600, 1500 messages max, sans corps) : liste affichée tout de suite, puis mise à jour ; messages disparus de Gmail retirés (dans la fenêtre des 60 derniers).
  - Liens cliquables : iframe `sandbox="allow-same-origin"` sans scripts, clics interceptés par l'appli → `open_url` (http/https seulement, xdg-open sans shell) ; mailto → fenêtre de rédaction ; vraie destination au survol.
  - Pièces jointes : liste sous l'en-tête, clic = enregistrement dans Téléchargements (nom nettoyé, jamais d'écrasement).
  - Tri affiné : List-Id/Precedence → Newsletters ; adresses de service, domaines (PayPal, LastPass, Amazon…), Auto-Submitted, plateformes d'envoi → Autres. Bouton « Déplacer vers Autres/Prioritaire » (touche M) = règle par expéditeur gardée dans le cache.
  - Gmail synchronisé : Épingler = étoile (\\Flagged, relue au chargement), Supprimer = déplacement dans la corbeille (\\Trash), Archiver = déplacement dans « Tous les messages » (\\All). Rappels : toujours local.
- v0.1.5 (v11) : nouveau look inspiré du nouvel Outlook (sans éléments propres à Microsoft) : barre du haut avec recherche centrée, ruban d'actions avec icônes et « Nouveau courrier », panneaux arrondis sur fond teinté, onglets Prioritaire (personnes) / Autres (notifications + newsletters), regroupement par date, bouton Répondre dans l'en-tête du mail, dossiers renommés (Boîte de réception, Éléments envoyés). Plus de présélection automatique d'un mail (corrige un « Chargement… » sans fin).
- v0.1.4 (v10) : actualisation auto toutes les 3 min + bouton Actualiser (F5), fusion sans doublons ; « Charger les mails plus anciens » (40 par compte, recherche par UID, Inbox et Envoyés, respecte le filtre de compte).
- v0.1.1 (v7) : filtre par compte (clic sur une adresse dans « Comptes »), 2e compte Gmail ajouté par Laurent (testeur à déclarer dans la console Google).

## État
- [x] Étape 1 — Squelette Tauri + interface
- [x] Étape 2 — Gmail : OAuth2 PKCE, refresh token dans le trousseau, IMAP XOAUTH2 (60 derniers mails de l'Inbox, lecture), tri simple — **testé en réel**
- [x] Étape 3 — Microsoft : code présent dans `core/` mais **abandonné** ; bouton retiré de l'interface (v6)
- [~] Étape 4 :
  - [x] (v6) Affichage HTML dans un cadre isolé (sandbox sans script ni navigation, CSP interne), images intégrées (cid:) converties en data:, images distantes bloquées par défaut avec bouton « Afficher les images »
  - [x] (v0.2.0) Liens cliquables, pièces jointes (enregistrement), cache local JSON, tri affiné + règles par expéditeur
  - [ ] Connexion IMAP persistante, ne pas re-télécharger les pièces jointes à l'ouverture, recherche serveur, joindre un fichier à l'envoi
- [~] Étape 5 :
  - [x] Envoi SMTP XOAUTH2 (`smtp.gmail.com:465`), Répondre, sélecteur d'expéditeur, retrait de compte — envoi non testé en réel
  - [x] (v0.2.0) Supprimer, archiver, épingler synchronisés avec Gmail
  - [ ] Brouillons, envoi différé, rappels synchronisés

## Configuration Google (faite le 2026-11-02)
- Projet Google Cloud `sxm-linux`, écran de consentement Externe « SXM Mail », Gmail de Laurent en utilisateur de test, client « Application de bureau » (« SXM Mail Fedora »).
- **Mode Test** (« Publier » grisé) : reconnexion tous les 7 jours. À régler plus tard (page d'accueil + règles de confidentialité sur un domaine de Laurent).
- `~/.config/sxm-mail/config.json` créé sur le Fedora le 2026-11-04 (nouveau secret client).

## Décisions
- 2026-11-02 : **Microsoft abandonné** (Azure exige un tenant). Outlook : appli Electron en Flatpak. SXM Mail = Gmail seul.
- Front sans bundler (HTML/CSS/JS pur). Le texte des mails est échappé ; le HTML n'est affiché que dans un iframe `sandbox` sans aucun droit.
- Interface inspirée des clients mail modernes, identité propre (« SXM Mail »).
- Sans compte enregistré, l'application affiche des mails de démonstration.
