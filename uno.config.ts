import { defineConfig, presetIcons, presetUno, transformerDirectives, transformerVariantGroup } from 'unocss'

export default defineConfig({
  presets: [
    presetUno(),
    presetIcons({
      scale: 1.2,
      warn: true,
      collections: {
        ri: () => import('@iconify-json/ri/icons.json').then((m) => m.default as any),
      },
    }),
  ],
  transformers: [transformerDirectives(), transformerVariantGroup()],
  shortcuts: {
    // 基础按钮：最小 32px 点击区域，padding 加大有分量感，cursor-pointer 明确可点
    'btn': 'inline-flex items-center justify-center gap-1.5 min-h-8 px-3.5 py-2 rounded-lg text-sm font-medium transition-all duration-200 cursor-pointer select-none disabled:opacity-40 disabled:cursor-not-allowed disabled:pointer-events-none',
    // 主按钮：琥珀色（暖棕主调统一），深色文字
    'btn-primary': 'btn bg-amber-500 text-slate-950 hover:bg-amber-400 active:bg-amber-600 font-semibold shadow-sm shadow-amber-500/20',
    // 次级按钮：slate-600 边框在深底上清晰可见
    'btn-outline': 'btn border border-slate-600 text-slate-300 hover:border-slate-400 hover:text-slate-100 hover:bg-slate-700/40 active:bg-slate-700/60',
    // ghost：平时低调（透明边框），hover 才显边框和背景
    'btn-ghost': 'btn text-slate-400 hover:text-slate-200 hover:bg-slate-700/50 border border-transparent hover:border-slate-600',
    // 危险按钮：玫瑰红系，hover 淡红背景
    'btn-danger': 'btn text-rose-400 hover:text-rose-300 hover:bg-rose-500/10 border border-transparent hover:border-rose-500/20',
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
