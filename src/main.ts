import './style.css';
import RFB from './vnc/rfb';
import WebAudio from './vnc/webaudio';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { WebglAddon } from '@xterm/addon-webgl';

// ────────────────────────────────────────────────────
// DOM refs
// ────────────────────────────────────────────────────
const overlay = document.getElementById('connect-overlay')!;
const form = document.getElementById('connect-form') as HTMLFormElement;
const hostInput = document.getElementById('ws-host') as HTMLInputElement;
const pwInput = document.getElementById('ws-password') as HTMLInputElement;
const errorEl = document.getElementById('connect-error')!;
const connectBtn = document.getElementById('connect-btn') as HTMLButtonElement;

const mainEl = document.getElementById('main')!;
const screenEl = document.getElementById('screen')!;
const leftPanel = document.getElementById('left-panel')!;
const rightPanel = document.getElementById('right-panel')!;
const resizeHandle = document.getElementById('resize-handle')!;

const tabsEl = document.getElementById('term-tabs')!;
const termContainer = document.getElementById('term-container')!;

// ────────────────────────────────────────────────────
// State
// ────────────────────────────────────────────────────
let rfb: any = null;
let audio: WebAudio | null = null;
let baseUrl = ''; // e.g. "localhost:6080"

interface TermTab {
    id: number;
    terminal: Terminal;
    fitAddon: FitAddon;
    ws: WebSocket;
    tabEl: HTMLElement;
    wrapEl: HTMLDivElement;
}

let tabs: TermTab[] = [];
let activeTabId = -1;
let nextTabId = 1;

// ────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────
function wsUrl(path: string): string {
    const proto = location.protocol === 'https:' ? 'wss' : 'ws';
    return `${proto}://${baseUrl}${path}`;
}

function httpUrl(path: string): string {
    const proto = location.protocol === 'https:' ? 'https' : 'http';
    return `${proto}://${baseUrl}${path}`;
}

function showError(msg: string) {
    errorEl.textContent = msg;
}

function setConnected(connected: boolean) {
    if (connected) {
        overlay.classList.add('hidden');
        mainEl.classList.remove('hidden');
    } else {
        overlay.classList.remove('hidden');
        mainEl.classList.add('hidden');
    }
}

// ────────────────────────────────────────────────────
// VNC + Audio connection
// ────────────────────────────────────────────────────
function connect(host: string, password?: string) {
    baseUrl = host;
    showError('');
    connectBtn.disabled = true;
    connectBtn.textContent = 'Connecting…';

    const vncUrl = wsUrl('/ws/vnc');

    try {
        rfb = new RFB(screenEl, vncUrl, {
            credentials: password ? { password } : {},
        });
    } catch (e: any) {
        showError(e.message ?? 'Failed to create connection');
        connectBtn.disabled = false;
        connectBtn.textContent = 'Connect';
        return;
    }

    rfb.scaleViewport = true;
    rfb.resizeSession = false;

    rfb.addEventListener('connect', () => {
        setConnected(true);
        connectBtn.disabled = false;
        connectBtn.textContent = 'Connect';
        screenEl.focus();

        // Start audio
        const audioUrl = wsUrl('/ws/audio');
        audio = new WebAudio(audioUrl);
        audio.start();

        // Create first terminal tab
        if (tabs.length === 0) {
            createTab();
        }

        // Start notification listener
        startNotifications();
    });

    rfb.addEventListener('disconnect', (ev: CustomEvent) => {
        setConnected(false);
        connectBtn.disabled = false;
        connectBtn.textContent = 'Connect';
        if (!ev.detail.clean) {
            showError('Connection lost');
        }
        rfb = null;
        cleanupAll();
    });

    rfb.addEventListener('credentialsrequired', () => {
        setConnected(false);
        showError('Server requires a password');
        connectBtn.disabled = false;
        connectBtn.textContent = 'Connect';
        pwInput.focus();
        rfb.disconnect();
        rfb = null;
    });
}

function disconnect() {
    if (rfb) {
        rfb.disconnect();
        rfb = null;
    }
    audio = null;
    cleanupAll();
    setConnected(false);
}

function cleanupAll() {
    for (const tab of tabs) {
        tab.ws.close();
        tab.terminal.dispose();
        tab.tabEl.remove();
        tab.wrapEl.remove();
    }
    tabs = [];
    activeTabId = -1;
}

// ────────────────────────────────────────────────────
// Terminal tabs
// ────────────────────────────────────────────────────
function createTab() {
    const id = nextTabId++;
    const sshUrl = wsUrl('/ws/ssh');

    // WebSocket for this session
    const ws = new WebSocket(sshUrl);
    ws.binaryType = 'arraybuffer';

    // Terminal instance
    const term = new Terminal({
        fontSize: 14,
        fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
        cursorBlink: true,
        theme: {
            background: '#1a1a2e',
            foreground: '#eee',
            cursor: '#e94560',
            selectionBackground: '#0f3460',
        },
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(new WebLinksAddon());

    // Wrapper div for this terminal
    const wrapEl = document.createElement('div');
    wrapEl.style.cssText = 'position:absolute;inset:0;display:none;';
    termContainer.appendChild(wrapEl);

    // Tab button
    const tabEl = document.createElement('div');
    tabEl.className = 'term-tab';
    tabEl.innerHTML = `<span class="tab-label">Shell ${id}</span><span class="close-tab">✕</span>`;
    tabsEl.appendChild(tabEl);

    const tab: TermTab = { id, terminal: term, fitAddon, ws, tabEl, wrapEl };
    tabs.push(tab);

    // Open terminal into wrapper
    term.open(wrapEl);

    // Try WebGL renderer
    try {
        term.loadAddon(new WebglAddon());
    } catch {
        // fallback to canvas
    }

    // WS ↔ Terminal
    ws.addEventListener('message', (ev: MessageEvent) => {
        if (ev.data instanceof ArrayBuffer) {
            term.write(new Uint8Array(ev.data));
        } else {
            term.write(ev.data);
        }
    });

    ws.addEventListener('close', () => {
        term.write('\r\n\x1b[31m[Session ended]\x1b[0m\r\n');
    });

    term.onData((data) => {
        if (ws.readyState === WebSocket.OPEN) {
            ws.send(new TextEncoder().encode(data));
        }
    });

    term.onBinary((data) => {
        if (ws.readyState === WebSocket.OPEN) {
            const buf = new Uint8Array(data.length);
            for (let i = 0; i < data.length; i++) buf[i] = data.charCodeAt(i);
            ws.send(buf);
        }
    });

    // Tab click
    tabEl.querySelector('.tab-label')!.addEventListener('click', () => {
        activateTab(id);
    });

    // Close tab
    tabEl.querySelector('.close-tab')!.addEventListener('click', (e) => {
        e.stopPropagation();
        closeTab(id);
    });

    activateTab(id);
}

function activateTab(id: number) {
    activeTabId = id;
    for (const tab of tabs) {
        const isActive = tab.id === id;
        tab.wrapEl.style.display = isActive ? 'block' : 'none';
        tab.tabEl.classList.toggle('active', isActive);
        if (isActive) {
            // Small delay so layout settles
            requestAnimationFrame(() => {
                tab.fitAddon.fit();
                tab.terminal.focus();
            });
        }
    }
}

function closeTab(id: number) {
    const idx = tabs.findIndex((t) => t.id === id);
    if (idx === -1) return;

    const tab = tabs[idx];
    tab.ws.close();
    tab.terminal.dispose();
    tab.tabEl.remove();
    tab.wrapEl.remove();
    tabs.splice(idx, 1);

    if (tabs.length === 0) {
        activeTabId = -1;
    } else if (activeTabId === id) {
        // Activate nearest tab
        const next = tabs[Math.min(idx, tabs.length - 1)];
        activateTab(next.id);
    }
}

// ────────────────────────────────────────────────────
// Notifications (Web Notification API)
// ────────────────────────────────────────────────────
let notifyWs: WebSocket | null = null;

function startNotifications() {
    // Request permission
    if ('Notification' in window && Notification.permission === 'default') {
        Notification.requestPermission();
    }

    const url = wsUrl('/ws/notify');
    notifyWs = new WebSocket(url);

    notifyWs.addEventListener('message', (ev: MessageEvent) => {
        try {
            const data = JSON.parse(ev.data);
            if (data.type === 'notification' && 'Notification' in window && Notification.permission === 'granted') {
                new Notification(data.summary || 'Notification', {
                    body: data.body || '',
                    icon: data.icon || undefined,
                    tag: `lemoncuc-${Date.now()}`,
                });
            }
        } catch {
            // ignore parse errors
        }
    });
}

// ────────────────────────────────────────────────────
// Resize handle (drag to split)
// ────────────────────────────────────────────────────
{
    let dragging = false;

    resizeHandle.addEventListener('mousedown', (e) => {
        e.preventDefault();
        dragging = true;
        resizeHandle.classList.add('active');
        document.body.style.cursor = 'col-resize';
        document.body.style.userSelect = 'none';
    });

    window.addEventListener('mousemove', (e) => {
        if (!dragging) return;
        const total = mainEl.clientWidth;
        const leftPx = Math.max(200, Math.min(e.clientX, total - 200));
        const leftPct = (leftPx / total) * 100;
        leftPanel.style.flex = `0 0 ${leftPct}%`;
        rightPanel.style.flex = `1 1 0`;
        // Refit active terminal
        const active = tabs.find((t) => t.id === activeTabId);
        if (active) active.fitAddon.fit();
    });

    window.addEventListener('mouseup', () => {
        if (!dragging) return;
        dragging = false;
        resizeHandle.classList.remove('active');
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
    });
}

// ────────────────────────────────────────────────────
// Toolbar buttons
// ────────────────────────────────────────────────────
document.getElementById('btn-fullscreen')!.addEventListener('click', () => {
    if (document.fullscreenElement) {
        document.exitFullscreen();
    } else {
        document.documentElement.requestFullscreen();
    }
});

document.getElementById('btn-paste')!.addEventListener('click', async () => {
    if (!rfb) return;
    const pasteModal = document.getElementById('paste-modal')!;
    const textarea = document.getElementById('paste-textarea') as HTMLTextAreaElement;

    // Pre-fill from clipboard if possible
    try {
        const clip = await navigator.clipboard.readText();
        if (clip) textarea.value = clip;
    } catch {
        // clipboard not available, leave empty
    }

    pasteModal.classList.remove('hidden');
    textarea.focus();
});

function sendTextToVnc(text: string) {
    if (!rfb || !text) return;
    for (let i = 0; i < text.length; i++) {
        const ch = text[i];
        if (ch === '\n') {
            rfb.sendKey(0xff0d, null, true);  // XK_Return
            rfb.sendKey(0xff0d, null, false);
        } else if (ch === '\r') {
            // skip \r (from \r\n pairs)
        } else {
            const code = ch.charCodeAt(0);
            rfb.sendKey(code, null, true);
            rfb.sendKey(code, null, false);
        }
    }
}

document.getElementById('btn-paste-send')!.addEventListener('click', () => {
    const textarea = document.getElementById('paste-textarea') as HTMLTextAreaElement;
    sendTextToVnc(textarea.value);
    textarea.value = '';
    document.getElementById('paste-modal')!.classList.add('hidden');
});

document.getElementById('btn-paste-cancel')!.addEventListener('click', () => {
    (document.getElementById('paste-textarea') as HTMLTextAreaElement).value = '';
    document.getElementById('paste-modal')!.classList.add('hidden');
});

document.getElementById('paste-modal')!.addEventListener('click', (e) => {
    if (e.target === document.getElementById('paste-modal')) {
        document.getElementById('paste-modal')!.classList.add('hidden');
    }
});

document.getElementById('btn-openapi')!.addEventListener('click', () => {
    window.open(httpUrl('/api/openapi.json'), '_blank');
});

// ────────────────────────────────────────────────────
// Keybindings modal
// ────────────────────────────────────────────────────
const kbModal = document.getElementById('keybindings-modal')!;

document.getElementById('btn-keybindings')!.addEventListener('click', () => {
    kbModal.classList.remove('hidden');
});
document.getElementById('btn-close-keybindings')!.addEventListener('click', () => {
    kbModal.classList.add('hidden');
});
kbModal.addEventListener('click', (e) => {
    if (e.target === kbModal) kbModal.classList.add('hidden');
});

document.getElementById('btn-disconnect')!.addEventListener('click', () => {
    disconnect();
});

document.getElementById('btn-new-tab')!.addEventListener('click', () => {
    createTab();
});

// ────────────────────────────────────────────────────
// Window resize → refit terminals
// ────────────────────────────────────────────────────
window.addEventListener('resize', () => {
    const active = tabs.find((t) => t.id === activeTabId);
    if (active) active.fitAddon.fit();
});

// ────────────────────────────────────────────────────
// Form submit
// ────────────────────────────────────────────────────
form.addEventListener('submit', (e) => {
    e.preventDefault();
    const host = hostInput.value.trim();
    const pw = pwInput.value;

    if (!host) {
        showError('Host is required');
        return;
    }

    localStorage.setItem('lemoncuc-host', host);
    connect(host, pw || undefined);
});

// Restore last host
const savedHost = localStorage.getItem('lemoncuc-host');
if (savedHost) hostInput.value = savedHost;

