/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./templates/**/*.html'],
  theme: {
    extend: {
      colors: {
        bg:         'var(--bg)',
        panel:      'var(--panel)',
        panel2:     'var(--panel2)',
        ink:        'var(--ink)',
        'ink-soft': 'var(--ink-soft)',
        'ink-faint':'var(--ink-faint)',
        line:       'var(--line)',
        accent:     'var(--accent)',
        warn:       'var(--warn)',
        danger:     'var(--danger)',
        ok:         'var(--ok)',
      },
      fontFamily: {
        sans: ['Inter', '-apple-system', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'ui-monospace', 'monospace'],
      },
    },
  },
  plugins: [],
}
