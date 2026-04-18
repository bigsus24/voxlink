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
  roomCode: null,
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
    showStatus('Detecting your IP and generating room code...');

    try {
      const result = await invoke('create_room', {
        roomName: `${state.username}'s Room`,
      });
      state.isInRoom = true;
      state.isHost = true;
      state.roomName = result.room_name || `${state.username}'s Room`;
      state.roomCode = result.room_code || null;
      enterRoom();
    } catch (e) {
      showStatus(`Error: ${e}`, true);
    }
  });

  // Join a Room (accepts 7-char VoxCode OR raw IP address)
  btnJoin.addEventListener('click', async () => {
    const input = joinAddress.value.trim();
    if (!input) {
      showStatus('Please enter a Room Code or IP address', true);
      return;
    }

    await setUsername(state.username);

    // Detect: 7-char alphanumeric = VoxCode; otherwise treat as IP
    const isVoxCode = /^[A-Za-z0-9]{7}$/.test(input);

    if (isVoxCode) {
      showStatus('Decoding room code...');
    } else {
      showStatus('Connecting...');
    }

    try {
      const command = isVoxCode ? 'join_by_code' : 'join_room';
      const args = isVoxCode
        ? { code: input.toUpperCase() }
        : { hostAddress: input };

      const result = await invoke(command, args);
      state.isInRoom = true;
      state.isHost = false;
      state.roomName = result.room_name || 'Room';
      state.roomCode = null;
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

  // Show room code and close button for hosts
  if (state.isHost && state.roomCode) {
    const badge = $('#room-code-badge');
    const closeBtn = $('#btn-close-room');
    badge.classList.remove('hidden');
    closeBtn.classList.remove('hidden');
    $('#room-code-value').textContent = state.roomCode;
  } else {
    $('#room-code-badge').classList.add('hidden');
    $('#btn-close-room').classList.add('hidden');
  }

  initRoomControls();
}

function initRoomControls() {
  const btnLeave = $('#btn-leave');
  const btnMute = $('#btn-mute');
  const chatInput = $('#chat-input');
  const btnSend = $('#btn-send');

  // Leave room (client)
  btnLeave.addEventListener('click', async () => {
    try { await invoke('leave_room'); } catch (e) { console.error('Leave error:', e); }
    resetRoomState();
    showView('lobby-view');
  });

  // Close room (host only)
  const btnClose = $('#btn-close-room');
  btnClose.addEventListener('click', async () => {
    try { await invoke('close_room'); } catch (e) { console.error('Close error:', e); }
    resetRoomState();
    showView('lobby-view');
  });

  // Copy room code
  const btnCopy = $('#btn-copy-code');
  if (btnCopy) {
    btnCopy.addEventListener('click', async () => {
      const code = $('#room-code-value').textContent;
      try {
        await navigator.clipboard.writeText(code);
      } catch {
        // Fallback for environments without clipboard API
        const ta = document.createElement('textarea');
        ta.value = code;
        document.body.appendChild(ta);
        ta.select();
        document.execCommand('copy');
        ta.remove();
      }
      // Animate checkmark
      $('#copy-icon').classList.add('hidden');
      $('#check-icon').classList.remove('hidden');
      setTimeout(() => {
        $('#copy-icon').classList.remove('hidden');
        $('#check-icon').classList.add('hidden');
      }, 2000);
    });
  }

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

function resetRoomState() {
  state.isInRoom = false;
  state.isHost = false;
  state.roomCode = null;
  state.users = [];
  state.messages = [];
  // Hide host-only controls
  $('#room-code-badge').classList.add('hidden');
  $('#btn-close-room').classList.add('hidden');
  // Clear chat
  const container = $('#chat-messages');
  container.innerHTML = '<div class="chat-empty"><p>No messages yet. Say hi! 👋</p></div>';
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
