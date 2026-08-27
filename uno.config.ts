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
    'btn': 'inline-flex items-center justify-center gap-1.5 min-h-8 px-3.5 py-2 rounded-2xl text-sm font-medium transition-all duration-150 active:scale-[0.97] select-none disabled:opacity-50 disabled:cursor-not-allowed disabled:pointer-events-none',
    // 主按钮：陶土橘实心 + 3D 悬浮阴影
    'btn-primary': 'btn text-white bg-[#D17159] hover:bg-[#C06047] font-semibold shadow-[rgba(50,50,93,0.25)_0px_50px_100px_-20px,rgba(0,0,0,0.3)_0px_30px_60px_-30px,rgba(10,37,64,0.35)_0px_-2px_6px_0px_inset]',
    // 次级按钮：白底描边，hover 有明显反馈（描边/文字/底色三处变化）
    'btn-outline': 'btn bg-white border border-[#E3D9CF] text-[#3E3E42] hover:border-[#D17159]/60 hover:text-[#C06047] hover:bg-[#FBF6F2] shadow-[rgba(50,50,93,0.25)_0px_50px_100px_-20px,rgba(0,0,0,0.3)_0px_30px_60px_-30px,rgba(10,37,64,0.35)_0px_-2px_6px_0px_inset]',
    // ghost：带细边框的轻量按钮（刷新等低优先级操作）
    'btn-ghost': 'btn bg-white/70 border border-[#E3D9CF]/70 text-[#6E6863] hover:bg-[#FBF6F2] hover:text-[#3E3E42] hover:border-[#D5C4B5]',
    // 危险按钮：玫红系，hover 加深背景
    'btn-danger': 'btn bg-white border border-[#E8C4B8] text-[#C06047] hover:bg-rose-50 hover:border-rose-300 hover:text-rose-600 text-xs',
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
