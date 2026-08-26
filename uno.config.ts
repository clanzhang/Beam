import { defineConfig, presetUno, transformerDirectives, transformerVariantGroup } from 'unocss'

export default defineConfig({
  presets: [presetUno()],
  transformers: [transformerDirectives(), transformerVariantGroup()],
  shortcuts: {
    'btn': 'inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed',
    'btn-primary': 'bg-cyan-400 text-slate-950 hover:opacity-90 active:scale-95 font-semibold',
    'btn-ghost': 'text-slate-400 hover:text-slate-200 hover:bg-white/5',
    'btn-outline': 'border border-line text-slate-200 hover:bg-white/5',
    'card': 'bg-surface border border-line rounded-xl',
  },
  theme: {
    colors: {
      surface: '#0f172a',
      elevated: '#1e293b',
      line: '#334155',
      ink: '#e2e8f0',
    },
  },
})
