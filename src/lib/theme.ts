export type ThemeMode = 'light' | 'dark' | 'system';

const STORAGE_KEY = 'theme-mode';

export function getStoredTheme(): ThemeMode {
	if (typeof localStorage === 'undefined') return 'system';
	const v = localStorage.getItem(STORAGE_KEY);
	if (v === 'light' || v === 'dark' || v === 'system') return v;
	return 'system';
}

export function setStoredTheme(mode: ThemeMode) {
	localStorage.setItem(STORAGE_KEY, mode);
}

export function systemPrefersDark(): boolean {
	return window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false;
}

export function applyTheme(mode: ThemeMode) {
	const root = document.documentElement;

	const isDark = mode === 'dark' || (mode === 'system' && systemPrefersDark());
	root.classList.toggle('dark', isDark);

	// optional: helps native UI (scrollbars, form controls)
	root.style.colorScheme = isDark ? 'dark' : 'light';
}

export function watchSystemTheme(onChange: () => void) {
	const mq = window.matchMedia('(prefers-color-scheme: dark)');
	const handler = () => onChange();
	mq.addEventListener?.('change', handler);
	return () => mq.removeEventListener?.('change', handler);
}
