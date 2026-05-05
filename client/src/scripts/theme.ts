// DigTrax theme manager
// Persists to localStorage. Listens to prefers-color-scheme for `system` mode.
// Tokens (CSS vars) are defined in /client/src/style/tokens.scss; this just flips data-theme.

export type ThemeMode = 'dark' | 'light' | 'system';

const STORAGE_KEY = 'digtrax-theme';

function resolveSystemTheme(): 'dark' | 'light' {
    if (typeof window === 'undefined' || !window.matchMedia) return 'dark';
    return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
}

function applyTheme(mode: ThemeMode) {
    const resolved = mode === 'system' ? resolveSystemTheme() : mode;
    document.documentElement.setAttribute('data-theme', resolved);
}

export function getTheme(): ThemeMode {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === 'dark' || stored === 'light' || stored === 'system') return stored;
    return 'dark';
}

export function setTheme(mode: ThemeMode) {
    localStorage.setItem(STORAGE_KEY, mode);
    applyTheme(mode);
}

export function initTheme() {
    const mode = getTheme();
    applyTheme(mode);
    // Re-apply on system pref change when in system mode
    if (window.matchMedia) {
        window.matchMedia('(prefers-color-scheme: light)').addEventListener('change', () => {
            if (getTheme() === 'system') applyTheme('system');
        });
    }
}
