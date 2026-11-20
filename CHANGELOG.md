# Journal des versions de SXM Mail

## 0.3.5 (2026-11-20)

### Corrections
- Écriture identique sur Fedora et macOS : la police Inter est maintenant intégrée à l'application (elle n'était utilisée que si elle était installée sur l'ordinateur).

## 0.3.4 (2026-11-20)

### Corrections
- Fedora 45 : l'application ne démarrait plus (« libssl.so.3 introuvable ») car Fedora 45 est passé à OpenSSL 4. OpenSSL est désormais intégré à SXM Mail, qui ne dépend plus de la version installée sur le système.
- Fedora : le logo s'affiche tout de suite après l'installation (cache des icônes rafraîchi automatiquement).

## 0.3.3 (2026-11-19)

### Nouveautés
- Nouveau logo : une boîte aux lettres au drapeau levé, sur fond bleu ciel (Dock, menu des applications et en haut à gauche de la fenêtre).

## 0.3.2 (2026-11-19)

### Nouveautés
- Images dans le texte du message : colle une image ou une capture d'écran (Ctrl+V / ⌘V) directement dans le mail, ou utilise le bouton « Image ». Elle s'affiche dans le corps du mail chez le destinataire.
- Texte brut partout : un texte collé dans un message (depuis Word, une page web, un autre mail) arrive sans police, taille ni couleur, et ce que tu copies depuis un mail reçu part aussi en texte brut.

## 0.3.1 (2026-11-19)

### Nouveautés
- Pièces jointes à l'envoi (Nouveau courrier et Répondre) : bouton « Joindre un fichier » ou glisser-déposer sur la fenêtre, jusqu'à 25 Mo au total (limite de Gmail).
- Bouton « Rechercher des mises à jour » dans la barre du haut, avec une pastille quand une nouvelle version est disponible.

## 0.3.0 (2026-11-19)

### Nouveautés
- Nouveaux dossiers reliés à Gmail : Archives, Courrier indésirable et Corbeille.
- Dossiers personnels (« Mes dossiers ») : crée un dossier pour une personne ou un sujet, il apparaît aussi dans Gmail sous forme de libellé.
- Bouton « Classer » : range un mail dans un dossier, avec l'option « Toujours classer les mails de cette personne ici » (les mails déjà reçus et les prochains suivent automatiquement).
- Bouton « Indésirable » et, dans le courrier indésirable, « Ce n'est pas un spam ».
- Corbeille : « Restaurer » et « Supprimer définitivement » (avec confirmation).
- « Marquer non lu » et « Remettre dans la boîte » pour les rappels.
- Mises à jour intégrées : SXM Mail vérifie les nouvelles versions et s'installe en un clic (Fedora et macOS).
- Couleurs plus vives : icônes colorées par dossier et par action, avatars de couleurs variées, bouton « Nouveau courrier » en dégradé.

### Sécurité
- Les mises à jour ne viennent que de la page Releases officielle et sont vérifiées par empreinte SHA256 avant l'installation.
- La suppression définitive n'est possible que depuis la corbeille ou le courrier indésirable.

## 0.2.1 (2026-11-07)

### Nouveautés
- Version macOS (universelle : Mac Intel et Apple Silicon).
