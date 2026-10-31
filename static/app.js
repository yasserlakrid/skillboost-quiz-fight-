const $ = s => document.querySelector(s), app = $('#app');
const esc = s => String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
const ICON = { 'AI': '🤖', 'Backend dev': '⚙️', 'Security': '🛡️', 'Graphic design': '🎨', 'Motion design': '🎞️', 'Filmmaking': '🎬', 'Blender': '🧊' };
let ws, role, raf, st = { players: [] };

const send = o => ws && ws.readyState === 1 && ws.send(JSON.stringify(o));
function note(msg) { const t = $('#toast'); t.textContent = msg; t.classList.add('on'); clearTimeout(note.h); note.h = setTimeout(() => t.classList.remove('on'), 3200); }

function open(first) {
  ws = new WebSocket((location.protocol === 'https:' ? 'wss://' : 'ws://') + location.host + '/ws');
  ws.onopen = () => send(first);
  ws.onmessage = e => on(JSON.parse(e.data));
  ws.onclose = () => { if (st.code) note('Connection lost. Refresh to rejoin.'); };
}

/* ---------- views ---------- */
function home() {
  const pre = location.hash.replace(/\D/g, '').slice(0, 4);
  app.innerHTML = `<section class="hero"><h1>Reflex Arena</h1>
    <p class="sub">Three emoji clues, three answers. The faster you pick the right one, the more you score.</p>
    <div class="glass card"><button class="btn" id="host">Host a game</button>
      <div class="or">or join with a room code</div>
      <div class="row"><input id="code" inputmode="numeric" maxlength="4" placeholder="Room code" value="${pre}"><input id="name" maxlength="14" placeholder="Your name"></div>
      <button class="btn ghost" id="join">Join game</button></div></section>`;
  $('#host').onclick = subjects;
  $('#join').onclick = () => {
    const code = $('#code').value.trim(), name = $('#name').value.trim();
    if (!code || !name) return note('Enter a room code and your name');
    role = 'player'; st.name = name; open({ t: 'join', code, name });
  };
}

function subjects() {
  app.innerHTML = `<section class="hero"><h1>Pick a subject</h1>
    <p class="sub">Every question in this room comes from the subject you choose.</p>
    <div class="subjects">${Object.entries(ICON).map(([n, i]) => `<button class="glass subj" data-s="${esc(n)}"><span>${i}</span>${esc(n)}</button>`).join('')}</div>
    <button class="btn ghost back" id="back">Back</button></section>`;
  $('#back').onclick = home;
  document.querySelectorAll('.subj').forEach(b => b.onclick = () => { role = 'host'; open({ t: 'host', topic: b.dataset.s }); });
}

function lobby() {
  const host = role === 'host';
  const chips = st.players.map(n => `<span class="chip">${esc(n)}</span>`).join('') || '<span class="muted">No players yet</span>';
  app.innerHTML = `<section class="hero"><h1>${host ? 'Join the game' : 'You’re in'}</h1>
    <div class="topic"><span>${ICON[st.topic] || '💡'}</span>${esc(st.topic)}</div>
    <div class="glass card wide"><div class="muted">${host ? 'Open ' + esc(location.host) + ' on your phone and enter' : 'Room code'}</div>
    <div class="code">${st.code}</div><div class="chips">${chips}</div>
    ${host ? `<button class="btn" id="start" ${st.players.length ? '' : 'disabled'}>Start game</button>` : '<div class="muted">Waiting for the host to start…</div>'}</div></section>`;
  if (host) $('#start').onclick = () => send({ t: 'start' });
}

function qv() {
  const q = st.q, host = role === 'host';
  app.innerHTML = `<section class="game ${host ? 'hostv' : ''}"><div class="topic"><span>${ICON[q.topic] || '💡'}</span>${esc(q.topic)}<em>${q.i}/${q.n}</em></div>
    <div class="stage"><div class="glass main"><div class="top"><div class="clock" id="clock"><i></i></div>
    <div class="hint">${host ? `<b id="cnt">0/${st.players.length}</b> answered` : 'Pick one'}</div></div>
    <div class="clues" id="clues"></div><div class="opts" id="opts"></div></div>
    ${host ? '<aside class="glass board" id="board"></aside>' : ''}</div></section>`;
  cl(); ops(); board();
}
const cl = () => { $('#clues').innerHTML = st.q.clues.map(c => `<div class="clue ${c ? 'on' : ''}">${c || '?'}</div>`).join(''); };

function ops() {
  const q = st.q, r = st.rv, host = role === 'host';
  $('#opts').innerHTML = q.opts.map((o, i) => {
    let c = '';
    if (r) c = i === r.answer ? 'ok' : 'dim' + (i === q.picked ? ' bad' : '');
    else if (i === q.picked) c = 'sel';
    return `<button class="opt ${c}" data-pick="${i}" ${host || q.picked != null || r ? 'disabled' : ''}><b>${'ABC'[i]}</b>${esc(o)}${r ? `<small>${r.counts[i]}</small>` : ''}</button>`;
  }).join('');
}

function board() {
  const b = $('#board'); if (!b) return;
  b.innerHTML = '<h3>Leaderboard</h3>' + st.scores.slice(0, 8).map(x => `<div>${esc(x.n)}<span>${x.s}</span>${x.g ? `<em>+${x.g}</em>` : '<em></em>'}</div>`).join('');
}

function hint() {
  const h = $('.hint'), r = st.rv; if (!h) return;
  h.innerHTML = role === 'host' ? 'Next question coming up'
    : r.you.none ? 'Too slow, no answer' : r.you.ok ? `Correct <b>+${r.you.gain}</b>` : 'Not this time';
}

function tick() {
  cancelAnimationFrame(raf);
  const q = st.q, go = () => {
    const el = $('#clock'); if (!el || st.rv) return;
    el.style.setProperty('--p', Math.max(0, 100 * (1 - (performance.now() - q.t0) / q.dur)));
    raf = requestAnimationFrame(go);
  };
  go();
}

function endv() {
  const host = role === 'host', s = st.end.scores;
  app.innerHTML = `<section class="hero"><h1>${host ? 'Final scores' : 'Game over'}</h1><div class="glass card wide">
    <ol class="rank">${s.map((x, i) => `<li class="${i === 0 ? 'first' : ''} ${x.n === st.name ? 'me' : ''}"><span>${i + 1}</span><b>${esc(x.n)}</b><em>${x.s}</em></li>`).join('')}</ol>
    ${host ? '<button class="btn" id="again">Play again</button>' : '<div class="muted">Waiting for the host to start another game…</div>'}</div></section>`;
  if (host) $('#again').onclick = () => send({ t: 'start' });
}

/* ---------- server messages ---------- */
function on(m) {
  switch (m.t) {
    case 'lobby': st.code = m.code; st.topic = m.topic; st.players = m.players; lobby(); break;
    case 'err': note(m.msg); if (!st.code) { role = null; ws.close(); } break;
    case 'q': st.q = { ...m, clues: [null, null, null], picked: null, t0: performance.now() }; st.rv = null; st.scores = m.scores; qv(); tick(); break;
    case 'clue': if (st.q) { st.q.clues[m.idx] = m.e; cl(); } break;
    case 'count': { const c = $('#cnt'); if (c) c.textContent = `${m.n}/${m.total}`; break; }
    case 'reveal': st.rv = m; st.scores = m.scores; ops(); board(); hint(); break;
    case 'end': st.end = m; endv(); break;
  }
}

app.addEventListener('click', e => {
  const b = e.target.closest('[data-pick]');
  if (!b || role !== 'player' || !st.q || st.q.picked != null || st.rv) return;
  st.q.picked = +b.dataset.pick;
  send({ t: 'pick', c: st.q.picked });
  ops(); $('.hint').textContent = 'Locked in';
});

home();
