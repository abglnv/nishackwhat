// ── NiShack Frontend ────────────────────────────────────
const API = '';                          // same origin
let ws = null;
let selectedStudent = null;
let allStudents = [];
let refreshTimer = null;

// ── DOM refs ────────────────────────────────────────────
const $  = (s) => document.querySelector(s);
const $$ = (s) => document.querySelectorAll(s);

const dom = {
  pillOnline:   $('#pillOnline'),
  pillTotal:    $('#pillTotal'),
  wsStatus:     $('#wsStatus'),
  filterSelect: $('#filterSelect'),
  studentList:  $('#studentList'),
  noSelection:  $('#noSelection'),
  detail:       $('#detail'),
  // detail header
  hostname:     $('#detailHostname'),
  status:       $('#detailStatus'),
  os:           $('#detailOS'),
  user:         $('#detailUser'),
  ip:           $('#detailIP'),
  // bars
  barCPU:       $('#barCPU'),
  lblCPU:       $('#lblCPU'),
  barRAM:       $('#barRAM'),
  lblRAM:       $('#lblRAM'),
  // sections
  screenshotContainer: $('#screenshotContainer'),
  appsBody:     $('#appsTable tbody'),
  tabsBody:     $('#tabsTable tbody'),
  violCount:    $('#violCount'),
  violList:     $('#violList'),
  notifList:    $('#notifList'),
  // chat
  chatTo:       $('#chatTo'),
  chatMessages: $('#chatMessages'),
  chatForm:     $('#chatForm'),
  chatInput:    $('#chatInput'),
};

// ── Init ────────────────────────────────────────────────
document.addEventListener('DOMContentLoaded', () => {
  connectWS();
  refreshStudents();
  refreshTimer = setInterval(refreshStudents, 5000);

  dom.filterSelect.addEventListener('change', renderStudentList);
  dom.chatForm.addEventListener('submit', sendChat);
});

// ── WebSocket ───────────────────────────────────────────
function connectWS() {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  ws = new WebSocket(`${proto}://${location.host}/ws`);

  ws.onopen = () => {
    dom.wsStatus.textContent = '⬤ Connected';
    dom.wsStatus.className = 'pill green';
    // Identify as teacher
    ws.send(JSON.stringify({ type: 'identify', role: 'teacher', id: 'teacher' }));
  };

  ws.onclose = () => {
    dom.wsStatus.textContent = '⬤ Disconnected';
    dom.wsStatus.className = 'pill red';
    setTimeout(connectWS, 3000);
  };

  ws.onerror = () => ws.close();

  ws.onmessage = (ev) => {
    try {
      const msg = JSON.parse(ev.data);
      handleWsMessage(msg);
    } catch {}
  };
}

function handleWsMessage(msg) {
  switch (msg.type) {
    case 'chat':
      appendChat(msg.from, msg.content, msg.from === 'teacher');
      break;
    case 'system':
      appendSystemChat(msg.content);
      refreshStudents();              // someone joined/left
      break;
    case 'notification':
      showToast(`🔔 ${msg.hostname}: ${msg.title}`);
      if (selectedStudent === msg.hostname) refreshDetail(msg.hostname);
      break;
    case 'violation':
      showToast(`⚠️ ${msg.hostname}: ${msg.detail}`);
      if (selectedStudent === msg.hostname) refreshDetail(msg.hostname);
      break;
  }
}

function sendChat(e) {
  e.preventDefault();
  const text = dom.chatInput.value.trim();
  if (!text || !ws || ws.readyState !== 1) return;

  const to = dom.chatTo.value;
  ws.send(JSON.stringify({ type: 'chat', to, content: text }));
  if (to !== 'all') {
    appendChat('teacher', text, true);  // echo for DM
  }
  dom.chatInput.value = '';
}

function appendChat(from, content, self) {
  const div = document.createElement('div');
  div.className = `chat-msg ${self ? 'self' : 'other'}`;
  div.innerHTML = `<div class="msg-from">${esc(from)}</div>${esc(content)}`;
  dom.chatMessages.appendChild(div);
  dom.chatMessages.scrollTop = dom.chatMessages.scrollHeight;
}

function appendSystemChat(content) {
  const div = document.createElement('div');
  div.className = 'chat-msg system';
  div.textContent = content;
  dom.chatMessages.appendChild(div);
  dom.chatMessages.scrollTop = dom.chatMessages.scrollHeight;
}

// ── Student list ────────────────────────────────────────
async function refreshStudents() {
  try {
    const res = await fetch(`${API}/api/students`);
    const data = await res.json();
    allStudents = data.students || [];

    const online = allStudents.filter(s => s.active).length;
    dom.pillOnline.textContent = `${online} Online`;
    dom.pillTotal.textContent  = `${allStudents.length} Total`;

    renderStudentList();
    updateChatTargets();

    // Auto-refresh detail if selected
    if (selectedStudent) refreshDetail(selectedStudent);
  } catch (err) {
    console.error('Failed to fetch students', err);
  }
}

function renderStudentList() {
  const filter = dom.filterSelect.value;
  let list = allStudents;
  if (filter === 'active')   list = list.filter(s => s.active);
  if (filter === 'inactive') list = list.filter(s => !s.active);

  dom.studentList.innerHTML = '';
  for (const s of list) {
    const li = document.createElement('li');
    li.className = selectedStudent === s.hostname ? 'selected' : '';
    li.innerHTML = `
      <span class="dot ${s.active ? 'on' : 'off'}"></span>
      <div class="stu-info">
        <span class="stu-name">${esc(s.hostname)}</span>
        <span class="stu-sub">${esc(s.username || s.ip)} • ${s.violation_count} violations</span>
      </div>`;
    li.onclick = () => selectStudent(s.hostname);
    dom.studentList.appendChild(li);
  }
}

function updateChatTargets() {
  const current = dom.chatTo.value;
  dom.chatTo.innerHTML = '<option value="all">Broadcast (all)</option>';
  for (const s of allStudents) {
    const opt = document.createElement('option');
    opt.value = s.hostname;
    opt.textContent = s.hostname;
    dom.chatTo.appendChild(opt);
  }
  dom.chatTo.value = current;
}

// ── Student detail ──────────────────────────────────────
function selectStudent(hostname) {
  selectedStudent = hostname;
  renderStudentList();
  refreshDetail(hostname);
}

async function refreshDetail(hostname) {
  try {
    const res = await fetch(`${API}/api/students/${encodeURIComponent(hostname)}`);
    if (!res.ok) return;
    const d = await res.json();
    showDetail(d);
  } catch (err) {
    console.error('Failed to fetch student detail', err);
  }
}

function showDetail(d) {
  dom.noSelection.classList.add('hidden');
  dom.detail.classList.remove('hidden');

  const s = d.summary;
  dom.hostname.textContent = s.hostname;
  dom.status.textContent   = s.active ? '● Active' : '○ Offline';
  dom.status.className     = `pill ${s.active ? 'green' : 'red'}`;
  dom.os.textContent       = s.os || '—';
  dom.user.textContent     = s.username || '—';
  dom.ip.textContent       = s.ip || '—';

  setBar('CPU', s.cpu_usage);
  setBar('RAM', s.ram_usage);

  // Screenshot
  if (d.screenshot && d.screenshot.image_url) {
    dom.screenshotContainer.innerHTML =
      `<img src="${esc(d.screenshot.image_url)}" alt="screenshot" />`;
  } else {
    dom.screenshotContainer.innerHTML = '<p class="muted">No screenshot available</p>';
  }

  // Apps
  dom.appsBody.innerHTML = '';
  if (d.apps && d.apps.applications) {
    for (const a of d.apps.applications) {
      dom.appsBody.innerHTML +=
        `<tr><td>${esc(a.name)}</td><td>${a.pid}</td><td>${a.memory_mb.toFixed(1)} MB</td></tr>`;
    }
  }

  // Tabs
  dom.tabsBody.innerHTML = '';
  if (d.apps && d.apps.browser_tabs) {
    for (const t of d.apps.browser_tabs) {
      dom.tabsBody.innerHTML +=
        `<tr><td>${esc(t.title)}</td><td title="${esc(t.url)}">${esc(t.url)}</td></tr>`;
    }
  }

  // Violations
  dom.violCount.textContent = d.violations ? d.violations.length : 0;
  dom.violList.innerHTML = '';
  for (const v of (d.violations || [])) {
    const li = document.createElement('li');
    li.className = `ev-${v.severity || 'low'}`;
    li.innerHTML = `<span class="ev-time">${fmtTime(v.timestamp)}</span>
      <span><strong>${esc(v.rule)}</strong> — ${esc(v.detail)}</span>`;
    dom.violList.appendChild(li);
  }

  // Notifications
  dom.notifList.innerHTML = '';
  for (const n of (d.notifications || [])) {
    const li = document.createElement('li');
    li.className = `ev-${n.level || 'info'}`;
    li.innerHTML = `<span class="ev-time">${fmtTime(n.timestamp)}</span>
      <span><strong>${esc(n.title)}</strong> — ${esc(n.message)}</span>`;
    dom.notifList.appendChild(li);
  }
}

function setBar(name, pct) {
  const bar  = $(`#bar${name}`);
  const lbl  = $(`#lbl${name}`);
  const v    = Math.min(100, Math.max(0, pct || 0));
  bar.style.width = `${v}%`;
  bar.style.background = v > 80 ? 'var(--red)' : v > 50 ? 'var(--yellow)' : 'var(--accent)';
  lbl.textContent = `${v.toFixed(0)}%`;
}

// ── Helpers ─────────────────────────────────────────────
function esc(s) {
  if (!s) return '';
  const d = document.createElement('div');
  d.textContent = s;
  return d.innerHTML;
}

function fmtTime(iso) {
  if (!iso) return '';
  const d = new Date(iso);
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function showToast(text) {
  const t = document.createElement('div');
  t.style.cssText = `position:fixed;top:60px;right:20px;background:#1e2028;color:#e0e0e8;
    padding:12px 18px;border-radius:8px;border:1px solid #2a2d38;z-index:9999;font-size:13px;
    box-shadow:0 4px 20px rgba(0,0,0,.4);animation:fadeIn .3s`;
  t.textContent = text;
  document.body.appendChild(t);
  setTimeout(() => t.remove(), 4000);
}

// Fade-in animation
const style = document.createElement('style');
style.textContent = `@keyframes fadeIn{from{opacity:0;transform:translateY(-10px)}to{opacity:1;transform:translateY(0)}}`;
document.head.appendChild(style);
