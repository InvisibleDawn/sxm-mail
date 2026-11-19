// SXM Mail — interface. Sans compte : données de démonstration. Avec compte Gmail : vrais mails.
'use strict';

const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);
const hasBackend = !!(window.__TAURI__ && window.__TAURI__.core);
// Couleurs vives (comptes, avatars, dossiers personnels)
const PALETTE = ['#3b82f6', '#ec4899', '#10b981', '#8b5cf6', '#f97316', '#06b6d4', '#ef4444', '#eab308', '#14b8a6', '#a855f7'];
let realMode = false;

let ACCOUNTS = [
  { id: 'o365', name: 'Office 365', color: '#3b82f6' },
  { id: 'gmail', name: 'Gmail', color: '#ec4899' },
  { id: 'hotmail', name: 'Hotmail', color: '#10b981' },
];

// state = dossier du message : inbox | sent | archive | junk | trash | snoozed | u:<nom du dossier personnel>
let MESSAGES = [
  { id: '1', from: 'Marie Laurent', addr: 'marie@exemple.com', acct: 'o365', cat: 'people', time: '14:12', unread: true, pinned: false, state: 'inbox',
    subject: 'Réunion de mardi', body: 'Bonjour,\n\nPeux-tu confirmer ta présence pour la réunion de mardi à 10h ? Il faudra prévoir un point sur le réseau.\n\nMerci,\nMarie' },
  { id: '2', from: 'Jean Petit', addr: 'jean@exemple.com', acct: 'gmail', cat: 'people', time: '11:40', unread: true, pinned: false, state: 'inbox',
    subject: 'Photos du week-end', body: 'Salut !\n\nJe t’envoie les photos comme promis. Dis-moi si tu les reçois bien.\n\nJean' },
  { id: '3', from: 'Banque', addr: 'noreply@banque.fr', acct: 'hotmail', cat: 'notifications', time: '09:05', unread: false, pinned: false, state: 'inbox',
    subject: 'Votre relevé est disponible', body: 'Votre relevé mensuel est disponible dans votre espace client.' },
  { id: '4', from: 'Fournisseur réseau', addr: 'commandes@reseau.fr', acct: 'o365', cat: 'notifications', time: 'Hier', unread: false, pinned: true, state: 'inbox',
    subject: 'Commande expédiée', body: 'Votre commande a été expédiée.\nNuméro de suivi : [NUMÉRO].' },
  { id: '5', from: 'Lettre tech', addr: 'news@lettre.tech', acct: 'gmail', cat: 'newsletters', time: 'Hier', unread: false, pinned: false, state: 'inbox',
    subject: 'Les nouveautés Linux de la semaine', body: 'Au programme cette semaine : Fedora, GNOME et le noyau Linux.' },
  { id: '6', from: 'Gagnant Loto', addr: 'promo@gains-faciles.biz', acct: 'gmail', cat: 'notifications', time: 'Hier', unread: true, pinned: false, state: 'junk',
    subject: 'Vous avez gagné 1 000 000 € !!!', body: 'Cliquez vite pour réclamer votre gain.' },
  { id: '7', from: 'Maman', addr: 'maman@exemple.com', acct: 'gmail', cat: 'people', time: 'Lun.', unread: false, pinned: false, state: 'u:Famille',
    subject: 'Repas de dimanche', body: 'On mange à la maison dimanche, viens avec les enfants !' },
  { id: '8', from: 'Ancien abonnement', addr: 'info@service.com', acct: 'gmail', cat: 'newsletters', time: '12/11', unread: false, pinned: false, state: 'trash',
    subject: 'Dernière chance', body: 'Votre offre expire bientôt.' },
];

// Copie de la démo, restaurée si le dernier compte est retiré.
const DEMO_ACCOUNTS = ACCOUNTS.slice();
const DEMO_MESSAGES = MESSAGES.map((m) => ({ ...m }));
const DEMO_FOLDERS = [{ label: 'Famille', accts: { gmail: 'Famille' } }, { label: 'Hôtel', accts: { o365: 'H&APQ-tel' } }];

// Dossiers fixes. server = chargé depuis Gmail au premier affichage.
const FOLDERS = [
  { id: 'inbox', label: 'Boîte de réception', icon: 'inbox', color: '#3b82f6' },
  { id: 'pinned', label: 'Épinglés', icon: 'pin', color: '#f59e0b' },
  { id: 'snoozed', label: 'Rappels', icon: 'snooze', color: '#8b5cf6' },
  { id: 'sent', label: 'Éléments envoyés', icon: 'send', color: '#06b6d4', server: true },
  { id: 'archive', label: 'Archives', icon: 'archive', color: '#10b981', server: true },
  { id: 'junk', label: 'Courrier indésirable', icon: 'spam', color: '#f97316', server: true },
  { id: 'trash', label: 'Corbeille', icon: 'delete', color: '#f43f5e', server: true },
];

const st = {
  folder: 'inbox', tab: 'focused', sel: '1', q: '', rules: {},
  folderRules: {},            // adresse → nom du dossier personnel (« Toujours classer ici »)
  userFolders: DEMO_FOLDERS.map((f) => ({ ...f })), // [{ label, accts: { compte: nom IMAP } }]
  loaded: {},                 // dossiers serveur déjà chargés pendant cette session
};

// Onglets de la boîte de réception : « Prioritaire » = personnes, « Autres » = notifications et newsletters.
const tabOf = (m) => st.rules[(m.addr || '').toLowerCase()] || (m.cat === 'people' ? 'focused' : 'other');

// Icônes (traits simples, couleur du texte)
const svg = (d) => `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${d}</svg>`;
const ICON = {
  reply: svg('<path d="M9 14 4 9l5-5"/><path d="M4 9h10a6 6 0 0 1 6 6v4"/>'),
  archive: svg('<rect x="3" y="4" width="18" height="5" rx="1"/><path d="M5 9v10a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V9"/><path d="M10 13h4"/>'),
  pin: svg('<path d="M12 17v5"/><path d="M9 3h6l-1 6 3 4H7l3-4Z"/>'),
  snooze: svg('<circle cx="12" cy="13" r="8"/><path d="M12 9v4l2 2"/><path d="M5 3 2 6"/><path d="m22 6-3-3"/>'),
  move: svg('<path d="M5 12h14"/><path d="m13 6 6 6-6 6"/>'),
  clip: svg('<path d="m21 11-8.5 8.5a5 5 0 0 1-7-7L14 4a3.3 3.3 0 0 1 4.7 4.7L10 17.4a1.7 1.7 0 0 1-2.4-2.4l7.8-7.8"/>'),
  inbox: svg('<path d="M3 13h5l1.5 3h5L16 13h5"/><path d="M5.5 5h13L21 13v5a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1v-5Z"/>'),
  send: svg('<path d="m21 3-9.5 9.5"/><path d="M21 3 14.5 21l-3-8.5L3 9.5Z"/>'),
  delete: svg('<path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="M19 6l-1 14H6L5 6"/>'),
  purge: svg('<path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="M19 6l-1 14H6L5 6"/><path d="m10 11 4 4M14 11l-4 4"/>'),
  spam: svg('<path d="M7.9 2h8.2L22 7.9v8.2L16.1 22H7.9L2 16.1V7.9Z"/><path d="M12 8v4M12 16h.01"/>'),
  notspam: svg('<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10Z"/><path d="m9 12 2 2 4-4"/>'),
  restore: svg('<path d="M3 12a9 9 0 1 0 3-6.7L3 8"/><path d="M3 3v5h5"/>'),
  folder: svg('<path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.7-.9l-.8-1.2A2 2 0 0 0 7.9 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>'),
  file: svg('<path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.7-.9l-.8-1.2A2 2 0 0 0 7.9 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/><path d="M12 10v6M9 13l3 3 3-3"/>'),
  unread: svg('<rect x="2" y="5" width="16" height="14" rx="2"/><path d="m2 7 8 6 8-6"/><circle cx="20" cy="5" r="3" fill="currentColor"/>'),
  unsnooze: svg('<path d="M3 13h5l1.5 3h5L16 13h5"/><path d="M5.5 5h13L21 13v5a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1v-5Z"/>'),
};
// Couleur de chaque action du ruban
const ACT_COLOR = { reply: '#3b82f6', delete: '#f43f5e', purge: '#e11d48', archive: '#10b981', file: '#8b5cf6', spam: '#f97316', notspam: '#10b981',
  restore: '#10b981', pin: '#f59e0b', snooze: '#a855f7', unsnooze: '#3b82f6', move: '#06b6d4', unread: '#0ea5e9' };

const EMPTY_READER = `<div class="reader-empty"><svg viewBox="0 0 200 120" aria-hidden="true"><defs><linearGradient id="er-g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#3B82F6"/><stop offset="1" stop-color="#8B5CF6"/></linearGradient></defs><path class="wave1" d="M0 84 C50 60 90 96 140 74 S190 56 200 64 V120 H0z"/><path class="wave2" d="M0 100 C60 84 120 112 200 94 V120 H0z"/><circle cx="40" cy="30" r="6" fill="#F472B6"/><circle cx="166" cy="26" r="9" fill="#FBBF24"/><circle cx="156" cy="62" r="4" fill="#34D399"/><rect x="62" y="22" width="76" height="52" rx="9" fill="url(#er-g)"/><path d="m68 30 32 23 32-23" fill="none" stroke="#E0E7FF" stroke-width="4.5" stroke-linecap="round" stroke-linejoin="round"/></svg><strong>Aucun message sélectionné</strong><span>Choisis un message dans la liste pour le lire ici.</span></div>`;
const $ = (id) => document.getElementById(id);
const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const initials = (n) => n.split(/\s+/).filter(Boolean).slice(0, 2).map((w) => w[0]).join('').toUpperCase() || '?';
const acct = (id) => ACCOUNTS.find((a) => a.id === id);
const byId = (id) => MESSAGES.find((m) => m.id === id);
const hash = (s) => { let h = 0; for (const c of String(s)) h = (h * 31 + c.codePointAt(0)) >>> 0; return h; };
const colorOf = (s) => PALETTE[hash(String(s).toLowerCase()) % PALETTE.length];
const avatarColor = (m) => colorOf(m.addr || m.from);
const userKey = (label) => 'u:' + label;
const isUserKey = (k) => typeof k === 'string' && k.startsWith('u:');
const folderMeta = (k) => FOLDERS.find((f) => f.id === k) || (isUserKey(k) ? { id: k, label: k.slice(2), icon: 'folder', color: colorOf(k.slice(2)), server: true, user: true } : null);
const isServerFolder = (k) => !!(folderMeta(k) || {}).server;

function visible() {
  const q = st.q.trim().toLowerCase();
  return MESSAGES.filter((m) => {
    if (m.state === 'deleted') return false;
    if (st.acct && m.acct !== st.acct) return false;
    if (st.folder === 'pinned' ? !m.pinned || m.state === 'trash' || m.state === 'junk' : m.state !== st.folder) return false;
    if (st.folder === 'inbox' && !q && tabOf(m) !== st.tab) return false;
    return !q || (m.from + ' ' + (m.addr || '') + ' ' + m.subject + ' ' + (m.body || m.snip || '')).toLowerCase().includes(q);
  });
}

function toast(msg) {
  const t = $('toast'); t.textContent = msg; t.classList.add('show');
  clearTimeout(toast.t); toast.t = setTimeout(() => t.classList.remove('show'), 2600);
}

function msgButton(m) {
  return `<button type="button" class="msg ${m.unread ? 'unread' : ''} ${m.id === st.sel ? 'on' : ''}" data-id="${esc(m.id)}" style="--c:${avatarColor(m)}">
    <span class="avatar" aria-hidden="true">${esc(initials(m.from))}</span>
    <span class="msg-body">
      <span class="row"><span>${m.sent ? 'À : ' : ''}${esc(m.from)}</span><span class="time">${m.pinned ? '<span class="pin-mark" title="Épinglé">' + ICON.pin + '</span> ' : ''}${esc(m.time)}</span></span>
      <span class="subj">${esc(m.subject)}</span>
      <span class="snip">${esc((m.body || m.snip || '').replace(/\s+/g, ' '))}</span>
    </span></button>`;
}

function folderCount(id, pool) {
  if (id === 'inbox' || id === 'junk' || isUserKey(id)) return pool.filter((m) => m.state === id && m.unread).length;
  if (id === 'pinned') return pool.filter((m) => m.pinned && m.state !== 'trash' && m.state !== 'junk').length;
  if (id === 'snoozed') return pool.filter((m) => m.state === 'snoozed').length;
  return 0;
}

function folderButton(f, pool) {
  const n = folderCount(f.id, pool);
  return `<button type="button" class="folder ${f.id === st.folder ? 'on' : ''}" data-folder="${esc(f.id)}" style="--c:${f.color}"><span class="f-ico">${ICON[f.icon]}</span><span class="f-label">${esc(f.label)}</span><span class="count">${n || ''}</span></button>`;
}

function renderFolders() {
  const pool = MESSAGES.filter((m) => !st.acct || m.acct === st.acct);
  $('folders').innerHTML = FOLDERS.map((f) => folderButton(f, pool)).join('');
  const mine = st.userFolders.filter((f) => !st.acct || f.accts[st.acct]);
  $('user-folders').innerHTML = mine.length
    ? mine.map((f) => folderButton(folderMeta(userKey(f.label)), pool)).join('')
    : '<p class="side-empty">Crée un dossier pour ranger les mails d’une personne ou d’un sujet.</p>';
}

function renderAccounts() {
  $('accounts').innerHTML = ACCOUNTS.map((a) => `<div class="account ${st.acct === a.id ? 'on' : ''}" style="--c:${a.color}"><span class="dot"></span><button type="button" class="acc-name" data-acct="${esc(a.id)}" aria-pressed="${st.acct === a.id}" title="Afficher seulement ce compte (cliquer à nouveau pour tout afficher)">${esc(a.name)}</button>${realMode ? `<button type="button" class="rm" data-rm="${esc(a.id)}" aria-label="Retirer ${esc(a.name)}">${st.removing === a.id ? 'Confirmer ?' : '×'}</button>` : ''}</div>`).join('');
}

function dayLabel(ts) {
  if (!ts) return 'Récents';
  const d = new Date(ts * 1000), now = new Date();
  const day = (x) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const diff = Math.round((day(now) - day(d)) / 86400000);
  if (diff <= 0) return 'Aujourd’hui';
  if (diff === 1) return 'Hier';
  if (diff < 7) return 'Cette semaine';
  if (d.getMonth() === now.getMonth() && d.getFullYear() === now.getFullYear()) return 'Plus tôt ce mois-ci';
  const label = d.toLocaleDateString('fr-FR', { month: 'long', year: 'numeric' });
  return label.charAt(0).toUpperCase() + label.slice(1);
}

function renderTabs() {
  const f = folderMeta(st.folder) || {};
  if (st.folder !== 'inbox' || st.q.trim()) {
    $('tabs').innerHTML = st.q.trim() ? '' : `<div class="list-title" style="--c:${f.color}"><span class="f-ico">${ICON[f.icon] || ''}</span><strong>${esc(f.label || '')}</strong>${folderHint(st.folder)}</div>`;
    return;
  }
  const pool = MESSAGES.filter((m) => m.state === 'inbox' && (!st.acct || m.acct === st.acct));
  $('tabs').innerHTML = '<div class="seg">' + [['focused', 'Prioritaire'], ['other', 'Autres']].map(([id, label]) => {
    const n = pool.filter((m) => tabOf(m) === id && m.unread).length;
    return `<button type="button" class="tab ${st.tab === id ? 'on' : ''}" data-tab="${id}" aria-pressed="${st.tab === id}">${label}${n ? ` <span class="count">${n}</span>` : ''}</button>`;
  }).join('') + '</div>';
}

function folderHint(k) {
  if (k === 'junk') return '<small>Gmail supprime ces messages après 30 jours.</small>';
  if (k === 'trash') return '<small>Vidée automatiquement par Gmail après 30 jours.</small>';
  if (k === 'archive') return '<small>Les mails sortis de la boîte de réception.</small>';
  if (k === 'snoozed') return '<small>Sur cet ordinateur uniquement.</small>';
  if (isUserKey(k)) {
    const who = Object.keys(st.folderRules).filter((a) => st.folderRules[a] === k.slice(2));
    return who.length ? `<small>Classement automatique : ${esc(who.slice(0, 3).join(', '))}${who.length > 3 ? '…' : ''}</small>` : '';
  }
  return '';
}

function renderList() {
  renderTabs();
  const list = visible();
  const more = realMode && !st.q && isServerFolder(st.folder) || (realMode && !st.q && st.folder === 'inbox')
    ? `<button type="button" class="btn-more" id="btn-more" ${st.loadingMore ? 'disabled' : ''}>${st.loadingMore ? 'Chargement…' : 'Charger les mails plus anciens'}</button>` : '';
  if (st.loading === st.folder && !list.length) { $('messages').innerHTML = '<div class="empty"><span class="spinner"></span>Chargement…</div>'; return; }
  if (!list.length) { $('messages').innerHTML = `<div class="empty">${st.q ? 'Aucun résultat.' : 'Aucun message ici.'}</div>` + more; return; }
  let html = '', group = null;
  for (const m of list) {
    const g = dayLabel(m.ts);
    if (g !== group) { html += `<div class="group-head">${esc(g)}</div>`; group = g; }
    html += msgButton(m);
  }
  $('messages').innerHTML = html + more;
}

// Actions proposées selon le dossier du message.
function actionsFor(m) {
  if (m.state === 'trash') return [['restore', 'Restaurer', 'Z'], ['purge', 'Supprimer définitivement', '⇧#']];
  if (m.state === 'junk') return [['notspam', 'Ce n’est pas un spam', '⇧!'], ['purge', 'Supprimer définitivement', '⇧#']];
  const a = [['reply', 'Répondre', 'R'], ['delete', 'Supprimer', '#']];
  if (m.state === 'snoozed') a.push(['unsnooze', 'Remettre dans la boîte', 'U']);
  if (m.state !== 'archive' && m.state !== 'sent') a.push(['archive', 'Archiver', 'E']);
  if (!m.sent) a.push(['file', 'Classer', 'V']);
  if (!m.sent) a.push(['spam', 'Indésirable', '!']);
  a.push(['pin', m.pinned ? 'Désépingler' : 'Épingler', 'S']);
  if (m.state === 'inbox') a.push(['snooze', 'Rappeler plus tard', 'H']);
  if (!m.sent) a.push(['unread', 'Marquer non lu', 'U']);
  if (m.state === 'inbox') a.push(['move', tabOf(m) === 'focused' ? 'Vers Autres' : 'Vers Prioritaire', 'M']);
  return a;
}

function renderReader() {
  const m = byId(st.sel);
  const bar = $('toolbar');
  if (!m) { bar.innerHTML = ''; $('reading').innerHTML = EMPTY_READER; return; }
  bar.innerHTML = actionsFor(m)
    .map(([a, l, k]) => `<button type="button" data-act="${a}" title="${l} (${k})" style="--c:${ACT_COLOR[a]}"><span class="t-ico">${ICON[a]}</span><span>${l}</span></button>`).join('');
  const where = folderMeta(m.state);
  const badge = where && m.state !== 'inbox' ? `<span class="where" style="--c:${where.color}">${ICON[where.icon]}${esc(where.label)}</span>` : '';
  const c = avatarColor(m);
  $('reading').innerHTML = `<div class="r-banner" style="--c:${c}" aria-hidden="true"><i></i><i></i><i></i></div>
    <div class="who"><span class="avatar lg" style="--c:${c}" aria-hidden="true">${esc(initials(m.from))}</span>
      <div class="who-text"><div class="name">${m.sent ? 'À : ' : ''}${esc(m.from)}${m.addr ? ` <span class="addr">&lt;${esc(m.addr)}&gt;</span>` : ''}</div><div class="meta">${m.sent ? 'Envoyé depuis' : 'À'} : ${esc((acct(m.acct) || {}).name || 'moi')}</div></div><div class="when">${esc(m.time)}</div>${m.state !== 'junk' && m.state !== 'trash' ? `<div class="quick"><button type="button" data-act="reply" aria-label="Répondre" title="Répondre (R)">${ICON.reply}</button></div>` : ''}</div>
    <h1>${esc(m.subject)}${badge}</h1>
    ${m.state === 'junk' ? '<div class="warn-bar">' + ICON.spam + '<span>Message classé comme indésirable : ne clique pas sur ses liens si tu ne connais pas l’expéditeur.</span></div>' : ''}
    ${m.atts && m.atts.length ? '<div class="att-bar">' + m.atts.map((a) => `<button type="button" class="att" data-att="${a.index}" title="Enregistrer dans Téléchargements">${ICON.clip}<span class="att-name">${esc(a.filename)}</span><span class="att-size">${fmtSize(a.size)}</span></button>`).join('') + '</div>' : ''}
    ${m.body == null ? '<p class="body">Chargement…</p>'
      : m.html ? (m.remote && !m.showImages && m.state !== 'junk' ? '<div class="img-bar"><span>Images distantes bloquées (protection contre le pistage).</span><button type="button" data-act="images">Afficher les images</button></div>' : '') + '<div class="html-slot" id="html-slot"></div>'
      : `<p class="body">${esc(m.body)}</p>`}`;
  if (m.html && m.body != null) mountHtml(m);
}

// Le HTML du mail est affiché dans un cadre isolé : aucun script, aucune navigation,
// et une politique de sécurité qui bloque tout chargement distant sauf les images autorisées.
function mountHtml(m) {
  const f = document.createElement('iframe');
  // allow-same-origin SANS allow-scripts : aucun script du mail ne s'exécute,
  // mais l'appli peut lire le cadre pour intercepter les clics sur les liens.
  f.setAttribute('sandbox', 'allow-same-origin');
  f.setAttribute('referrerpolicy', 'no-referrer');
  f.title = 'Contenu du message';
  const img = m.showImages ? 'data: https: http:' : 'data:';
  f.srcdoc = '<!doctype html><meta charset="utf-8">'
    + `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src ${img}; font-src data:">`
    + '<style>html,body{margin:0;background:#fff;color:#1a1d23;font-family:system-ui,sans-serif;font-size:15px;line-height:1.6;overflow-wrap:anywhere}body{padding:20px 4px}' + '*{text-align:left!important}table,div,center{margin-left:0!important}table{max-width:100%!important}table[width]{width:100%!important}p{margin:0 0 14px}img{max-width:100%;height:auto}</style>'
    + m.html;
  f.addEventListener('load', () => {
    const d = f.contentDocument; if (!d) return;
    // Survol d'un lien : affiche sa vraie destination (protection contre l'hameçonnage).
    d.querySelectorAll('a[href]').forEach((a) => { a.title = a.getAttribute('href'); });
    d.addEventListener('click', (e) => {
      const a = e.target.closest && e.target.closest('a[href]'); if (!a) return;
      e.preventDefault();
      const href = a.getAttribute('href').trim();
      if (/^mailto:/i.test(href)) { openCompose(); $('c-to').value = decodeURIComponent(href.slice(7).split('?')[0]); return; }
      if (!/^https?:\/\//i.test(href)) return;
      if (!hasBackend) { toast('Lien : ' + href); return; }
      invoke('open_url', { url: href }).then(() => toast('Lien ouvert dans le navigateur')).catch((err) => showInfo('Lien impossible à ouvrir', String(err)));
    });
  });
  $('html-slot').appendChild(f);
}

function fmtSize(n) {
  if (n < 1024) return n + ' o';
  if (n < 1048576) return Math.round(n / 1024) + ' Ko';
  return (n / 1048576).toFixed(1).replace('.', ',') + ' Mo';
}

async function saveAttachment(i) {
  const m = byId(st.sel); if (!m || !m.real || !m.atts) return;
  const att = m.atts.find((a) => a.index === i); if (!att) return;
  toast('Enregistrement de « ' + att.filename + ' »…');
  try {
    const path = await invoke('save_attachment', { accountId: m.acct, mailbox: m.mailbox || 'INBOX', uid: m.uid, index: i });
    showInfo('Pièce jointe enregistrée', path);
  } catch (e) { showInfo('Pièce jointe non enregistrée', String(e)); }
}

function render() { renderFolders(); renderList(); renderReader(); }

function select(id) {
  st.sel = id; const m = byId(id); if (m) m.unread = false; render();
  // Message réel : le corps est chargé à la demande (le serveur le marque comme lu).
  if (m && m.real && m.body == null) {
    invoke('fetch_message', { accountId: m.acct, mailbox: m.mailbox || 'INBOX', uid: m.uid })
      .then((b) => { m.body = b.text; m.html = b.html; m.remote = b.has_remote_images; m.atts = b.attachments || []; saveCache(); if (st.sel === id) { renderReader(); renderList(); } })
      .catch((e) => { m.body = '(Impossible de charger ce message)\n' + e; if (st.sel === id) renderReader(); });
  }
}
function nextAfter(id) { const l = visible(); const i = l.findIndex((m) => m.id === id); return (l[i + 1] || l[i - 1] || {}).id; }

// --- Déplacements (Gmail) -------------------------------------------------------------------

const TARGET_TEXT = { inbox: 'Remis dans la boîte de réception', trash: 'Placé dans la corbeille', archive: 'Archivé', junk: 'Classé comme indésirable' };

// Déplace un message vers un dossier : target = inbox | trash | archive | junk | u:<nom>.
async function moveTo(m, target, opts = {}) {
  const nxt = opts.quiet ? null : nextAfter(m.id);
  const before = m.state;
  const label = isUserKey(target) ? target.slice(2) : null;
  m.state = target; if (st.sel === m.id && !opts.quiet) st.sel = null;
  if (!opts.quiet) render();
  if (m.real) {
    try {
      let targetName = null;
      if (label) targetName = await ensureFolder(m.acct, label);
      await invoke('move_message', { accountId: m.acct, mailbox: m.mailbox || 'INBOX', uid: m.uid, target: label ? 'user' : target, targetName });
    } catch (e) {
      m.state = before; render();
      if (!opts.quiet) showInfo('Déplacement impossible', String(e));
      return false;
    }
    // Le message a un nouvel identifiant dans le dossier d'arrivée : on le relira à la prochaine visite.
    MESSAGES = MESSAGES.filter((x) => x !== m);
    st.loaded[target] = false;
  }
  if (!opts.quiet) {
    toast(label ? 'Classé dans « ' + label + ' »' : TARGET_TEXT[target] || 'Déplacé');
    saveCache(); if (nxt) select(nxt); else render();
  }
  return true;
}

// Nom IMAP du dossier personnel dans ce compte (créé dans Gmail s'il n'existe pas encore).
async function ensureFolder(accountId, label) {
  let f = st.userFolders.find((x) => x.label.toLowerCase() === label.toLowerCase());
  if (f && f.accts[accountId]) return f.accts[accountId];
  const raw = await invoke('create_folder', { accountId, label });
  if (!f) { f = { label, accts: {} }; st.userFolders.push(f); sortFolders(); }
  f.accts[accountId] = raw;
  renderFolders(); saveCache();
  return raw;
}
const sortFolders = () => st.userFolders.sort((a, b) => a.label.localeCompare(b.label, 'fr', { sensitivity: 'base' }));

async function act(a) {
  const m = byId(st.sel); if (!m) return;
  if (a === 'reply') { openCompose(m); return; }
  if (a === 'images') { m.showImages = true; renderReader(); return; }
  if (a === 'file') { openFileDialog(m); return; }
  const gmail = (action) => invoke('message_action', { accountId: m.acct, mailbox: m.mailbox || 'INBOX', uid: m.uid, action });
  if (a === 'move') {
    const key = (m.addr || '').toLowerCase(); if (!key) return;
    const to = tabOf(m) === 'focused' ? 'other' : 'focused';
    st.rules[key] = to; st.sel = null; render(); saveCache();
    toast('Les mails de ' + m.addr + ' iront dans « ' + (to === 'focused' ? 'Prioritaire' : 'Autres') + ' »');
    return;
  }
  if (a === 'pin' || a === 'unread') {
    const field = a === 'pin' ? 'pinned' : 'unread';
    const before = m[field]; m[field] = a === 'pin' ? !before : true;
    if (a === 'unread') st.sel = null;
    render();
    if (m.real) {
      try { await gmail(a === 'pin' ? (m.pinned ? 'star' : 'unstar') : 'unread'); }
      catch (e) { m[field] = before; render(); showInfo(a === 'pin' ? 'Épinglage impossible' : 'Action impossible', String(e)); return; }
    }
    toast(a === 'unread' ? 'Marqué comme non lu' : m.pinned ? 'Épinglé (étoile dans Gmail)' : 'Désépinglé'); saveCache(); return;
  }
  if (a === 'snooze' || a === 'unsnooze') {
    const nxt = nextAfter(m.id); m.state = a === 'snooze' ? 'snoozed' : 'inbox'; st.sel = null; render(); saveCache();
    toast(a === 'snooze' ? 'Déplacé dans Rappels (sur cet ordinateur uniquement)' : 'Remis dans la boîte de réception'); if (nxt) select(nxt); return;
  }
  if (a === 'purge') {
    if (!await confirmBox('Supprimer définitivement ?', `« ${m.subject} » sera effacé de Gmail. Cette action est irréversible.`, 'Supprimer définitivement')) return;
    const nxt = nextAfter(m.id);
    if (m.real) {
      try { await gmail('purge'); } catch (e) { showInfo('Suppression impossible', String(e)); return; }
    }
    MESSAGES = MESSAGES.filter((x) => x !== m); st.sel = null;
    toast('Message supprimé définitivement'); saveCache(); if (nxt) select(nxt); else render(); return;
  }
  // Un message en Rappels est toujours dans la boîte de réception côté Gmail.
  if (a === 'delete') moveTo(m, 'trash');
  else if (a === 'archive') moveTo(m, 'archive');
  else if (a === 'spam') moveTo(m, 'junk');
  else if (a === 'notspam' || a === 'restore') moveTo(m, 'inbox');
}

document.addEventListener('click', (e) => {
  const t = e.target.closest('[data-id],[data-folder],[data-act],[data-rm],[data-acct],[data-tab],[data-att],[data-file]');
  if (!t) return;
  if (t.dataset.file != null) fileInto(t.dataset.file);
  else if (t.dataset.att != null) saveAttachment(Number(t.dataset.att));
  else if (t.dataset.rm) removeAccount(t.dataset.rm);
  else if (t.dataset.tab) { st.tab = t.dataset.tab; st.sel = null; render(); }
  else if (t.dataset.acct) { st.acct = st.acct === t.dataset.acct ? null : t.dataset.acct; st.sel = null; renderAccounts(); render(); }
  else if (t.dataset.id) select(t.dataset.id);
  else if (t.dataset.folder) openFolder(t.dataset.folder);
  else if (t.dataset.act) act(t.dataset.act);
});

function openFolder(k) {
  if (k === 'inbox' && st.folder === 'inbox') { st.acct = null; renderAccounts(); }
  st.folder = k; st.sel = null; render();
  if (realMode && isServerFolder(k) && !st.loaded[k]) loadFolder(k);
}

$('q').addEventListener('input', (e) => { st.q = e.target.value; renderList(); });

// Raccourcis clavier (hors champs de saisie)
document.addEventListener('keydown', (e) => {
  if (e.target.closest('input,textarea,select,dialog') || e.ctrlKey || e.metaKey || e.altKey) return;
  const l = visible(); const i = l.findIndex((m) => m.id === st.sel);
  const m = byId(st.sel);
  const has = (a) => m && actionsFor(m).some(([x]) => x === a);
  const run = (a) => { if (has(a)) act(a); };
  if (e.key === 'j' && l[i + 1]) select(l[i + 1].id);
  else if (e.key === 'k' && l[i - 1]) select(l[i - 1].id);
  else if (e.key === 'e') run('archive');
  else if (e.key === 's') run('pin');
  else if (e.key === 'h') run('snooze');
  else if (e.key === '#') run(has('purge') ? 'purge' : 'delete');
  else if (e.key === '!') run(has('notspam') ? 'notspam' : 'spam');
  else if (e.key === 'v') run('file');
  else if (e.key === 'z') run('restore');
  else if (e.key === 'u') run(has('unsnooze') ? 'unsnooze' : 'unread');
  else if (e.key === 'c') openCompose();
  else if (e.key === 'r') run('reply');
  else if (e.key === 'm') run('move');
  else if (e.key === '/') { e.preventDefault(); $('q').focus(); }
});

// --- Classer dans un dossier personnel -----------------------------------------------------

function openFileDialog(m) {
  st.filing = m.id;
  const rule = st.folderRules[(m.addr || '').toLowerCase()];
  $('file-sub').textContent = `« ${m.subject} » — de ${m.from}${m.addr ? ' <' + m.addr + '>' : ''}`;
  const list = st.userFolders;
  $('file-folders').innerHTML = list.length
    ? list.map((f) => `<button type="button" class="chip ${rule === f.label ? 'on' : ''}" data-file="${esc(f.label)}" style="--c:${colorOf(f.label)}">${ICON.folder}<span>${esc(f.label)}</span></button>`).join('')
    : '<p class="dlg-hint">Aucun dossier pour l’instant : crée le premier ci-dessous.</p>';
  $('file-new').value = m.sent ? '' : m.from;
  $('file-rule').checked = !!rule || !m.sent;
  $('file-rule-label').textContent = m.addr ? `Toujours classer les mails de ${m.addr} dans ce dossier` : 'Toujours classer les mails de cette personne dans ce dossier';
  $('file-rule').disabled = !m.addr;
  $('file-dialog').showModal();
  $('file-new').select();
}

async function fileInto(label) {
  const m = byId(st.filing); $('file-dialog').close(); if (!m || !label) return;
  const key = (m.addr || '').toLowerCase();
  const always = $('file-rule').checked && key;
  if (always) st.folderRules[key] = label;
  else if (key && st.folderRules[key] === label) delete st.folderRules[key];
  const ok = await moveTo(m, userKey(label));
  if (!ok) return;
  if (always) {
    // Les autres mails de cette personne déjà dans la boîte de réception suivent le même chemin.
    const others = MESSAGES.filter((x) => x !== m && x.state === 'inbox' && (x.addr || '').toLowerCase() === key);
    for (const x of others) await moveTo(x, userKey(label), { quiet: true });
    toast(`Classé dans « ${label} » — les prochains mails de ${m.addr} iront directement ici` + (others.length ? ` (${others.length} autre${others.length > 1 ? 's' : ''} déplacé${others.length > 1 ? 's' : ''})` : ''));
    render();
  }
  saveCache();
}

$('file-create').addEventListener('click', async () => {
  const label = $('file-new').value.trim().replace(/^\/+|\/+$/g, '');
  if (!label) { $('file-new').focus(); return; }
  const m = byId(st.filing);
  if (!realMode || !m || !m.real) {
    if (!st.userFolders.some((f) => f.label.toLowerCase() === label.toLowerCase())) { st.userFolders.push({ label, accts: { [m ? m.acct : 'demo']: label } }); sortFolders(); }
  }
  fileInto((st.userFolders.find((f) => f.label.toLowerCase() === label.toLowerCase()) || { label }).label);
});
$('file-new').addEventListener('keydown', (e) => { if (e.key === 'Enter') { e.preventDefault(); $('file-create').click(); } });
$('file-cancel').addEventListener('click', () => $('file-dialog').close());

// Nouveau dossier depuis le volet de gauche
$('btn-new-folder').addEventListener('click', () => {
  const accts = ACCOUNTS;
  $('folder-acct').innerHTML = accts.map((a) => `<option value="${esc(a.id)}">${esc(a.name)}</option>`).join('');
  if (st.acct) $('folder-acct').value = st.acct;
  $('folder-acct').parentElement.querySelector('label[for="folder-acct"]').hidden = accts.length < 2;
  $('folder-acct').hidden = accts.length < 2;
  $('folder-name').value = '';
  $('folder-dialog').showModal(); $('folder-name').focus();
});
$('folder-cancel').addEventListener('click', () => $('folder-dialog').close());
$('folder-name').addEventListener('keydown', (e) => { if (e.key === 'Enter') { e.preventDefault(); $('folder-ok').click(); } });
$('folder-ok').addEventListener('click', async () => {
  const label = $('folder-name').value.trim().replace(/^\/+|\/+$/g, '');
  if (!label) { $('folder-name').focus(); return; }
  const accountId = $('folder-acct').value;
  $('folder-ok').disabled = true;
  try {
    if (realMode) await ensureFolder(accountId, label);
    else if (!st.userFolders.some((f) => f.label.toLowerCase() === label.toLowerCase())) { st.userFolders.push({ label, accts: { [accountId]: label } }); sortFolders(); }
    $('folder-dialog').close(); renderFolders(); toast('Dossier « ' + label + ' » créé'); openFolder(userKey(label));
  } catch (e) { showInfo('Création impossible', String(e)); }
  finally { $('folder-ok').disabled = false; }
});

function confirmBox(title, text, yes) {
  return new Promise((resolve) => {
    $('confirm-title').textContent = title; $('confirm-text').textContent = text; $('confirm-yes').textContent = yes || 'Confirmer';
    const done = (v) => { $('confirm').close(); $('confirm-yes').onclick = $('confirm-no').onclick = null; resolve(v); };
    $('confirm-yes').onclick = () => done(true); $('confirm-no').onclick = () => done(false);
    $('confirm').onclose = () => resolve(false);
    $('confirm').showModal();
  });
}

// Rédaction et envoi (SMTP via le backend)
function openCompose(reply) {
  const accts = realMode ? ACCOUNTS : [];
  $('c-from').innerHTML = accts.map((a) => `<option value="${esc(a.id)}">${esc(a.name)}</option>`).join('') || '<option value="">(aucun compte connecté)</option>';
  if (reply && reply.acct) $('c-from').value = reply.acct;
  $('c-to').value = reply ? (reply.addr || '') : '';
  $('c-subj').value = reply ? (/^re:/i.test(reply.subject) ? reply.subject : 'Re: ' + reply.subject) : '';
  const quoted = reply && reply.body ? '\n\n' + reply.from + ' a écrit :\n' + reply.body.split('\n').map((l) => '> ' + l).join('\n') : '';
  $('c-body').value = quoted;
  $('compose').showModal();
  (reply ? $('c-body') : $('c-to')).focus();
  if (reply) $('c-body').setSelectionRange(0, 0);
}
$('btn-compose').addEventListener('click', () => openCompose());
$('c-send').addEventListener('click', async () => {
  if (!hasBackend || !realMode || !$('c-from').value) { $('compose').close(); toast('Connecte d’abord un compte pour envoyer'); return; }
  $('c-send').disabled = true; $('c-send').textContent = 'Envoi…';
  try {
    await invoke('send_mail', { accountId: $('c-from').value, to: $('c-to').value, subject: $('c-subj').value, body: $('c-body').value });
    $('compose').close(); toast('Message envoyé'); st.loaded.sent = false;
  } catch (e) { showInfo('Envoi impossible', String(e)); }
  finally { $('c-send').disabled = false; $('c-send').textContent = 'Envoyer'; }
});

// Retrait d'un compte : deux clics (« × » puis « Confirmer ? »), le jeton est effacé du trousseau.
async function removeAccount(id) {
  if (st.removing !== id) { st.removing = id; renderAccounts(); setTimeout(() => { if (st.removing === id) { st.removing = null; renderAccounts(); } }, 3000); return; }
  st.removing = null;
  try { await invoke('remove_account', { id }); toast('Compte retiré'); await loadReal(); }
  catch (e) { showInfo('Retrait impossible', String(e)); }
}

$('btn-add-account').addEventListener('click', () => {
  if (!hasBackend) { toast('Disponible dans l’application installée'); return; }
  $('add-dialog').showModal();
});

function showInfo(title, text) {
  $('info-title').textContent = title; $('info-text').textContent = text; $('info').showModal();
}

function fmtTime(ts) {
  if (!ts) return '';
  const d = new Date(ts * 1000), now = new Date();
  return d.toDateString() === now.toDateString()
    ? d.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
    : d.toLocaleDateString('fr-FR', { day: '2-digit', month: '2-digit' });
}

// Charge les comptes enregistrés puis l'Inbox de chacun. Sans compte : on reste en démo.
async function loadReal() {
  if (!hasBackend) return;
  let accounts;
  try { accounts = await invoke('list_accounts'); } catch (e) { showInfo('Comptes illisibles', String(e)); return; }
  if (!accounts.length) {
    realMode = false; ACCOUNTS = DEMO_ACCOUNTS.slice(); MESSAGES = DEMO_MESSAGES.map((m) => ({ ...m })); st.userFolders = DEMO_FOLDERS.map((f) => ({ ...f }));
    st.sel = '1'; renderAccounts(); render();
    toast('Mode démo : « Ajouter un compte » pour connecter Gmail'); return; }
  realMode = true; st.loaded = {};
  ACCOUNTS = accounts.map((a, i) => ({ id: a.id, name: a.email, color: PALETTE[i % PALETTE.length] }));
  MESSAGES = []; st.sel = null; st.userFolders = [];
  await loadCache(new Set(ACCOUNTS.map((a) => a.id)));
  st.hadCache = MESSAGES.length > 0;
  renderAccounts(); render();
  toast(st.hadCache ? 'Mise à jour des mails…' : 'Chargement des mails…');
  loadFolderList();
  await refresh('startup');
}

// Dossiers personnels de chaque compte (libellés Gmail).
async function loadFolderList() {
  const results = await Promise.allSettled(ACCOUNTS.map((a) => invoke('list_folders', { accountId: a.id })));
  const map = new Map();
  results.forEach((r, i) => {
    if (r.status !== 'fulfilled') return;
    r.value.filter((f) => f.kind === 'user').forEach((f) => {
      const k = f.label.toLowerCase();
      if (!map.has(k)) map.set(k, { label: f.label, accts: {} });
      map.get(k).accts[ACCOUNTS[i].id] = f.name;
    });
  });
  // Les comptes injoignables gardent leurs dossiers connus.
  st.userFolders.forEach((f) => Object.entries(f.accts).forEach(([id, raw]) => {
    const i = ACCOUNTS.findIndex((a) => a.id === id);
    if (i >= 0 && results[i].status === 'fulfilled') return;
    const k = f.label.toLowerCase(); if (!map.has(k)) map.set(k, { label: f.label, accts: {} }); map.get(k).accts[id] = raw;
  }));
  st.userFolders = [...map.values()]; sortFolders();
  renderFolders(); saveCache();
}

// --- Cache local : la liste s'affiche tout de suite au lancement, puis se met à jour ---------

async function loadCache(ids) {
  try {
    const raw = await invoke('cache_load'); if (!raw) return;
    const c = JSON.parse(raw);
    st.rules = c.rules || {};
    st.folderRules = c.folderRules || {};
    st.userFolders = (c.folders || []).map((f) => ({ label: f.label, accts: Object.fromEntries(Object.entries(f.accts || {}).filter(([id]) => ids.has(id))) }))
      .filter((f) => Object.keys(f.accts).length);
    MESSAGES = (c.messages || []).filter((m) => ids.has(m.acct))
      .map((m) => ({ ...m, state: m.state === 'archive' && !m.mailbox ? 'inbox' : m.state, time: fmtTime(m.ts), body: null, real: true }));
    MESSAGES.sort((x, y) => y.ts - x.ts);
  } catch (_) { /* cache absent ou illisible : on repart de Gmail */ }
}

function saveCache() {
  if (!realMode || !hasBackend) return;
  clearTimeout(saveCache.t);
  saveCache.t = setTimeout(() => {
    const messages = MESSAGES.filter((m) => m.real && m.state !== 'deleted').slice(0, 2000).map((m) => ({
      id: m.id, uid: m.uid, acct: m.acct, mailbox: m.mailbox, from: m.from, addr: m.addr, subject: m.subject, cat: m.cat,
      unread: m.unread, ts: m.ts, sent: !!m.sent, pinned: !!m.pinned, state: m.state,
      snip: m.body ? m.body.replace(/\s+/g, ' ').slice(0, 160) : (m.snip || ''),
    }));
    invoke('cache_save', { data: JSON.stringify({ v: 2, rules: st.rules, folderRules: st.folderRules, folders: st.userFolders, messages }) }).catch(() => {});
  }, 800);
}

// --- Actualisation, dossiers et « Charger plus » -------------------------------------------

function toMsg(m, state) {
  const sent = state === 'sent';
  return { id: m.id, uid: m.uid, acct: m.acct, mailbox: m.mailbox, from: m.from, addr: m.addr, subject: m.subject, cat: m.cat,
    unread: !sent && m.unread, time: fmtTime(m.ts), ts: m.ts, body: null, real: true, sent, pinned: !!m.flagged, state };
}

// Ajoute les messages absents (même identifiant = déjà là) ; renvoie le nombre de nouveaux.
function addMessages(list) {
  const known = new Map(MESSAGES.map((m) => [m.id, m]));
  // Déjà connus (cache) : on reprend l'état Gmail (lu, étoile) et le tri actuel. Les Rappels restent des Rappels.
  list.forEach((m) => { const ex = known.get(m.id); if (ex) { ex.unread = m.unread; ex.pinned = m.pinned; ex.cat = m.cat; if (ex.state !== 'snoozed') ex.state = m.state; } });
  const fresh = list.filter((m) => !known.has(m.id));
  MESSAGES.push(...fresh);
  MESSAGES.sort((a, b) => b.ts - a.ts);
  return fresh.length;
}

// Remplace la première page d'un dossier d'un compte : retire du cache les messages qui n'y sont plus.
function mergePage(accountId, state, list) {
  const inState = (m) => m.real && m.acct === accountId && (m.state === state || (state === 'inbox' && m.state === 'snoozed'));
  if (list.length) {
    const ids = new Set(list.map((m) => m.id)), minUid = Math.min(...list.map((m) => m.uid)), box = list[0].mailbox;
    MESSAGES = MESSAGES.filter((m) => !(inState(m) && m.mailbox === box && m.uid >= minUid && !ids.has(m.id)));
  } else {
    MESSAGES = MESSAGES.filter((m) => !(inState(m) && m.state !== 'snoozed'));
  }
  return addMessages(list);
}

// Accès Gmail pour un dossier : { kind, name } par compte (null si le compte n'a pas ce dossier).
function folderRequest(k, accountId) {
  if (!isUserKey(k)) return { kind: k, name: null };
  const f = st.userFolders.find((x) => x.label === k.slice(2));
  return f && f.accts[accountId] ? { kind: 'user', name: f.accts[accountId] } : null;
}

async function loadFolder(k, mode) {
  st.loaded[k] = true;
  if (!mode) { st.loading = k; renderList(); }
  const jobs = ACCOUNTS.map((a) => ({ a, req: folderRequest(k, a.id) })).filter((j) => j.req);
  const results = await Promise.allSettled(jobs.map((j) => invoke('fetch_folder', { accountId: j.a.id, kind: j.req.kind, name: j.req.name, beforeUid: null })));
  const errors = []; let added = 0;
  results.forEach((r, i) => {
    if (r.status === 'rejected') { errors.push(jobs[i].a.name + ' : ' + r.reason); return; }
    added += mergePage(jobs[i].a.id, k, r.value.messages.map((m) => toMsg(m, k)));
  });
  if (st.loading === k) st.loading = null;
  if (errors.length) { st.loaded[k] = false; if (mode !== 'auto') showInfo((folderMeta(k) || {}).label + ' indisponible', errors.join('\n\n')); }
  if (st.sel && !byId(st.sel)) st.sel = null;
  saveCache();
  if (st.folder === k) render(); else renderFolders();
  return added;
}

// mode : 'manual' (bouton, F5), 'auto' (toutes les 3 min) ou 'startup' (lancement).
async function refresh(mode) {
  if (!realMode || st.refreshing) return;
  st.refreshing = true; $('btn-refresh').classList.add('spin');
  const accounts = ACCOUNTS.slice();
  const results = await Promise.allSettled(accounts.map((a) => invoke('fetch_folder', { accountId: a.id, kind: 'inbox', name: null, beforeUid: null })));
  let added = 0; const errors = [];
  results.forEach((r, i) => {
    if (r.status === 'rejected') { errors.push(accounts[i].name + ' : ' + r.reason); return; }
    added += mergePage(accounts[i].id, 'inbox', r.value.messages.map((m) => toMsg(m, 'inbox')));
  });
  // Le dossier affiché (s'il vient de Gmail) est aussi actualisé.
  if (st.folder !== 'inbox' && isServerFolder(st.folder) && mode !== 'startup') await loadFolder(st.folder, mode);
  st.refreshing = false; $('btn-refresh').classList.remove('spin');
  if (st.sel && !byId(st.sel)) st.sel = null;
  render(); saveCache();
  const filed = await applyFolderRules();
  const announce = mode === 'manual' || mode === 'auto' || (mode === 'startup' && st.hadCache);
  if (added && announce) toast((added === 1 ? '1 nouveau mail' : added + ' nouveaux mails') + (filed ? ` (${filed} classé${filed > 1 ? 's' : ''} automatiquement)` : ''));
  else if (filed) toast(`${filed} mail${filed > 1 ? 's' : ''} classé${filed > 1 ? 's' : ''} automatiquement`);
  else if (mode === 'manual') toast(errors.length ? 'Actualisation impossible' : 'Aucun nouveau mail');
  if (errors.length && mode !== 'auto') showInfo(mode === 'startup' ? 'Certains comptes n’ont pas pu être chargés' : 'Actualisation impossible', errors.join('\n\n'));
}

// « Toujours classer les mails de X ici » : les nouveaux mails reçus sont rangés dans Gmail.
async function applyFolderRules() {
  const todo = MESSAGES.filter((m) => m.real && m.state === 'inbox' && st.folderRules[(m.addr || '').toLowerCase()]).slice(0, 25);
  let n = 0;
  for (const m of todo) if (await moveTo(m, userKey(st.folderRules[m.addr.toLowerCase()]), { quiet: true })) n++;
  if (n) { if (st.sel && !byId(st.sel)) st.sel = null; render(); saveCache(); }
  return n;
}

async function loadOlder() {
  if (st.loadingMore) return;
  const k = st.folder;
  const jobs = ACCOUNTS.filter((a) => !st.acct || a.id === st.acct).map((a) => {
    const req = folderRequest(k, a.id); if (!req) return null;
    const mine = MESSAGES.filter((m) => m.real && m.acct === a.id && m.state === k);
    if (!mine.length) return null;
    const oldest = Math.min(...mine.map((m) => m.uid));
    return { a, args: { accountId: a.id, kind: req.kind, name: req.name, beforeUid: oldest } };
  }).filter(Boolean);
  if (!jobs.length) { toast('Aucun mail plus ancien'); return; }
  st.loadingMore = true; renderList();
  const results = await Promise.allSettled(jobs.map((j) => invoke('fetch_folder', j.args)));
  let added = 0; const errors = [];
  results.forEach((r, i) => {
    if (r.status === 'rejected') { errors.push(jobs[i].a.name + ' : ' + r.reason); return; }
    added += addMessages(r.value.messages.map((m) => toMsg(m, k)));
  });
  st.loadingMore = false; renderFolders(); renderList(); saveCache();
  toast(added ? added + ' mails plus anciens chargés' : 'Aucun mail plus ancien');
  if (errors.length) showInfo('Chargement impossible', errors.join('\n\n'));
}

$('btn-refresh').addEventListener('click', () => refresh('manual'));
document.addEventListener('click', (e) => { if (e.target.closest('#btn-more')) loadOlder(); });
document.addEventListener('keydown', (e) => { if (e.key === 'F5') { e.preventDefault(); refresh('manual'); } });
setInterval(() => refresh('auto'), 3 * 60 * 1000); // toutes les 3 minutes

$('add-cancel').addEventListener('click', () => $('add-dialog').close());
$('add-dialog').addEventListener('click', async (e) => {
  const provider = e.target.dataset && e.target.dataset.provider;
  if (!provider) return;
  $('add-dialog').close();
  toast('Le navigateur s’ouvre : connecte-toi…');
  try {
    await invoke('add_account', { provider });
    await loadReal();
  } catch (err) { showInfo('Ajout du compte impossible', String(err)); }
});
$('info-close').addEventListener('click', () => $('info').close());

// Thème : suit le système par défaut, bascule manuelle ensuite
$('btn-theme').addEventListener('click', () => {
  const root = document.documentElement;
  const dark = root.dataset.theme ? root.dataset.theme === 'dark' : matchMedia('(prefers-color-scheme: dark)').matches;
  root.dataset.theme = dark ? 'light' : 'dark';
});

(async () => {
  try {
    const info = await window.__TAURI__.core.invoke('app_info');
    $('app-version').textContent = `v${info.version}`;
  } catch (_) { $('app-version').textContent = 'Aperçu web'; }
})();

renderAccounts();
render();
loadReal();
