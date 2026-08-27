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
    // 基础按钮：深色主题统一风格，最小 32px 点击区域，文字 13px
    'btn': 'inline-flex items-center justify-center gap-1.5 min-h-8 px-4 py-2 rounded-lg text-[13px] font-medium transition-all duration-200 cursor-pointer select-none disabled:opacity-40 disabled:cursor-not-allowed disabled:pointer-events-none',
    // 主按钮：琥珀实心（唯一的高饱和填充），深色文字
    'btn-primary': 'btn bg-amber-500 text-slate-950 hover:bg-amber-400 active:bg-amber-600 font-semibold shadow-sm shadow-amber-500/15',
    // 次级按钮：半透明深底 + 可见边框，无浅色填充
    'btn-outline': 'btn bg-slate-800/60 border border-slate-600 text-slate-300 hover:bg-slate-700/60 hover:border-slate-500 hover:text-slate-100 active:bg-slate-700',
    // ghost：半透明深底 + 细边框，看起来像个可点按钮
    'btn-ghost': 'btn bg-slate-800/40 border border-slate-700 text-slate-300 hover:bg-slate-700/50 hover:border-slate-500 hover:text-slate-100',
    // 危险按钮：弱红半透明底，与文件行区分
    'btn-danger': 'btn bg-rose-500/10 border border-rose-500/20 text-rose-400 hover:bg-rose-500/20 hover:border-rose-500/30 hover:text-rose-300',
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
