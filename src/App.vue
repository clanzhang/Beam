<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import QRCode from 'qrcode'
import {
  apiBase,
  deleteFile,
  formatSize,
  formatTime,
  getClipboard,
  getServerInfo,
  importFiles,
  listFiles,
  openDir,
  pickDir,
  readSystemClipboard,
  setClipboard,
  setDir,
  type ClipboardEntry,
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
const dragging = ref(false)
const searchQuery = ref('')
const sortBy = ref<'time' | 'name' | 'size'>('time')

const clipText = ref('')
const remoteClip = ref<ClipboardEntry>({ text: '', updated_at: 0 })
const autoSync = ref(false)
const clipSent = ref(false)

let wsConn: WebSocket | null = null
let unlistenDrag: (() => void) | undefined

const totalSize = computed(() => files.value.reduce((s, f) => s + f.size, 0))

const filteredFiles = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  let list = q ? files.value.filter((f) => f.name.toLowerCase().includes(q)) : [...files.value]
  if (sortBy.value === 'name') list.sort((a, b) => a.name.localeCompare(b.name, 'zh'))
  else if (sortBy.value === 'size') list.sort((a, b) => a.size - b.size)
  else list.sort((a, b) => b.modified - a.modified)
  return list
})
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
  connectWs()
  setupDragDrop()
  loadClipboard()
  timer = window.setInterval(refresh, 5000)
}

/** WebSocket 实时推送：收到 files_changed 立即刷新（保留轮询作兜底） */
function connectWs() {
  if (!info.value || wsConn) return
  const socket = new WebSocket(`ws://127.0.0.1:${info.value.port}/t/${info.value.token}/api/ws`)
  socket.onmessage = (e) => {
    if (e.data === 'clipboard_changed') loadClipboard()
    else refresh()
  }
  socket.onclose = () => {
    wsConn = null
    setTimeout(connectWs, 3000)
  }
  socket.onerror = () => socket.close()
  wsConn = socket
}

/** 桌面拖拽发送：拖入文件 → 复制进共享目录 → 手机端实时可见 */
async function setupDragDrop() {
  const { getCurrentWebview } = await import('@tauri-apps/api/webview')
  unlistenDrag = await getCurrentWebview().onDragDropEvent((event) => {
    const t = event.payload.type
    if (t === 'over' || t === 'enter') {
      dragging.value = true
    } else if (t === 'leave') {
      dragging.value = false
    } else if (t === 'drop') {
      dragging.value = false
      importFiles(event.payload.paths).then((n) => {
        if (n > 0) refresh()
      })
    }
  })
}

const IMAGE_EXTS = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'heic', 'heif', 'bmp']
function isImage(name: string): boolean {
  const ext = (name.split('.').pop() || '').toLowerCase()
  return IMAGE_EXTS.includes(ext)
}
function thumbnailUrl(name: string): string {
  return `${apiBase()}/api/files/${encodeURIComponent(name)}`
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

async function loadClipboard() {
  try {
    remoteClip.value = await getClipboard()
  } catch {
    /* 忽略 */
  }
}

async function sendClipText() {
  const text = clipText.value
  if (!text.trim()) return
  try {
    await setClipboard(text)
    clipSent.value = true
    setTimeout(() => (clipSent.value = false), 1500)
    clipText.value = ''
    await loadClipboard()
  } catch {
    /* 忽略 */
  }
}

async function copyRemoteClip() {
  if (!remoteClip.value.text) return
  try {
    await navigator.clipboard.writeText(remoteClip.value.text)
  } catch {
    /* 忽略 */
  }
}

let clipTimer: number | undefined
let lastSystemClip = ''
// 开启自动同步后：每 2s 读取系统剪贴板，有变化就推到共享端
function syncSystemClipboard() {
  if (!autoSync.value) {
    if (clipTimer) {
      window.clearInterval(clipTimer)
      clipTimer = undefined
    }
    return
  }
  if (clipTimer) return
  clipTimer = window.setInterval(async () => {
    try {
      const text = await readSystemClipboard()
      if (text != null && text !== lastSystemClip && text.trim()) {
        lastSystemClip = text
        await setClipboard(text)
        await loadClipboard()
      }
    } catch {
      /* 忽略 */
    }
  }, 2000)
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

onMounted(() => {
  init()
})
onUnmounted(() => {
  if (timer) window.clearInterval(timer)
  if (clipTimer) window.clearInterval(clipTimer)
  unlistenDrag?.()
  wsConn?.close()
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
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-sm font-semibold text-slate-200 flex items-center gap-2">
              <span class="w-7 h-7 rounded-lg bg-slate-700/40 grid place-items-center text-sm">📋</span> 文字互传
            </h3>
            <label class="flex items-center gap-1.5 text-[11px] text-slate-400 cursor-pointer select-none">
              <input v-model="autoSync" type="checkbox" class="accent-amber-500" @change="syncSystemClipboard" />
              自动同步电脑剪贴板
            </label>
          </div>
          <div class="flex items-start gap-2">
            <textarea
              v-model="clipText"
              rows="2"
              placeholder="粘贴文字，发送到手机…"
              class="flex-1 text-xs text-slate-300 bg-[#0a0f1d] border border-[#334155] rounded-lg px-3 py-2 resize-none outline-none focus:border-amber-400/60 transition-colors"
            ></textarea>
            <button class="btn btn-primary" :disabled="!clipText.trim()" @click="sendClipText">
              {{ clipSent ? '✓ 已发送' : '发送' }}
            </button>
          </div>
          <div
            v-if="remoteClip.text"
            class="mt-3 flex items-start gap-2 bg-[#0a0f1d] border border-[#334155] rounded-lg px-3 py-2"
          >
            <p class="flex-1 text-xs text-slate-300 break-all leading-5 max-h-24 overflow-y-auto whitespace-pre-wrap">
              {{ remoteClip.text }}
            </p>
            <button class="btn btn-ghost shrink-0" @click="copyRemoteClip">复制</button>
          </div>
          <p v-else class="mt-3 text-[11px] text-slate-600">
            手机发来的文字会显示在这里；开启自动同步后，电脑复制的文字会实时推到手机。
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
            <div class="flex items-center gap-2">
              <input
                v-model="searchQuery"
                placeholder="🔍 搜索文件…"
                class="w-36 text-xs text-slate-300 bg-[#0a0f1d] border border-[#334155] rounded-lg px-2.5 py-1.5 outline-none focus:border-amber-400/60 transition-colors placeholder:text-slate-600"
              />
              <select
                v-model="sortBy"
                class="text-xs text-slate-300 bg-[#0a0f1d] border border-[#334155] rounded-lg px-2 py-1.5 outline-none cursor-pointer"
              >
                <option value="time">最新</option>
                <option value="name">名称</option>
                <option value="size">大小</option>
              </select>
              <button class="btn btn-ghost" @click="doRefresh" :disabled="refreshing">
                <span class="i-ri-refresh-line text-sm" :class="refreshing ? 'animate-spin' : ''"></span>
                刷新
              </button>
            </div>
          </div>

          <div v-if="files.length === 0" class="py-14 text-center">
            <div class="text-4xl mb-3">📭</div>
            <p class="text-sm text-slate-500">还没有文件，用手机扫左侧二维码传一个过来吧</p>
          </div>
          <div v-else-if="filteredFiles.length === 0" class="py-10 text-center">
            <p class="text-sm text-slate-500">没有匹配「{{ searchQuery }}」的文件</p>
          </div>

          <ul v-else class="divide-y divide-[#1f2b40]">
            <li
              v-for="f in filteredFiles"
              :key="f.name"
              class="group flex items-center gap-3 py-3 px-2 -mx-2 rounded-lg hover:bg-white/[0.03] transition-colors"
            >
              <div
                v-if="isImage(f.name)"
                class="w-10 h-10 rounded-lg overflow-hidden shrink-0 ring-1 ring-slate-700/70 bg-[#0a0f1d]"
              >
                <img :src="thumbnailUrl(f.name)" :alt="f.name" class="w-10 h-10 object-cover" loading="lazy" />
              </div>
              <div
                v-else
                class="w-10 h-10 rounded-lg grid place-items-center text-lg shrink-0"
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

    <!-- 拖拽发送遮罩 -->
    <div v-if="dragging" class="fixed inset-0 z-50 pointer-events-none">
      <div
        class="absolute inset-4 rounded-2xl border-2 border-dashed border-amber-400/70 bg-amber-400/10 backdrop-blur-sm flex flex-col items-center justify-center gap-3"
      >
        <div class="text-4xl">📤</div>
        <p class="text-lg font-semibold text-amber-300">松开鼠标，发送到共享目录</p>
        <p class="text-xs text-slate-400">文件会复制进共享目录，手机端立即可见可下载</p>
      </div>
    </div>
  </div>
</template>
