import RFB from './vnc/rfb';
import WebAudio from './vnc/webaudio';

// ── DOM refs ──
const overlay = document.getElementById('connect-overlay')!;
const form = document.getElementById('connect-form') as HTMLFormElement;
const urlInput = document.getElementById('ws-url') as HTMLInputElement;
const audioUrlInput = document.getElementById('ws-audio-url') as HTMLInputElement;
const pwInput = document.getElementById('ws-password') as HTMLInputElement;
const errorEl = document.getElementById('connect-error')!;
const connectBtn = document.getElementById('connect-btn') as HTMLButtonElement;
const screenEl = document.getElementById('screen')!;
const toolbar = document.getElementById('toolbar')!;

let rfb: any = null;
let audio: WebAudio | null = null;

// ── Helpers ──
function showError(msg: string) {
    errorEl.textContent = msg;
}

function setConnected(connected: boolean) {
    if (connected) {
        overlay.classList.add('hidden');
        screenEl.classList.add('connected');
        toolbar.classList.add('connected');
    } else {
        overlay.classList.remove('hidden');
        screenEl.classList.remove('connected');
        toolbar.classList.remove('connected');
    }
}

// ── Connect ──
/** Derive audio URL from VNC URL: swap port to 5702 */
function deriveAudioUrl(vncUrl: string): string {
    try {
        const u = new URL(vncUrl);
        u.port = '5702';
        return u.toString();
    } catch {
        return '';
    }
}

function connect(url: string, password?: string) {
    showError('');
    connectBtn.disabled = true;
    connectBtn.textContent = 'Connecting…';

    try {
        rfb = new RFB(screenEl, url, {
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

        // Start audio streaming immediately (Connect click = user gesture)
        const audioUrl = audioUrlInput.value.trim() || deriveAudioUrl(url);
        if (audioUrl) {
            audio = new WebAudio(audioUrl);
            audio.start();
        }
    });

    rfb.addEventListener('disconnect', (ev: CustomEvent) => {
        setConnected(false);
        connectBtn.disabled = false;
        connectBtn.textContent = 'Connect';
        if (!ev.detail.clean) {
            showError('Connection lost');
        }
        rfb = null;
    });

    rfb.addEventListener('credentialsrequired', () => {
        // Bring back overlay so user can type password
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
    setConnected(false);
}

// ── Form submit ──
form.addEventListener('submit', (e) => {
    e.preventDefault();
    const url = urlInput.value.trim();
    const pw = pwInput.value;

    if (!url) {
        showError('URL is required');
        return;
    }
    if (!/^wss?:\/\//.test(url)) {
        showError('URL must start with ws:// or wss://');
        return;
    }

    connect(url, pw || undefined);
});

// ── Toolbar buttons ──
document.getElementById('btn-fullscreen')!.addEventListener('click', () => {
    if (document.fullscreenElement) {
        document.exitFullscreen();
    } else {
        document.documentElement.requestFullscreen();
    }
});

document.getElementById('btn-cad')!.addEventListener('click', () => {
    rfb?.sendCtrlAltDel();
});

document.getElementById('btn-paste')!.addEventListener('click', async () => {
    if (!rfb) return;
    try {
        const text = await navigator.clipboard.readText();
        if (text) {
            for (let i = 0; i < text.length; i++) {
                const code = text.charCodeAt(i);
                rfb.sendKey(code, null, true);  // keydown
                rfb.sendKey(code, null, false); // keyup
            }
        }
    } catch {
        // Fallback: prompt
        const text = prompt('Paste text to send:');
        if (text) {
            for (let i = 0; i < text.length; i++) {
                const code = text.charCodeAt(i);
                rfb.sendKey(code, null, true);
                rfb.sendKey(code, null, false);
            }
        }
    }
});

document.getElementById('btn-disconnect')!.addEventListener('click', () => {
    disconnect();
});

// ── Restore last URLs from localStorage ──
const savedUrl = localStorage.getItem('vnc-url');
if (savedUrl) urlInput.value = savedUrl;
const savedAudioUrl = localStorage.getItem('vnc-audio-url');
if (savedAudioUrl) audioUrlInput.value = savedAudioUrl;

form.addEventListener('submit', () => {
    localStorage.setItem('vnc-url', urlInput.value.trim());
    localStorage.setItem('vnc-audio-url', audioUrlInput.value.trim());
});

