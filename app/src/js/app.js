/* ══════════════════════════════════════════════════════════════
   ChatCall — Main Application Controller
   ══════════════════════════════════════════════════════════════ */

// ── Tauri IPC Bridge ────────────────────────────────────────
const invoke = window.__TAURI__?.core?.invoke || (async () => ({}));
const listen = window.__TAURI__?.event?.listen || (() => () => {});

// ── State ───────────────────────────────────────────────────
const state = {
  username: 'User',
  isInRoom: false,
  isHost: false,
  isMuted: false,
  roomName: '',
  users: [],
  messages: [],
};

// ── DOM References ──────────────────────────────────────────
const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => document.querySelectorAll(sel);

// ── Views ───────────────────────────────────────────────────
function showView(viewId) {
  $$('.view').forEach(v => v.classList.remove('active'));
  $(`#${viewId}`).classList.add('active');
}

// ── Lobby Logic ─────────────────────────────────────────────
function initLobby() {
  const usernameInput = $('#username-input');
  const avatarEl = $('#username-avatar');
  const btnCreate = $('#btn-create-room');
  const btnJoin = $('#btn-join-room');
  const joinAddress = $('#join-address');

  // Update avatar initial as user types
  usernameInput.addEventListener('input', () => {
    const name = usernameInput.value.trim();
    state.username = name || 'User';
    avatarEl.textContent = name ? name[0].toUpperCase() : 'U';

    // Update avatar color based on name
    const hue = hashStr(state.username) % 360;
    avatarEl.style.background = `linear-gradient(135deg, hsl(${hue}, 70%, 55%), hsl(${(hue + 40) % 360}, 70%, 50%))`;
  });

  // Host a Room
  btnCreate.addEventListener('click', async () => {
    await setUsername(state.username);
    showStatus('Creating room...');

    try {
      const result = await invoke('create_room', {
        roomName: `${state.username}'s Room`,
      });
      state.isInRoom = true;
      state.isHost = true;
      state.roomName = result.room_name || `${state.username}'s Room`;
      enterRoom();
    } catch (e) {
      showStatus(`Error: ${e}`, true);
    }
  });

  // Join a Room
  btnJoin.addEventListener('click', async () => {
    const address = joinAddress.value.trim();
    if (!address) {
      showStatus('Please enter a host address', true);
      return;
    }

    await setUsername(state.username);
    showStatus('Connecting...');

    try {
      const result = await invoke('join_room', {
        hostAddress: address,
      });
      state.isInRoom = true;
      state.isHost = false;
      state.roomName = result.room_name || 'Room';
      enterRoom();
    } catch (e) {
      showStatus(`Connection failed: ${e}`, true);
    }
  });

  // Enter key to join
  joinAddress.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') btnJoin.click();
  });
}

// ── Room Logic ──────────────────────────────────────────────
function enterRoom() {
  hideStatus();
  showView('room-view');

  $('#room-name').textContent = state.roomName;
  $('#self-name').textContent = state.username + (state.isHost ? ' (Host)' : '');
  $('#self-avatar').textContent = state.username[0].toUpperCase();
  $('#user-count').textContent = `${state.users.length + 1} user${state.users.length > 0 ? 's' : ''}`;

  // Update avatar color
  const hue = hashStr(state.username) % 360;
  $('#self-avatar').style.background = `linear-gradient(135deg, hsl(${hue}, 70%, 55%), hsl(${(hue + 40) % 360}, 70%, 50%))`;

  initRoomControls();
}

function initRoomControls() {
  const btnLeave = $('#btn-leave');
  const btnMute = $('#btn-mute');
  const chatInput = $('#chat-input');
  const btnSend = $('#btn-send');

  // Leave room
  btnLeave.addEventListener('click', async () => {
    try {
      await invoke('leave_room');
    } catch (e) {
      console.error('Leave error:', e);
    }
    state.isInRoom = false;
    state.isHost = false;
    state.users = [];
    state.messages = [];
    showView('lobby-view');
  });

  // Toggle mute
  btnMute.addEventListener('click', async () => {
    state.isMuted = !state.isMuted;
    btnMute.classList.toggle('muted', state.isMuted);
    $('#mic-on-icon').classList.toggle('hidden', state.isMuted);
    $('#mic-off-icon').classList.toggle('hidden', !state.isMuted);
    $('#self-mute-icon').textContent = state.isMuted ? '🔇' : '🎤';
    $('#self-mute-icon').classList.toggle('muted', state.isMuted);

    try {
      await invoke('toggle_mute');
    } catch (e) {
      console.error('Mute error:', e);
    }
  });

  // Send chat message
  const sendMessage = async () => {
    const text = chatInput.value.trim();
    if (!text) return;

    addChatMessage({
      username: state.username,
      text,
      timestamp: new Date(),
      isSelf: true,
    });

    chatInput.value = '';

    try {
      await invoke('send_message', { text });
    } catch (e) {
      console.error('Send error:', e);
    }
  };

  btnSend.addEventListener('click', sendMessage);
  chatInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  });
}

// ── Chat UI ─────────────────────────────────────────────────
function addChatMessage({ username, text, timestamp, isSelf = false }) {
  const container = $('#chat-messages');
  const emptyMsg = container.querySelector('.chat-empty');
  if (emptyMsg) emptyMsg.remove();

  const msgEl = document.createElement('div');
  msgEl.className = `chat-message ${isSelf ? 'self' : 'other'}`;

  const time = new Date(timestamp);
  const timeStr = time.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

  msgEl.innerHTML = `
    <div class="chat-msg-author">${escapeHtml(username)}</div>
    <div class="chat-msg-text">${escapeHtml(text)}</div>
    <div class="chat-msg-time">${timeStr}</div>
  `;

  container.appendChild(msgEl);
  container.scrollTop = container.scrollHeight;

  state.messages.push({ username, text, timestamp });
}

// ── User Grid ───────────────────────────────────────────────
function addUserTile(user) {
  const grid = $('#user-grid');
  const hue = hashStr(user.username) % 360;

  const tile = document.createElement('div');
  tile.className = 'user-tile scale-enter';
  tile.id = `user-tile-${user.user_id}`;
  tile.innerHTML = `
    <div class="user-avatar-ring">
      <div class="voice-ring"></div>
      <div class="user-avatar" style="background: linear-gradient(135deg, hsl(${hue}, 70%, 55%), hsl(${(hue + 40) % 360}, 70%, 50%))">${user.username[0].toUpperCase()}</div>
    </div>
    <span class="user-name">${escapeHtml(user.username)}</span>
    <div class="user-status-icons">
      <span class="mute-icon ${user.is_muted ? 'muted' : ''}">${user.is_muted ? '🔇' : '🎤'}</span>
    </div>
  `;

  grid.appendChild(tile);
}

function removeUserTile(userId) {
  const tile = $(`#user-tile-${userId}`);
  if (tile) {
    tile.style.animation = 'fadeOut 0.3s ease forwards';
    setTimeout(() => tile.remove(), 300);
  }
}

// ── Status helpers ──────────────────────────────────────────
function showStatus(text, isError = false) {
  const bar = $('#lobby-status');
  const textEl = bar.querySelector('.status-text');
  bar.classList.remove('hidden');
  textEl.textContent = text;

  if (isError) {
    bar.style.borderColor = 'rgba(239, 68, 68, 0.3)';
    bar.style.background = 'rgba(239, 68, 68, 0.1)';
    setTimeout(() => hideStatus(), 4000);
  } else {
    bar.style.borderColor = '';
    bar.style.background = '';
  }
}

function hideStatus() {
  $('#lobby-status').classList.add('hidden');
}

// ── Utility functions ───────────────────────────────────────
function hashStr(str) {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = ((hash << 5) - hash + str.charCodeAt(i)) | 0;
  }
  return Math.abs(hash);
}

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

async function setUsername(username) {
  try {
    await invoke('set_username', { username });
  } catch (e) {
    console.error('setUsername error:', e);
  }
}

// ── Voice Level Animation (simulated for now) ───────────────
function animateVoiceLevel() {
  const bar = $('#voice-level');
  if (!state.isInRoom || state.isMuted) {
    bar.style.width = '0%';
    requestAnimationFrame(animateVoiceLevel);
    return;
  }

  // Simulated voice level (will be connected to real audio data later)
  const level = Math.random() * 15 + 2;
  bar.style.width = `${level}%`;
  requestAnimationFrame(animateVoiceLevel);
}

// ── Initialize ──────────────────────────────────────────────
document.addEventListener('DOMContentLoaded', () => {
  initLobby();
  animateVoiceLevel();

  // Load saved profile
  invoke('get_profile').then(profile => {
    if (profile?.username) {
      state.username = profile.username;
      $('#username-input').value = profile.username;
      $('#username-avatar').textContent = profile.username[0].toUpperCase();

      const hue = hashStr(profile.username) % 360;
      $('#username-avatar').style.background = `linear-gradient(135deg, hsl(${hue}, 70%, 55%), hsl(${(hue + 40) % 360}, 70%, 50%))`;
    }
  }).catch(() => {});
});
