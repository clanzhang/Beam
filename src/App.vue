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
  const opts = { width: 230, margin: 1, color: { dark: '#1e3a8aff', light: '#ffffffff' } }
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
  if (['jpg', 'jpeg', 'png', 'gif', 'webp', 'heic', 'bmp'].includes(ext)) return 'bg-cyan-100 text-cyan-600'
  if (['mp4', 'mov', 'avi', 'mkv', 'webm'].includes(ext)) return 'bg-purple-100 text-purple-600'
  if (['mp3', 'wav', 'm4a', 'flac'].includes(ext)) return 'bg-pink-100 text-pink-600'
  if (['zip', 'rar', '7z', 'tar', 'gz'].includes(ext)) return 'bg-amber-100 text-amber-600'
  if (['pdf'].includes(ext)) return 'bg-rose-100 text-rose-600'
  if (['doc', 'docx', 'txt', 'md'].includes(ext)) return 'bg-sky-100 text-sky-600'
  if (['xls', 'xlsx', 'csv'].includes(ext)) return 'bg-emerald-100 text-emerald-600'
  return 'bg-slate-100 text-slate-500'
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
    class="min-h-screen text-slate-700"
    style="background-image: linear-gradient(120deg, #a1c4fd 0%, #c2e9fb 100%);"
  >
    <header
      class="sticky top-0 z-10 border-b border-white/60 bg-white/60 backdrop-blur-md px-6 py-4 flex items-center gap-3"
    >
      <div
        class="w-10 h-10 rounded-xl bg-gradient-to-br from-sky-500 via-indigo-500 to-indigo-600 grid place-items-center text-white font-extrabold text-base shadow-lg shadow-indigo-400/40"
      >
        AB
      </div>
      <div>
        <h1 class="text-base font-bold tracking-wide text-slate-800">
          AirBox <span class="text-sky-500">·</span> 局域网快传
        </h1>
        <p class="text-[11px] text-slate-500 mt-0.5">手机扫码即连 · 双向互传 · 文件不出局域网</p>
      </div>
      <div class="ml-auto flex items-center gap-2">
        <span
          class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-emerald-50 text-emerald-600 border border-emerald-300 text-xs font-semibold"
        >
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse shadow-[0_0_8px_#10b981]"></span>
          {{ statusText }}
        </span>
      </div>
    </header>

    <main class="p-6 grid grid-cols-[minmax(310px,380px)_1fr] gap-6">
      <!-- 左列：二维码 + 链接 -->
      <section class="space-y-6">
        <div
          class="rounded-2xl border border-white/70 bg-white/85 backdrop-blur p-6 flex flex-col items-center shadow-[0_16px_40px_rgba(99,102,241,0.15)]"
        >
          <h2 class="text-sm font-bold text-slate-700 self-start mb-4 flex items-center gap-2">
            <span class="text-base">📱</span> 手机扫码传文件
          </h2>
          <div
            class="w-56 h-56 rounded-2xl bg-white p-3 shadow-[0_0_40px_rgba(56,189,248,0.35),0_8px_24px_rgba(30,64,175,0.15)] ring-1 ring-sky-300/60 flex items-center justify-center"
          >
            <img v-if="qrDataUrl" :src="qrDataUrl" alt="扫码连接" class="w-full h-full" />
            <div v-else class="text-slate-400 text-sm">二维码生成中…</div>
          </div>
          <p class="mt-4 text-xs text-slate-500 text-center leading-5">
            手机连上<b class="text-indigo-600">同一个 Wi-Fi / 热点</b>，用相机或微信扫码
            <br />即可互传文件，无需安装 App
          </p>
        </div>

        <div
          class="rounded-2xl border border-white/70 bg-white/85 backdrop-blur p-5 shadow-[0_16px_40px_rgba(99,102,241,0.15)]"
        >
          <h3 class="text-sm font-bold text-slate-700 mb-3">连接地址</h3>
          <div class="flex items-center gap-2">
            <code
              class="flex-1 text-xs text-indigo-700 bg-white border border-slate-200 rounded-lg px-3 py-2.5 truncate font-mono"
            >
              {{ info?.url || '…' }}
            </code>
            <button
              class="shrink-0 inline-flex items-center gap-1.5 px-4 py-2.5 rounded-lg text-sm font-bold text-white bg-gradient-to-r from-sky-500 to-indigo-500 hover:from-sky-400 hover:to-indigo-400 active:scale-95 transition-all shadow-[0_4px_16px_rgba(59,130,246,0.4)] disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="!info"
              @click="copyUrl"
            >
              {{ copied ? '✓ 已复制' : '复制' }}
            </button>
          </div>
          <p class="mt-3 text-[11px] text-slate-400 leading-4">
            链接含随机令牌，仅本次会话有效 · 只有扫码 / 知道链接的人能访问
          </p>
        </div>
      </section>

      <!-- 右列：收件目录 + 文件列表 -->
      <section class="space-y-6 min-w-0">
        <div
          class="rounded-2xl border border-white/70 bg-white/85 backdrop-blur p-5 shadow-[0_16px_40px_rgba(99,102,241,0.15)]"
        >
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-sm font-bold text-slate-700 flex items-center gap-2">
              <span>📂</span> 收件目录
            </h3>
            <div class="flex gap-2">
              <button
                class="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg text-sm font-semibold text-indigo-600 bg-indigo-50 border border-indigo-200 hover:bg-indigo-100 active:scale-95 transition-all disabled:opacity-50"
                @click="chooseDir"
                :disabled="picking"
              >
                {{ picking ? '选择中…' : '选择目录' }}
              </button>
              <button
                class="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg text-sm font-semibold text-sky-600 bg-sky-50 border border-sky-200 hover:bg-sky-100 active:scale-95 transition-all disabled:opacity-50"
                @click="reveal"
                :disabled="!info"
              >
                打开目录
              </button>
            </div>
          </div>
          <p class="text-xs text-slate-600 bg-white border border-slate-200 rounded-lg px-3 py-2.5 truncate">
            📍 {{ info?.dir || '加载中…' }}
          </p>
        </div>

        <div
          class="rounded-2xl border border-white/70 bg-white/85 backdrop-blur p-5 shadow-[0_16px_40px_rgba(99,102,241,0.15)]"
        >
          <div class="flex items-center justify-between mb-2">
            <h3 class="text-sm font-bold text-slate-700 flex items-center gap-2">
              <span>🗂️</span> 已收文件
              <span class="text-[11px] font-normal text-slate-400">
                {{ files.length ? `${files.length} 个 · 共 ${formatSize(totalSize)}` : '' }}
              </span>
            </h3>
            <button
              class="inline-flex items-center gap-1 px-3 py-1.5 rounded-lg text-xs font-semibold text-sky-600 bg-sky-50 border border-sky-200 hover:bg-sky-100 active:scale-95 transition-all"
              @click="refresh"
            >
              ⟳ 刷新
            </button>
          </div>

          <div v-if="files.length === 0" class="py-14 text-center">
            <div class="text-4xl mb-3">📭</div>
            <p class="text-sm text-slate-500">还没有文件，用手机扫左侧二维码传一个过来吧</p>
          </div>

          <ul v-else class="divide-y divide-slate-100">
            <li v-for="f in files" :key="f.name" class="flex items-center gap-3 py-3">
              <div
                class="w-10 h-10 rounded-xl grid place-items-center text-lg shrink-0 border border-white/80 shadow-sm"
                :class="fileTint(f.name)"
              >
                {{ fileIcon(f.name) }}
              </div>
              <div class="min-w-0 flex-1">
                <p class="text-sm font-semibold text-slate-700 truncate">{{ f.name }}</p>
                <p class="text-[11px] text-slate-400 mt-0.5">{{ formatSize(f.size) }} · {{ formatTime(f.modified) }}</p>
              </div>
              <button
                class="inline-flex items-center gap-1 px-3 py-1.5 rounded-lg text-xs font-semibold text-rose-500 bg-rose-50 border border-rose-200 hover:bg-rose-100 active:scale-95 transition-all disabled:opacity-50"
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

    <footer class="px-6 pb-5 text-[11px] text-slate-400/80">
      文件保存在本地收件目录，不上传任何第三方服务器 · 关闭 AirBox 即断开连接
    </footer>
  </div>
</template>
