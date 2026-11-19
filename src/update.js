// SXM Mail — mises à jour : vérification au démarrage puis toutes les 6 heures,
// petite fenêtre en bas à droite avec les nouveautés, installation en un clic.
'use strict';
(function () {
  const T = window.__TAURI__;
  const $ = (id) => document.getElementById(id);
  const escH = (s) => String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  let info = null, busy = false;

  // Notes de version (titres ### et puces -) → sections.
  function parseNotes(md) {
    const sections = []; let cur = null;
    for (const raw of (md || '').split(/\r?\n/)) {
      const line = raw.trim();
      if (line === '---') break; // la suite liste les fichiers à télécharger
      const h = line.match(/^#{2,4}\s+(.+)/);
      if (h) { cur = { title: h[1].replace(/\*\*/g, ''), items: [] }; sections.push(cur); continue; }
      const li = line.match(/^[-*]\s+(.+)/);
      if (li) {
        if (!cur) { cur = { title: 'Nouveautés', items: [] }; sections.push(cur); }
        cur.items.push(li[1].replace(/\*\*(.+?)\*\*/g, '$1').replace(/`(.+?)`/g, '$1'));
      }
    }
    return sections.filter((s) => s.items.length);
  }
  const kind = (title) => { const x = title.toLowerCase(); return /correct|fix|bug/.test(x) ? 'fix' : /sécurité|securit|fiabilit/.test(x) ? 'sec' : 'new'; };
  const plural = (n, one, many) => n + ' ' + (n > 1 ? many : one);

  function render() {
    const box = $('upd');
    if (!info) { box.hidden = true; return; }
    const sections = parseNotes(info.notes);
    const c = { new: 0, fix: 0, sec: 0 };
    sections.forEach((s) => (c[kind(s.title)] += s.items.length));
    const summary = [c.new && plural(c.new, 'nouveauté', 'nouveautés'), c.fix && plural(c.fix, 'correction', 'corrections'), c.sec && plural(c.sec, 'amélioration de sécurité', 'améliorations de sécurité')].filter(Boolean).join(' · ');
    $('upd-title').textContent = `SXM Mail ${info.version} est disponible`;
    $('upd-sub').textContent = summary || `Tu as la version ${info.current}.`;
    $('upd-list').innerHTML = sections.map((s) => `<section class="upd-sec ${kind(s.title)}"><h4>${escH(s.title)}</h4><ul>${s.items.map((i) => `<li>${escH(i)}</li>`).join('')}</ul></section>`).join('');
    $('upd-toggle').hidden = !sections.length;
    box.hidden = false;
  }

  function setState(msg, o = {}) {
    $('upd-status').textContent = msg || ''; $('upd-status').hidden = !msg;
    $('upd-bar').hidden = !o.progress;
    $('upd-actions').hidden = !!o.hideActions;
    $('upd-restart-row').hidden = !o.restart;
  }

  async function check(manual) {
    if (!T || !T.core) { if (manual) toast('Disponible dans l’application installée'); return; }
    const btn = $('btn-check-update'); btn.classList.add('spin');
    try {
      info = await T.core.invoke('check_update');
      if (info) { setState(''); render(); }
      else if (manual) toast('Tu as déjà la dernière version 👍');
    } catch (e) {
      if (manual) showInfo('Vérification des mises à jour impossible', String(e));
    } finally { btn.classList.remove('spin'); }
  }

  if (T && T.event) {
    T.event.listen('update-progress', ({ payload }) => {
      const pct = payload.total ? Math.min(100, Math.round((payload.done / payload.total) * 100)) : 0;
      $('upd-fill').style.width = pct + '%';
      setState(`Téléchargement… ${pct} %`, { progress: true, hideActions: true });
    });
  }

  $('upd-toggle').onclick = () => {
    const open = $('upd').classList.toggle('open');
    $('upd-toggle').setAttribute('aria-expanded', open);
    $('upd-toggle-label').textContent = open ? 'Masquer les nouveautés' : 'Voir les nouveautés';
  };
  $('upd-later').onclick = () => { $('upd').hidden = true; };
  $('upd-close').onclick = () => { if (!busy) $('upd').hidden = true; };
  $('upd-restart').onclick = () => T.core.invoke('restart_app');
  $('btn-check-update').onclick = () => check(true);

  $('upd-go').onclick = async () => {
    if (busy || !info) return;
    busy = true;
    $('upd-fill').style.width = '0%';
    setState('Téléchargement… 0 %', { progress: true, hideActions: true });
    try {
      await T.core.invoke('install_update', { url: info.asset_url, name: info.asset_name, digest: info.digest || null });
      setState('Mise à jour installée ✔ Redémarre SXM Mail pour en profiter.', { hideActions: true, restart: true });
    } catch (e) {
      setState('Échec de la mise à jour : ' + e);
    }
    busy = false;
  };

  // Vérification 5 secondes après le lancement, puis toutes les 6 heures.
  setTimeout(() => check(false), 5000);
  setInterval(() => check(false), 6 * 3600 * 1000);
})();
