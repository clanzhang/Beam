<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import QRCode from 'qrcode'
import {
  deleteFile,
  formatSize,
  formatTime,
  getServerInfo,
  listFiles,
  openDir,
  pickDir,
  setDir,
  type FileInfo,
  type ServerInfo,
} from './api'

const info = ref<ServerInfo | null>(null)
const files = ref<FileInfo[]>([])
const qrDataUrl = ref('')
const copied = ref(false)
const picking = ref(false)
const busy = ref(false)

const totalSize = computed(() => files.value.reduce((s, f) => s + f.size, 0))
const statusText = computed(() =>
  info.value ? `服务运行中 · ${info.value.lan_ip}:${info.value.port}` : '启动中…',
)

let timer: number | undefined

async function refresh() {
  try {
    files.value = await listFiles()
  } catch {
    /* 服务偶发抖动不打扰用户 */
  }
}

async function init() {
  info.value = await getServerInfo()
  await refresh()
  const opts = { width: 230, margin: 1, color: { dark: '#0b1226ff', light: '#ffffffff' } }
  qrDataUrl.value = await QRCode.toDataURL(info.value.url, opts)
  timer = window.setInterval(refresh, 5000)
}

async function copyUrl() {
  if (!info.value) return
  try {
    await navigator.clipboard.writeText(info.value.url)
    copied.value = true
    setTimeout(() => (copied.value = false), 1500)
  } catch {
    /* webview 剪贴板被拒时静默失败 */
  }
}

async function chooseDir() {
  picking.value = true
  try {
    const dir = await pickDir()
    if (dir && info.value) {
      await setDir(dir)
      info.value.dir = dir
      await refresh()
    }
  } finally {
    picking.value = false
  }
}

async function remove(name: string) {
  if (!window.confirm(`确定删除「${name}」？`)) return
  busy.value = true
  try {
    await deleteFile(name)
    await refresh()
  } finally {
    busy.value = false
  }
}

function reveal() {
  if (info.value) openDir(info.value.dir)
}

function fileTint(name: string): string {
  const ext = (name.split('.').pop() || '').toLowerCase()
  if (['jpg', 'jpeg', 'png', 'gif', 'webp', 'heic', 'bmp'].includes(ext)) return 'from-cyan-500/25 to-cyan-400/10 text-cyan-300'
  if (['mp4', 'mov', 'avi', 'mkv', 'webm'].includes(ext)) return 'from-purple-500/25 to-purple-400/10 text-purple-300'
  if (['mp3', 'wav', 'm4a', 'flac'].includes(ext)) return 'from-pink-500/25 to-pink-400/10 text-pink-300'
  if (['zip', 'rar', '7z', 'tar', 'gz'].includes(ext)) return 'from-amber-500/25 to-amber-400/10 text-amber-300'
  if (['pdf'].includes(ext)) return 'from-rose-500/25 to-rose-400/10 text-rose-300'
  if (['doc', 'docx', 'txt', 'md'].includes(ext)) return 'from-sky-500/25 to-sky-400/10 text-sky-300'
  if (['xls', 'xlsx', 'csv'].includes(ext)) return 'from-emerald-500/25 to-emerald-400/10 text-emerald-300'
  return 'from-slate-500/25 to-slate-400/10 text-slate-300'
}

function fileIcon(name: string): string {
  const ext = (name.split('.').pop() || '').toLowerCase()
  const map: Record<string, string> = {
    jpg: '🖼️', jpeg: '🖼️', png: '🖼️', gif: '🖼️', webp: '🖼️', heic: '🖼️',
    mp4: '🎬', mov: '🎬', avi: '🎬', mkv: '🎬',
    mp3: '🎵', wav: '🎵', m4a: '🎵',
    zip: '🗜️', rar: '🗜️', '7z': '🗜️',
    pdf: '📄', doc: '📝', docx: '📝', txt: '📃', md: '📃',
    xls: '📊', xlsx: '📊', csv: '📊',
  }
  return map[ext] || '📦'
}

onMounted(init)
onUnmounted(() => {
  if (timer) window.clearInterval(timer)
})
</script>

<template>
  <div
    class="min-h-screen text-slate-100"
    :class="'bg-[radial-gradient(1100px_480px_at_50%_-12%,#1d3a6e_0%,rgba(13,23,44,0)_60%)],bg-[#0b1226]'"
    style="background: radial-gradient(1100px 480px at 50% -12%, #1d3a6e 0%, rgba(13,23,44,0) 60%), linear-gradient(180deg, #0e1730 0%, #0b1226 100%)"
  >
    <header
      class="sticky top-0 z-10 border-b border-[#26375c]/60 bg-[#0d1630]/80 backdrop-blur-md px-6 py-4 flex items-center gap-3"
    >
      <div
        class="w-10 h-10 rounded-xl bg-gradient-to-br from-cyan-400 via-sky-400 to-indigo-500 grid place-items-center text-slate-950 font-extrabold text-base shadow-lg shadow-cyan-500/30"
      >
        AB
      </div>
      <div>
        <h1 class="text-base font-bold tracking-wide">
          AirBox <span class="text-cyan-300">·</span>
          <span class="text-slate-300">局域网快传</span>
        </h1>
        <p class="text-[11px] text-slate-400 mt-0.5">手机扫码即连 · 双向互传 · 文件不出局域网</p>
      </div>
      <div class="ml-auto flex items-center gap-2">
        <span
          class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-emerald-400/10 text-emerald-300 border border-emerald-400/30 text-xs font-medium"
        >
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse shadow-[0_0_8px_#34d399]"></span>
          {{ statusText }}
        </span>
      </div>
    </header>

    <main class="p-6 grid grid-cols-[minmax(310px,380px)_1fr] gap-6">
      <!-- 左列：二维码 + 链接 -->
      <section class="space-y-6">
        <div
          class="rounded-2xl border border-[#2a3d6b] bg-gradient-to-b from-[#182947] to-[#12203c] p-6 flex flex-col items-center shadow-[0_16px_40px_rgba(0,0,0,0.35)]"
        >
          <h2 class="text-sm font-semibold text-slate-200 self-start mb-4 flex items-center gap-2">
            <span class="text-base">📱</span> 手机扫码传文件
          </h2>
          <div
            class="w-56 h-56 rounded-2xl bg-white p-3 shadow-[0_0_50px_rgba(34,211,238,0.25),0_8px_24px_rgba(0,0,0,0.3)] ring-1 ring-cyan-300/40 flex items-center justify-center"
          >
            <img v-if="qrDataUrl" :src="qrDataUrl" alt="扫码连接" class="w-full h-full" />
            <div v-else class="text-slate-400 text-sm">二维码生成中…</div>
          </div>
          <p class="mt-4 text-xs text-slate-400 text-center leading-5">
            手机连上<b class="text-cyan-300">同一个 Wi-Fi / 热点</b>，用相机或微信扫码
            <br />即可互传文件，无需安装 App
          </p>
        </div>

        <div
          class="rounded-2xl border border-[#2a3d6b] bg-gradient-to-b from-[#182947] to-[#12203c] p-5 shadow-[0_16px_40px_rgba(0,0,0,0.35)]"
        >
          <h3 class="text-sm font-semibold text-slate-200 mb-3">连接地址</h3>
          <div class="flex items-center gap-2">
            <code
              class="flex-1 text-xs text-cyan-300 bg-[#0a1326] border border-[#2a3d6b] rounded-lg px-3 py-2.5 truncate font-mono"
            >
              {{ info?.url || '…' }}
            </code>
            <button
              class="shrink-0 inline-flex items-center gap-1.5 px-4 py-2.5 rounded-lg text-sm font-bold text-[#062033] bg-gradient-to-r from-cyan-300 to-sky-400 hover:from-cyan-200 hover:to-sky-300 active:scale-95 transition-all shadow-[0_4px_16px_rgba(34,211,238,0.35)] disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="!info"
              @click="copyUrl"
            >
              {{ copied ? '✓ 已复制' : '复制' }}
            </button>
          </div>
          <p class="mt-3 text-[11px] text-slate-500 leading-4">
            链接含随机令牌，仅本次会话有效 · 只有扫码 / 知道链接的人能访问
          </p>
        </div>
      </section>

      <!-- 右列：收件目录 + 文件列表 -->
      <section class="space-y-6 min-w-0">
        <div
          class="rounded-2xl border border-[#2a3d6b] bg-gradient-to-b from-[#182947] to-[#12203c] p-5 shadow-[0_16px_40px_rgba(0,0,0,0.35)]"
        >
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-sm font-semibold text-slate-200 flex items-center gap-2">
              <span>📂</span> 收件目录
            </h3>
            <div class="flex gap-2">
              <button
                class="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg text-sm font-semibold text-indigo-300 bg-indigo-500/10 border border-indigo-400/40 hover:bg-indigo-500/20 active:scale-95 transition-all disabled:opacity-50"
                @click="chooseDir"
                :disabled="picking"
              >
                {{ picking ? '选择中…' : '选择目录' }}
              </button>
              <button
                class="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg text-sm font-semibold text-cyan-300 bg-cyan-500/10 border border-cyan-400/40 hover:bg-cyan-500/20 active:scale-95 transition-all disabled:opacity-50"
                @click="reveal"
                :disabled="!info"
              >
                打开目录
              </button>
            </div>
          </div>
          <p class="text-xs text-slate-300 bg-[#0a1326] border border-[#2a3d6b] rounded-lg px-3 py-2.5 truncate">
            📍 {{ info?.dir || '加载中…' }}
          </p>
        </div>

        <div
          class="rounded-2xl border border-[#2a3d6b] bg-gradient-to-b from-[#182947] to-[#12203c] p-5 shadow-[0_16px_40px_rgba(0,0,0,0.35)]"
        >
          <div class="flex items-center justify-between mb-2">
            <h3 class="text-sm font-semibold text-slate-200 flex items-center gap-2">
              <span>🗂️</span> 已收文件
              <span class="text-[11px] font-normal text-slate-400">
                {{ files.length ? `${files.length} 个 · 共 ${formatSize(totalSize)}` : '' }}
              </span>
            </h3>
            <button
              class="inline-flex items-center gap-1 px-3 py-1.5 rounded-lg text-xs font-semibold text-cyan-300 bg-cyan-500/10 border border-cyan-400/30 hover:bg-cyan-500/20 active:scale-95 transition-all"
              @click="refresh"
            >
              ⟳ 刷新
            </button>
          </div>

          <div v-if="files.length === 0" class="py-14 text-center">
            <div class="text-4xl mb-3">📭</div>
            <p class="text-sm text-slate-400">还没有文件，用手机扫左侧二维码传一个过来吧</p>
          </div>

          <ul v-else class="divide-y divide-[#26375c]/60">
            <li v-for="f in files" :key="f.name" class="flex items-center gap-3 py-3">
              <div
                class="w-10 h-10 rounded-xl bg-gradient-to-br grid place-items-center text-lg shrink-0 border border-white/5"
                :class="fileTint(f.name)"
              >
                {{ fileIcon(f.name) }}
              </div>
              <div class="min-w-0 flex-1">
                <p class="text-sm font-semibold text-slate-100 truncate">{{ f.name }}</p>
                <p class="text-[11px] text-slate-400 mt-0.5">{{ formatSize(f.size) }} · {{ formatTime(f.modified) }}</p>
              </div>
              <button
                class="inline-flex items-center gap-1 px-3 py-1.5 rounded-lg text-xs font-semibold text-rose-300 bg-rose-500/10 border border-rose-400/30 hover:bg-rose-500/20 active:scale-95 transition-all disabled:opacity-50"
                :disabled="busy"
                @click="remove(f.name)"
              >
                删除
              </button>
            </li>
          </ul>
        </div>
      </section>
    </main>

    <footer class="px-6 pb-5 text-[11px] text-slate-500">
      文件保存在本地收件目录，不上传任何第三方服务器 · 关闭 AirBox 即断开连接
    </footer>
  </div>
</template>
