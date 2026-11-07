# Connecter Gmail : créer l'identifiant OAuth (≈ 10 min)

Google exige que toute application déclare un « Client ID ». C'est gratuit et fait une seule fois.
Les intitulés de la console changent parfois légèrement ; le principe reste le même.

1. Va sur https://console.cloud.google.com et crée un projet (ex. « SXM Mail »).
2. **API et services → Écran de consentement OAuth** (ou « Google Auth Platform ») :
   - Type d'utilisateur : **Externe**.
   - Nom de l'application : SXM Mail. Adresse e-mail d'assistance : la tienne.
   - Ajoute ton adresse Gmail comme **utilisateur de test**.
3. **Identifiants → Créer des identifiants → ID client OAuth** :
   - Type d'application : **Application de bureau**.
   - Copie l'**ID client** et le **code secret du client**.
4. **Publie l'application** (écran de consentement → « Publier l'application », statut « En production »).
   Sans cette étape, Google expire le jeton de connexion **tous les 7 jours** et il faudrait te reconnecter chaque semaine.
5. Crée le fichier `~/.config/sxm-mail/config.json` :

```json
{
  "google_client_id": "xxxxxxxx.apps.googleusercontent.com",
  "google_client_secret": "GOCSPX-xxxxxxxx"
}
```

6. Lance SXM Mail, clique sur **+ Ajouter un compte**, connecte-toi dans le navigateur.

## À savoir
- Tant que l'application n'est pas validée par Google, l'écran de connexion affiche « Google n'a pas validé cette application ».
  Pour un usage personnel, clique **Paramètres avancés → Accéder à SXM Mail**. Tes données ne passent par aucun serveur : tout reste sur ton ordinateur.
- Le jeton de renouvellement est stocké dans le trousseau système (GNOME Keyring / KWallet), pas dans un fichier.
- L'accès IMAP est le scope complet `https://mail.google.com/` : c'est celui que Google impose pour lire et envoyer par IMAP/SMTP.
- Pour retirer l'accès : https://myaccount.google.com/permissions
