// SXM Mail — interface. Sans compte : données de démonstration. Avec compte Gmail : vrais mails (étape 2).
'use strict';

const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);
const hasBackend = !!(window.__TAURI__ && window.__TAURI__.core);
const PALETTE = ['#2f6fed', '#d93025', '#0f8a6a', '#7c3aed', '#c2410c', '#0e7490'];
let realMode = false;

let ACCOUNTS = [
  { id: 'o365', name: 'Office 365', color: '#2f6fed' },
  { id: 'gmail', name: 'Gmail', color: '#d93025' },
  { id: 'hotmail', name: 'Hotmail', color: '#0f8a6a' },
];

// category : people | notifications | newsletters
let MESSAGES = [
  { id: '1', from: 'Marie Laurent', acct: 'o365', cat: 'people', time: '14:12', unread: true, pinned: false, state: 'inbox', av: '#2f6fed',
    subject: 'Réunion de mardi', body: 'Bonjour,\n\nPeux-tu confirmer ta présence pour la réunion de mardi à 10h ? Il faudra prévoir un point sur le réseau.\n\nMerci,\nMarie' },
  { id: '2', from: 'Jean Petit', acct: 'gmail', cat: 'people', time: '11:40', unread: true, pinned: false, state: 'inbox', av: '#0f8a6a',
    subject: 'Photos du week-end', body: 'Salut !\n\nJe t’envoie les photos comme promis. Dis-moi si tu les reçois bien.\n\nJean' },
  { id: '3', from: 'Banque', acct: 'hotmail', cat: 'notifications', time: '09:05', unread: false, pinned: false, state: 'inbox', av: '#7c3aed',
    subject: 'Votre relevé est disponible', body: 'Votre relevé mensuel est disponible dans votre espace client.' },
  { id: '4', from: 'Fournisseur réseau', acct: 'o365', cat: 'notifications', time: 'Hier', unread: false, pinned: true, state: 'inbox', av: '#c2410c',
    subject: 'Commande expédiée', body: 'Votre commande a été expédiée.\nNuméro de suivi : [NUMÉRO].' },
  { id: '5', from: 'Lettre tech', acct: 'gmail', cat: 'newsletters', time: 'Hier', unread: false, pinned: false, state: 'inbox', av: '#5f6673',
    subject: 'Les nouveautés Linux de la semaine', body: 'Au programme cette semaine : Fedora, GNOME et le noyau Linux.' },
];

// Copie de la démo, restaurée si le dernier compte est retiré.
const DEMO_ACCOUNTS = ACCOUNTS.slice();
const DEMO_MESSAGES = MESSAGES.map((m) => ({ ...m }));

const FOLDERS = [
  { id: 'inbox', label: 'Boîte de réception' },
  { id: 'pinned', label: 'Épinglés' },
  { id: 'snoozed', label: 'Rappels' },
  { id: 'sent', label: 'Éléments envoyés' },
];
const SECTIONS = [
  { id: 'people', label: 'Personnes' },
  { id: 'notifications', label: 'Notifications' },
  { id: 'newsletters', label: 'Newsletters' },
];

const st = { folder: 'inbox', tab: 'focused', sel: '1', q: '', collapsed: {}, rules: {} };

// Onglets de la boîte de réception : « Prioritaire » = personnes, « Autres » = notifications et newsletters.
// Une règle choisie par l'utilisateur (« Déplacer vers… ») l'emporte sur le tri automatique.
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
  delete: svg('<path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="M19 6l-1 14H6L5 6"/>'),
};
const $ = (id) => document.getElementById(id);
const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const initials = (n) => n.split(/\s+/).slice(0, 2).map((w) => w[0]).join('').toUpperCase();
const acct = (id) => ACCOUNTS.find((a) => a.id === id);
const byId = (id) => MESSAGES.find((m) => m.id === id);

function visible() {
  const q = st.q.trim().toLowerCase();
  return MESSAGES.filter((m) => {
    if (m.state === 'deleted') return false;
    if (st.acct && m.acct !== st.acct) return false;
    if (st.folder === 'pinned' ? !m.pinned : m.state !== st.folder) return false;
    if (st.folder === 'inbox' && !q && tabOf(m) !== st.tab) return false;
    return !q || (m.from + ' ' + m.subject + ' ' + (m.body || '')).toLowerCase().includes(q);
  });
}

function toast(msg) {
  const t = $('toast'); t.textContent = msg; t.classList.add('show');
  clearTimeout(toast.t); toast.t = setTimeout(() => t.classList.remove('show'), 2400);
}

function msgButton(m) {
  return `<button type="button" class="msg ${m.unread ? 'unread' : ''} ${m.id === st.sel ? 'on' : ''}" data-id="${m.id}">
    <span class="avatar" style="background:${m.av}" aria-hidden="true">${esc(initials(m.from))}</span>
    <span class="msg-body">
      <span class="row"><span>${m.sent ? 'À : ' : ''}${esc(m.from)}</span><span class="time">${m.pinned ? '<span class="pin-mark" title="Épinglé">' + ICON.pin + '</span> ' : ''}${esc(m.time)}</span></span>
      <span class="subj">${esc(m.subject)}</span>
      <span class="snip">${esc((m.body || m.snip || '').replace(/\s+/g, ' '))}</span>
    </span></button>`;
}

function renderFolders() {
  const pool = MESSAGES.filter((m) => !st.acct || m.acct === st.acct);
  $('folders').innerHTML = FOLDERS.map((f) => {
    const n = f.id === 'inbox' ? pool.filter((m) => m.state === 'inbox' && m.unread).length
      : f.id === 'pinned' ? pool.filter((m) => m.pinned).length
      : pool.filter((m) => m.state === f.id).length;
    return `<button type="button" class="folder ${f.id === st.folder ? 'on' : ''}" data-folder="${f.id}"><span>${f.label}</span><span class="count">${n || ''}</span></button>`;
  }).join('');
}

function renderAccounts() {
  $('accounts').innerHTML = ACCOUNTS.map((a) => `<div class="account ${st.acct === a.id ? 'on' : ''}"><span class="dot" style="background:${a.color}"></span><button type="button" class="acc-name" data-acct="${esc(a.id)}" aria-pressed="${st.acct === a.id}" title="Afficher seulement ce compte (cliquer à nouveau pour tout afficher)">${esc(a.name)}</button>${realMode ? `<button type="button" class="rm" data-rm="${esc(a.id)}" aria-label="Retirer ${esc(a.name)}">${st.removing === a.id ? 'Confirmer ?' : '×'}</button>` : ''}</div>`).join('');
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
  if (st.folder !== 'inbox' || st.q.trim()) { $('tabs').innerHTML = ''; return; }
  const pool = MESSAGES.filter((m) => m.state === 'inbox' && (!st.acct || m.acct === st.acct));
  $('tabs').innerHTML = [['focused', 'Prioritaire'], ['other', 'Autres']].map(([id, label]) => {
    const n = pool.filter((m) => tabOf(m) === id && m.unread).length;
    return `<button type="button" class="tab ${st.tab === id ? 'on' : ''}" data-tab="${id}" aria-pressed="${st.tab === id}">${label}${n ? ` <span class="count">${n}</span>` : ''}</button>`;
  }).join('');
}

function renderList() {
  renderTabs();
  const list = visible();
  const more = realMode && !st.q && (st.folder === 'inbox' || st.folder === 'sent')
    ? `<button type="button" class="btn-more" id="btn-more" ${st.loadingMore ? 'disabled' : ''}>${st.loadingMore ? 'Chargement…' : 'Charger les mails plus anciens'}</button>` : '';
  if (!list.length) { $('messages').innerHTML = '<div class="empty">Aucun message ici.</div>' + more; return; }
  let html = '', group = null;
  for (const m of list) {
    const g = dayLabel(m.ts);
    if (g !== group) { html += `<div class="group-head">${esc(g)}</div>`; group = g; }
    html += msgButton(m);
  }
  $('messages').innerHTML = html + more;
}

function renderReader() {
  const m = byId(st.sel);
  const bar = $('toolbar');
  if (!m) { bar.innerHTML = ''; $('reading').innerHTML = '<div class="empty">Sélectionne un message.</div>'; return; }
  const actions = [['reply', 'Répondre', 'R'], ['delete', 'Supprimer', '#'], ['archive', 'Archiver', 'E'], ['pin', m.pinned ? 'Désépingler' : 'Épingler', 'S'], ['snooze', 'Rappeler plus tard', 'H']];
  if (m.state === 'inbox') actions.push(['move', tabOf(m) === 'focused' ? 'Déplacer vers Autres' : 'Déplacer vers Prioritaire', 'M']);
  bar.innerHTML = actions
    .map(([a, l, k]) => `<button type="button" data-act="${a}" title="${l} (${k})">${ICON[a]}<span>${l}</span></button>`).join('');
  $('reading').innerHTML = `<h1>${esc(m.subject)}</h1>
    <div class="who"><span class="avatar lg" style="background:${m.av}" aria-hidden="true">${esc(initials(m.from))}</span>
      <div class="who-text"><div class="name">${m.sent ? 'À : ' : ''}${esc(m.from)}${m.addr ? ` <span class="addr">&lt;${esc(m.addr)}&gt;</span>` : ''}</div><div class="meta">${m.sent ? 'Envoyé depuis' : 'À'} : ${esc((acct(m.acct) || {}).name || 'moi')}</div></div><div class="when">${esc(m.time)}</div><div class="quick"><button type="button" data-act="reply" aria-label="Répondre" title="Répondre (R)">${ICON.reply}</button></div></div>
    ${m.atts && m.atts.length ? '<div class="att-bar">' + m.atts.map((a) => `<button type="button" class="att" data-att="${a.index}" title="Enregistrer dans Téléchargements">${ICON.clip}<span class="att-name">${esc(a.filename)}</span><span class="att-size">${fmtSize(a.size)}</span></button>`).join('') + '</div>' : ''}
    ${m.body == null ? '<p class="body">Chargement…</p>'
      : m.html ? (m.remote && !m.showImages ? '<div class="img-bar"><span>Images distantes bloquées (protection contre le pistage).</span><button type="button" data-act="images">Afficher les images</button></div>' : '') + '<div class="html-slot" id="html-slot"></div>'
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
    + '<style>html,body{margin:0;background:#fff;color:#1a1d23;font-family:system-ui,sans-serif;font-size:15px;line-height:1.6;overflow-wrap:anywhere}body{padding:20px 4px}'+ '*{text-align:left!important}table,div,center{margin-left:0!important}table{max-width:100%!important}table[width]{width:100%!important}p{margin:0 0 14px}img{max-width:100%;height:auto}</style>'
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

async function act(a) {
  const m = byId(st.sel); if (!m) return;
  if (a === 'reply') { openCompose(m); return; }
  if (a === 'images') { m.showImages = true; renderReader(); return; }
  const gmail = (action) => invoke('message_action', { accountId: m.acct, mailbox: m.mailbox || 'INBOX', uid: m.uid, action });
  if (a === 'move') {
    const key = (m.addr || '').toLowerCase(); if (!key) return;
    const to = tabOf(m) === 'focused' ? 'other' : 'focused';
    st.rules[key] = to; st.sel = null; render(); saveCache();
    toast('Les mails de ' + m.addr + ' iront dans « ' + (to === 'focused' ? 'Prioritaire' : 'Autres') + ' »');
    return;
  }
  if (a === 'pin') {
    const before = m.pinned; m.pinned = !before; render();
    if (m.real) {
      try { await gmail(m.pinned ? 'star' : 'unstar'); }
      catch (e) { m.pinned = before; render(); showInfo('Épinglage impossible', String(e)); return; }
    }
    toast(m.pinned ? 'Épinglé (étoile dans Gmail)' : 'Désépinglé'); saveCache(); return;
  }
  if (a === 'snooze') {
    const nxt = nextAfter(m.id); m.state = 'snoozed'; st.sel = null; render(); saveCache();
    toast('Déplacé dans Rappels (sur cet ordinateur uniquement)'); if (nxt) select(nxt); return;
  }
  if (a !== 'delete' && a !== 'archive') return;
  const before = m.state, nxt = nextAfter(m.id);
  m.state = a === 'delete' ? 'deleted' : 'archive'; st.sel = null; render();
  if (m.real) {
    try { await gmail(a); }
    catch (e) { m.state = before; render(); showInfo(a === 'delete' ? 'Suppression impossible' : 'Archivage impossible', String(e)); return; }
    MESSAGES = MESSAGES.filter((x) => x !== m); // n'existe plus dans ce dossier côté Gmail
  }
  toast(a === 'delete' ? 'Placé dans la corbeille Gmail' : 'Archivé dans Gmail (Tous les messages)');
  saveCache(); if (nxt) select(nxt); else render();
}

document.addEventListener('click', (e) => {
  const t = e.target.closest('[data-id],[data-folder],[data-section],[data-act],[data-rm],[data-acct],[data-tab],[data-att]');
  if (!t) return;
  if (t.dataset.att != null) saveAttachment(Number(t.dataset.att));
  else if (t.dataset.rm) removeAccount(t.dataset.rm);
  else if (t.dataset.tab) { st.tab = t.dataset.tab; st.sel = null; render(); }
  else if (t.dataset.acct) { st.acct = st.acct === t.dataset.acct ? null : t.dataset.acct; st.sel = null; renderAccounts(); render(); }
  else if (t.dataset.id) select(t.dataset.id);
  else if (t.dataset.folder === 'sent' && realMode && !st.sentLoaded) { st.folder = 'sent'; render(); loadSent(); }
  else if (t.dataset.folder) { if (t.dataset.folder === 'inbox' && st.folder === 'inbox') { st.acct = null; renderAccounts(); } st.folder = t.dataset.folder; st.sel = null; render(); }
  else if (t.dataset.section) { st.collapsed[t.dataset.section] = !st.collapsed[t.dataset.section]; renderList(); }
  else if (t.dataset.act) act(t.dataset.act);
});

$('q').addEventListener('input', (e) => { st.q = e.target.value; renderList(); });

// Raccourcis clavier (hors champs de saisie)
document.addEventListener('keydown', (e) => {
  if (e.target.closest('input,textarea,dialog') || e.ctrlKey || e.metaKey || e.altKey) return;
  const l = visible(); const i = l.findIndex((m) => m.id === st.sel);
  if (e.key === 'j' && l[i + 1]) select(l[i + 1].id);
  else if (e.key === 'k' && l[i - 1]) select(l[i - 1].id);
  else if (e.key === 'e') act('archive');
  else if (e.key === 's') act('pin');
  else if (e.key === 'h') act('snooze');
  else if (e.key === '#') act('delete');
  else if (e.key === 'c') openCompose();
  else if (e.key === 'r') act('reply');
  else if (e.key === 'm') act('move');
  else if (e.key === '/') { e.preventDefault(); $('q').focus(); }
});

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
    $('compose').close(); toast('Message envoyé');
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
// Messages envoyés : chargés depuis Gmail au premier affichage du dossier.
async function loadSent() {
  st.sentLoaded = true;
  $('messages').innerHTML = '<div class="empty">Chargement des messages envoyés…</div>';
  const accounts = ACCOUNTS.slice();
  const results = await Promise.allSettled(accounts.map((a) => invoke('fetch_sent', { accountId: a.id })));
  const errors = [];
  results.forEach((r, i) => {
    if (r.status === 'rejected') { errors.push(accounts[i].name + ' : ' + r.reason); return; }
    addMessages(r.value.map((m) => toMsg(m, accounts[i].color, true)));
  });
  if (errors.length) { st.sentLoaded = false; showInfo('Messages envoyés indisponibles', errors.join('\n\n')); }
  saveCache();
  if (st.folder === 'sent') { const l = visible(); st.sel = (l[0] || {}).id; if (st.sel) select(st.sel); else render(); }
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
    realMode = false; ACCOUNTS = DEMO_ACCOUNTS.slice(); MESSAGES = DEMO_MESSAGES.map((m) => ({ ...m })); st.sel = '1'; renderAccounts(); render();
    toast('Mode démo : « Ajouter un compte » pour connecter Gmail'); return; }
  realMode = true; st.sentLoaded = false;
  ACCOUNTS = accounts.map((a, i) => ({ id: a.id, name: a.email, color: PALETTE[i % PALETTE.length] }));
  MESSAGES = []; st.sel = null;
  await loadCache(new Set(ACCOUNTS.map((a) => a.id)));
  st.hadCache = MESSAGES.length > 0;
  renderAccounts(); render();
  toast(st.hadCache ? 'Mise à jour des mails…' : 'Chargement des mails…');
  await refresh('startup');
}

// --- Cache local : la liste s'affiche tout de suite au lancement, puis se met à jour ---------

async function loadCache(ids) {
  try {
    const raw = await invoke('cache_load'); if (!raw) return;
    const c = JSON.parse(raw);
    st.rules = c.rules || {};
    MESSAGES = (c.messages || []).filter((m) => ids.has(m.acct))
      .map((m) => ({ ...m, time: fmtTime(m.ts), body: null, real: true, av: (acct(m.acct) || {}).color || PALETTE[0] }));
    MESSAGES.sort((x, y) => y.ts - x.ts);
  } catch (_) { /* cache absent ou illisible : on repart de Gmail */ }
}

function saveCache() {
  if (!realMode || !hasBackend) return;
  clearTimeout(saveCache.t);
  saveCache.t = setTimeout(() => {
    const messages = MESSAGES.filter((m) => m.real && m.state !== 'deleted').slice(0, 1500).map((m) => ({
      id: m.id, uid: m.uid, acct: m.acct, mailbox: m.mailbox, from: m.from, addr: m.addr, subject: m.subject, cat: m.cat,
      unread: m.unread, ts: m.ts, sent: !!m.sent, pinned: !!m.pinned, state: m.state,
      snip: m.body ? m.body.replace(/\s+/g, ' ').slice(0, 160) : (m.snip || ''),
    }));
    invoke('cache_save', { data: JSON.stringify({ v: 1, rules: st.rules, messages }) }).catch(() => {});
  }, 800);
}

// --- Actualisation et « Charger plus » -------------------------------------------------

function toMsg(m, color, sent) {
  return { id: m.id, uid: m.uid, acct: m.acct, mailbox: m.mailbox, from: m.from, addr: m.addr, subject: m.subject, cat: m.cat,
    unread: !sent && m.unread, time: fmtTime(m.ts), ts: m.ts, body: null, real: true, sent: !!sent, pinned: !!m.flagged,
    state: sent ? 'sent' : 'inbox', av: color };
}

// Ajoute les messages absents (même identifiant = déjà là) ; renvoie le nombre de nouveaux.
function addMessages(list) {
  const known = new Map(MESSAGES.map((m) => [m.id, m]));
  // Déjà connus (cache) : on reprend l'état Gmail (lu, étoile) et le tri actuel.
  list.forEach((m) => { const ex = known.get(m.id); if (ex) { ex.unread = m.unread; ex.pinned = m.pinned; ex.cat = m.cat; } });
  const fresh = list.filter((m) => !known.has(m.id));
  MESSAGES.push(...fresh);
  MESSAGES.sort((a, b) => b.ts - a.ts);
  return fresh.length;
}

// mode : 'manual' (bouton, F5), 'auto' (toutes les 3 min) ou 'startup' (lancement).
async function refresh(mode) {
  if (!realMode || st.refreshing) return;
  st.refreshing = true; $('btn-refresh').classList.add('spin');
  const accounts = ACCOUNTS.slice();
  const results = await Promise.allSettled(accounts.map((a) => invoke('fetch_inbox', { accountId: a.id })));
  let added = 0; const errors = [];
  results.forEach((r, i) => {
    if (r.status === 'rejected') { errors.push(accounts[i].name + ' : ' + r.reason); return; }
    const fresh = r.value.map((m) => toMsg(m, accounts[i].color, false));
    // Messages du cache supprimés ou archivés depuis Gmail (dans la fenêtre des derniers reçus) : on les retire.
    if (fresh.length) {
      const ids = new Set(fresh.map((m) => m.id)), minUid = Math.min(...fresh.map((m) => m.uid));
      MESSAGES = MESSAGES.filter((m) => !(m.real && m.acct === accounts[i].id && m.state === 'inbox' && (m.mailbox || 'INBOX') === 'INBOX' && m.uid >= minUid && !ids.has(m.id)));
    }
    added += addMessages(fresh);
  });
  st.refreshing = false; $('btn-refresh').classList.remove('spin');
  if (st.sel && !byId(st.sel)) st.sel = null;
  render(); saveCache();
  const announce = mode === 'manual' || mode === 'auto' || (mode === 'startup' && st.hadCache);
  if (added && announce) toast(added === 1 ? '1 nouveau mail' : added + ' nouveaux mails');
  else if (mode === 'manual') toast(errors.length ? 'Actualisation impossible' : 'Aucun nouveau mail');
  if (errors.length && mode !== 'auto') showInfo(mode === 'startup' ? 'Certains comptes n’ont pas pu être chargés' : 'Actualisation impossible', errors.join('\n\n'));
}

async function loadOlder() {
  if (st.loadingMore) return;
  const sent = st.folder === 'sent';
  const jobs = ACCOUNTS.filter((a) => !st.acct || a.id === st.acct).map((a) => {
    const mine = MESSAGES.filter((m) => m.real && m.acct === a.id && !!m.sent === sent);
    if (!mine.length) return null;
    const oldest = Math.min(...mine.map((m) => m.uid));
    return { a, args: { accountId: a.id, mailbox: mine[0].mailbox, beforeUid: oldest, sent } };
  }).filter(Boolean);
  if (!jobs.length) return;
  st.loadingMore = true; renderList();
  const results = await Promise.allSettled(jobs.map((j) => invoke('fetch_older', j.args)));
  let added = 0; const errors = [];
  results.forEach((r, i) => {
    if (r.status === 'rejected') { errors.push(jobs[i].a.name + ' : ' + r.reason); return; }
    added += addMessages(r.value.map((m) => toMsg(m, jobs[i].a.color, sent)));
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

// Test IPC avec le moteur Rust
(async () => {
  try {
    const info = await window.__TAURI__.core.invoke('app_info');
    $('app-version').textContent = `${info.name} ${info.version}`;
  } catch (_) { $('app-version').textContent = 'SXM Mail (aperçu web)'; }
})();

renderAccounts();
render();
loadReal();
