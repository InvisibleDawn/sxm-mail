# Connecter Office 365 et Hotmail : enregistrer l'application dans Azure (≈ 10 min)

Un seul enregistrement sert aux deux : comptes professionnels (Office 365) et personnels (Hotmail / Outlook.com). Pas de secret à gérer.

1. Va sur https://portal.azure.com (un compte Microsoft gratuit suffit, même sans abonnement Azure) → **Microsoft Entra ID → Inscriptions d'applications → Nouvelle inscription**.
2. Nom : SXM Mail. Types de comptes pris en charge : **Comptes dans un annuaire organisationnel et comptes Microsoft personnels** (le dernier choix, multilocataire + personnel).
3. URI de redirection : plateforme **Client public / natif (mobile et bureau)**, valeur `http://localhost`.
4. Une fois créée, copie l'**ID d'application (client)**.
5. **Authentification** : vérifie que « Autoriser les flux de clients publics » est sur **Oui**.
6. **Autorisations d'API → Ajouter une autorisation → API que mon organisation utilise** → cherche « Office 365 Exchange Online » → **Autorisations déléguées** → coche `IMAP.AccessAsUser.All` **et** `SMTP.Send` (pour l'envoi). (`offline_access`, `openid`, `email`, `profile` sont déjà de base.)
7. Ajoute dans `~/.config/sxm-mail/config.json` (à côté des clés Google) :

```json
{
  "microsoft_client_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
}
```

8. Dans SXM Mail : **+ Ajouter un compte → Office 365 / Hotmail**.

## Pour l'Office 365 de l'hôtel
- Si l'admin du tenant bloque le consentement des utilisateurs, la connexion affichera « Approbation de l'administrateur requise ». Il faut alors que l'admin accorde le consentement pour l'application, ou autorise ce type d'application.
- Pour envoyer, l'authentification SMTP (« SMTP AUTH ») doit aussi être autorisée sur ta boîte : elle est souvent désactivée par défaut dans Office 365 (Exchange → boîte aux lettres → applications de messagerie → SMTP authentifié).
- L'IMAP doit être activé pour ta boîte (Exchange Online → boîte aux lettres → applications de messagerie → IMAP).
- Si tu ne peux pas l'obtenir, ajoute seulement Hotmail et Gmail : ils ne dépendent pas de l'admin.

## À savoir
- Le jeton de renouvellement est stocké dans le trousseau système, et mis à jour automatiquement (Microsoft le renouvelle à chaque usage).
- Je n'ai pas pu tester la connexion réelle (pas de Client ID dans mon environnement).
