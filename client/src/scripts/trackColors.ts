// Map track data → V4 track-color palette tokens.
// Used to color row covers, hot-cue dots, and accent glows in track tiles.

// V4 track-color palette (8 hues, references CSS vars from tokens.scss).
export const TRACK_COLORS = [
    'var(--t-orange)',
    'var(--t-amber)',
    'var(--t-lime)',
    'var(--t-mint)',
    'var(--t-cyan)',
    'var(--t-sky)',
    'var(--t-violet)',
    'var(--t-rose)',
];

// Quasar mood color name → V4 track-color CSS var.
// The Settings UI lets users pick a Quasar color name for each mood; this remaps to V4.
const MOOD_TO_TRACK: Record<string, string> = {
    red: 'var(--t-orange)',
    'red-10': 'var(--t-orange)',
    'deep-orange': 'var(--t-orange)',
    'deep-orange-10': 'var(--t-orange)',
    orange: 'var(--t-orange)',
    brown: 'var(--t-orange)',
    amber: 'var(--t-amber)',
    yellow: 'var(--t-amber)',
    lime: 'var(--t-lime)',
    'lime-9': 'var(--t-lime)',
    'light-green': 'var(--t-mint)',
    green: 'var(--t-mint)',
    'green-10': 'var(--t-mint)',
    cyan: 'var(--t-cyan)',
    'cyan-10': 'var(--t-cyan)',
    teal: 'var(--t-cyan)',
    'teal-10': 'var(--t-cyan)',
    'light-blue': 'var(--t-sky)',
    'light-blue-10': 'var(--t-sky)',
    blue: 'var(--t-sky)',
    'blue-4': 'var(--t-sky)',
    'blue-grey': 'var(--t-sky)',
    'blue-grey-9': 'var(--t-sky)',
    indigo: 'var(--t-violet)',
    'indigo-5': 'var(--t-violet)',
    purple: 'var(--t-violet)',
    'purple-4': 'var(--t-violet)',
    'deep-purple': 'var(--t-violet)',
    'deep-purple-4': 'var(--t-violet)',
    pink: 'var(--t-rose)',
    'pink-4': 'var(--t-rose)',
    grey: 'var(--color-fg-subtle)',
    'grey-8': 'var(--color-fg-subtle)',
};

export function moodColorToTrack(quasarColor?: string): string {
    if (!quasarColor) return 'var(--color-accent)';
    return MOOD_TO_TRACK[quasarColor] || 'var(--color-accent)';
}

// Parallel glow tokens for the 8 V4 track colors.
const TRACK_GLOWS = [
    'var(--t-orange-glow)',
    'var(--t-amber-glow)',
    'var(--t-lime-glow)',
    'var(--t-mint-glow)',
    'var(--t-cyan-glow)',
    'var(--t-sky-glow)',
    'var(--t-violet-glow)',
    'var(--t-rose-glow)',
];

const COLOR_TO_GLOW: Record<string, string> = {
    'var(--t-orange)': 'var(--t-orange-glow)',
    'var(--t-amber)': 'var(--t-amber-glow)',
    'var(--t-lime)': 'var(--t-lime-glow)',
    'var(--t-mint)': 'var(--t-mint-glow)',
    'var(--t-cyan)': 'var(--t-cyan-glow)',
    'var(--t-sky)': 'var(--t-sky-glow)',
    'var(--t-violet)': 'var(--t-violet-glow)',
    'var(--t-rose)': 'var(--t-rose-glow)',
    'var(--color-accent)': 'var(--color-accent-glow)',
};

export function colorToGlow(color: string): string {
    return COLOR_TO_GLOW[color] || 'var(--color-accent-glow)';
}

// Hash a string to a stable index in TRACK_COLORS.
// Same value always gets the same color across the app.
export function hashColor(str: string): string {
    if (!str) return TRACK_COLORS[0];
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
        hash = ((hash << 5) - hash) + str.charCodeAt(i);
        hash |= 0;
    }
    return TRACK_COLORS[Math.abs(hash) % TRACK_COLORS.length];
}

export function hashGlow(str: string): string {
    if (!str) return TRACK_GLOWS[0];
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
        hash = ((hash << 5) - hash) + str.charCodeAt(i);
        hash |= 0;
    }
    return TRACK_GLOWS[Math.abs(hash) % TRACK_GLOWS.length];
}

// Get { color, glow } for a track. Mood-based when available, hash-based fallback so
// every track gets a distinct, stable color from the 8-color V4 palette.
export interface TrackPaint {
    color: string;
    glow: string;
}

export function trackPaint(track: any, settings: any): TrackPaint {
    // Mood-based first
    if (track?.mood && settings?.quickTag?.moods) {
        const m = settings.quickTag.moods.find((x: any) => x.mood === track.mood);
        if (m?.color) {
            const c = moodColorToTrack(m.color);
            return { color: c, glow: colorToGlow(c) };
        }
    }
    // Hash-based fallback — stable per track
    const seed = track?.path || track?.title || '';
    return { color: hashColor(seed), glow: hashGlow(seed) };
}
