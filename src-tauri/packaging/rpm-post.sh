#!/bin/sh
# Après installation / désinstallation : rafraîchit le cache des icônes et le menu des applications,
# pour que le logo de SXM Mail s'affiche tout de suite.
gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor >/dev/null 2>&1 || true
update-desktop-database -q /usr/share/applications >/dev/null 2>&1 || true
exit 0
