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
    // 基础按钮：统一最小 32px 点击区域与内边距，主次按钮 padding 一致
    'btn': 'inline-flex items-center justify-center gap-1.5 min-h-8 px-3.5 py-2 rounded-2xl text-sm font-medium transition-all duration-200 active:scale-[0.97] select-none disabled:opacity-50 disabled:cursor-not-allowed disabled:pointer-events-none',
    // 主按钮：陶土橘实心 + 3D 悬浮阴影
    'btn-primary': 'btn text-white bg-[#D17159] hover:bg-[#C06047] font-semibold shadow-[rgba(50,50,93,0.25)_0px_50px_100px_-20px,rgba(0,0,0,0.3)_0px_30px_60px_-30px,rgba(10,37,64,0.35)_0px_-2px_6px_0px_inset]',
    // 次级按钮：白底描边，hover 边框/文字变青、底色淡青
    'btn-outline': 'btn bg-white border border-[#E3D9CF] text-[#3E3E42] hover:border-cyan-400/40 hover:text-cyan-300 hover:bg-cyan-400/5 shadow-[rgba(50,50,93,0.25)_0px_50px_100px_-20px,rgba(0,0,0,0.3)_0px_30px_60px_-30px,rgba(10,37,64,0.35)_0px_-2px_6px_0px_inset]',
    // ghost：默认无边框低调，hover 时才显轮廓
    'btn-ghost': 'btn bg-transparent border border-transparent text-[#6E6863] hover:border-line/40 hover:bg-white/5 hover:text-[#3E3E42]',
    // 危险按钮：玫瑰红系，hover 淡红背景强化危险感知
    'btn-danger': 'btn bg-transparent text-rose-400 border border-transparent hover:text-rose-300 hover:bg-rose-500/10 hover:border-rose-500/20 text-xs',
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
