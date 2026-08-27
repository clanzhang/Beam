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
const refreshing = ref(false)

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

async function doRefresh() {
  refreshing.value = true
  try {
    // 至少 0.5s 的 loading 反馈，刷新完成后自动停止
    await Promise.all([refresh(), new Promise((r) => setTimeout(r, 500))])
  } finally {
    refreshing.value = false
  }
}

async function init() {
  info.value = await getServerInfo()
  await refresh()
  const opts = { width: 230, margin: 1, color: { dark: '#0b1220ff', light: '#ffffffff' } }
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
  if (['jpg', 'jpeg', 'png', 'gif', 'webp', 'heic', 'bmp'].includes(ext)) return 'bg-cyan-500/10 text-cyan-300'
  if (['mp4', 'mov', 'avi', 'mkv', 'webm'].includes(ext)) return 'bg-purple-500/10 text-purple-300'
  if (['mp3', 'wav', 'm4a', 'flac'].includes(ext)) return 'bg-pink-500/10 text-pink-300'
  if (['zip', 'rar', '7z', 'tar', 'gz'].includes(ext)) return 'bg-amber-500/10 text-amber-300'
  if (['pdf'].includes(ext)) return 'bg-rose-500/10 text-rose-300'
  if (['doc', 'docx', 'txt', 'md'].includes(ext)) return 'bg-sky-500/10 text-sky-300'
  if (['xls', 'xlsx', 'csv'].includes(ext)) return 'bg-emerald-500/10 text-emerald-300'
  return 'bg-slate-500/10 text-slate-300'
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
    class="min-h-screen text-slate-200"
    style="background: radial-gradient(1100px 480px at 50% -12%, #1d2c4d 0%, rgba(11, 18, 32, 0) 60%), #0b1220"
  >
    <!-- 顶部头栏 -->
    <header
      class="sticky top-0 z-10 border-b border-[#1f2b40] bg-[#0d1526]/85 backdrop-blur-md px-6 py-4 flex items-center gap-3"
    >
      <div
        class="w-10 h-10 rounded-xl bg-gradient-to-br from-amber-400 to-amber-600 grid place-items-center text-slate-950 font-extrabold text-base shadow-lg shadow-amber-500/25"
      >
        AB
      </div>
      <div>
        <h1 class="text-base font-bold tracking-wide text-slate-100">
          AirBox <span class="text-amber-400">·</span> 局域网快传
        </h1>
        <p class="text-[11px] text-slate-500 mt-0.5">手机扫码即连 · 双向互传 · 文件不出局域网</p>
      </div>
      <div class="ml-auto flex items-center gap-2">
        <span
          class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 text-xs font-medium"
        >
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse shadow-[0_0_8px_#34d399]"></span>
          {{ statusText }}
        </span>
      </div>
    </header>

    <main class="p-6 grid grid-cols-[minmax(310px,380px)_1fr] gap-6 max-w-6xl mx-auto">
      <!-- 左列：二维码 + 链接 -->
      <section class="space-y-6">
        <div class="rounded-2xl border border-[#334155] bg-[#0f172a] p-6 flex flex-col items-center shadow-[0_16px_40px_rgba(0,0,0,0.35)]">
          <h2 class="text-sm font-semibold text-slate-200 self-start mb-4 flex items-center gap-2">
            <span class="w-7 h-7 rounded-lg bg-slate-700/40 grid place-items-center text-sm">📱</span> 手机扫码传文件
          </h2>
          <div
            class="w-56 h-56 rounded-2xl bg-white p-3 shadow-[0_0_40px_rgba(245,158,11,0.15),0_8px_24px_rgba(0,0,0,0.3)] ring-1 ring-slate-600 flex items-center justify-center"
          >
            <img v-if="qrDataUrl" :src="qrDataUrl" alt="扫码连接" class="w-full h-full" />
            <div v-else class="text-slate-400 text-sm">二维码生成中…</div>
          </div>
          <p class="mt-4 text-xs text-slate-500 text-center leading-5">
            手机连上<b class="text-amber-400">同一个 Wi-Fi / 热点</b>，用相机或微信扫码
            <br />即可互传文件，无需安装 App
          </p>
        </div>

        <div class="rounded-2xl border border-[#334155] bg-[#0f172a] p-5 shadow-[0_16px_40px_rgba(0,0,0,0.35)]">
          <h3 class="text-sm font-semibold text-slate-200 mb-3">连接地址</h3>
          <div class="flex items-center gap-2">
            <code
              class="flex-1 text-xs text-amber-300 bg-[#0a0f1d] border border-[#334155] rounded-lg px-3 py-2.5 truncate font-mono"
            >
              {{ info?.url || '…' }}
            </code>
            <button class="shrink-0 btn btn-primary px-5" :disabled="!info" @click="copyUrl">
              {{ copied ? '✓ 已复制' : '复制' }}
            </button>
          </div>
          <p class="mt-3 text-[11px] text-slate-600 leading-4">
            链接含随机令牌，仅本次会话有效 · 只有扫码 / 知道链接的人能访问
          </p>
        </div>
      </section>

      <!-- 右列：收件目录 + 文件列表 -->
      <section class="space-y-6 min-w-0">
        <div class="rounded-2xl border border-[#334155] bg-[#0f172a] p-5 shadow-[0_16px_40px_rgba(0,0,0,0.35)]">
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-sm font-semibold text-slate-200 flex items-center gap-2">
              <span class="w-7 h-7 rounded-lg bg-slate-700/40 grid place-items-center text-sm">📂</span> 收件目录
            </h3>
            <div class="flex gap-2.5">
              <button class="btn btn-outline" @click="chooseDir" :disabled="picking">
                {{ picking ? '选择中…' : '选择目录' }}
              </button>
              <button class="btn btn-primary" @click="reveal" :disabled="!info">打开目录</button>
            </div>
          </div>
          <p class="text-xs text-slate-300 bg-[#0a0f1d] border border-[#334155] rounded-lg px-3 py-2.5 truncate">
            📍 {{ info?.dir || '加载中…' }}
          </p>
        </div>

        <div class="rounded-2xl border border-[#334155] bg-[#0f172a] p-5 shadow-[0_16px_40px_rgba(0,0,0,0.35)]">
          <div class="flex items-center justify-between mb-2">
            <h3 class="text-sm font-semibold text-slate-200 flex items-center gap-2">
              <span class="w-7 h-7 rounded-lg bg-slate-700/40 grid place-items-center text-sm">🗂️</span> 已收文件
              <span class="text-[11px] font-normal text-slate-500">
                {{ files.length ? `${files.length} 个 · 共 ${formatSize(totalSize)}` : '' }}
              </span>
            </h3>
            <button class="btn btn-ghost" @click="doRefresh" :disabled="refreshing">
              <span class="i-ri-refresh-line text-sm" :class="refreshing ? 'animate-spin' : ''"></span>
              刷新
            </button>
          </div>

          <div v-if="files.length === 0" class="py-14 text-center">
            <div class="text-4xl mb-3">📭</div>
            <p class="text-sm text-slate-500">还没有文件，用手机扫左侧二维码传一个过来吧</p>
          </div>

          <ul v-else class="divide-y divide-[#1f2b40]">
            <li
              v-for="f in files"
              :key="f.name"
              class="group flex items-center gap-3 py-3 px-2 -mx-2 rounded-lg hover:bg-white/[0.03] transition-colors"
            >
              <div
                class="w-10 h-10 rounded-xl grid place-items-center text-lg shrink-0"
                :class="fileTint(f.name)"
              >
                {{ fileIcon(f.name) }}
              </div>
              <div class="min-w-0 flex-1">
                <p class="text-sm font-medium text-slate-200 truncate">{{ f.name }}</p>
                <p class="text-[11px] text-slate-500 mt-0.5">{{ formatSize(f.size) }} · {{ formatTime(f.modified) }}</p>
              </div>
              <button
                class="btn btn-danger opacity-50 group-hover:opacity-100 focus-visible:opacity-100 transition-opacity duration-150 ml-auto"
                :disabled="busy"
                @click="remove(f.name)"
                title="删除"
              >
                <span class="i-ri-delete-bin-5-line text-sm"></span>
                删除
              </button>
            </li>
          </ul>
        </div>
      </section>
    </main>

    <footer class="mt-4 bg-[#0a0f1d] border-t border-[#1f2b40] text-slate-600 px-6 py-4 flex items-center justify-between text-[11px]">
      <span>文件保存在本地收件目录，不上传任何第三方服务器</span>
      <span>关闭 AirBox 即断开连接</span>
    </footer>
  </div>
</template>
