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
    await refresh()
  } finally {
    refreshing.value = false
  }
}

async function init() {
  info.value = await getServerInfo()
  await refresh()
  const opts = { width: 230, margin: 1, color: { dark: '#3E3E42ff', light: '#ffffffff' } }
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
  if (['jpg', 'jpeg', 'png', 'gif', 'webp', 'heic', 'bmp'].includes(ext)) return 'bg-cyan-50 text-cyan-600'
  if (['mp4', 'mov', 'avi', 'mkv', 'webm'].includes(ext)) return 'bg-purple-50 text-purple-600'
  if (['mp3', 'wav', 'm4a', 'flac'].includes(ext)) return 'bg-pink-50 text-pink-600'
  if (['zip', 'rar', '7z', 'tar', 'gz'].includes(ext)) return 'bg-amber-50 text-amber-600'
  if (['pdf'].includes(ext)) return 'bg-rose-50 text-rose-600'
  if (['doc', 'docx', 'txt', 'md'].includes(ext)) return 'bg-sky-50 text-sky-600'
  if (['xls', 'xlsx', 'csv'].includes(ext)) return 'bg-emerald-50 text-emerald-600'
  return 'bg-orange-50 text-orange-600/80'
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
  <div class="min-h-screen text-[#3E3E42]" style="background-color: #EFEDEA">
    <!-- 顶部白色头栏 -->
    <header
      class="sticky top-0 z-10 border-b border-[#E7E0D9] bg-white/85 backdrop-blur-md px-6 py-4 flex items-center gap-3"
    >
      <div
        class="w-10 h-10 rounded-2xl bg-gradient-to-br from-[#E08B6B] via-[#D17159] to-[#B85C44] grid place-items-center text-white font-extrabold text-base shadow-[0_6px_16px_rgba(209,113,89,0.35)]"
      >
        AB
      </div>
      <div>
        <h1 class="text-base font-bold tracking-wide text-[#3E3E42]">
          AirBox <span class="text-[#D17159]">·</span> 局域网快传
        </h1>
        <p class="text-[11px] text-[#8A857F] mt-0.5">手机扫码即连 · 双向互传 · 文件不出局域网</p>
      </div>
      <div class="ml-auto flex items-center gap-2">
        <span
          class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-[#F6E3DD] text-[#C06047] border border-[#E8C4B8] text-xs font-semibold"
        >
          <span class="w-1.5 h-1.5 rounded-full bg-[#D17159] animate-pulse shadow-[0_0_8px_#D17159]"></span>
          {{ statusText }}
        </span>
      </div>
    </header>

    <main class="p-6 grid grid-cols-[minmax(310px,380px)_1fr] gap-6 max-w-6xl mx-auto">
      <!-- 左列：二维码 + 链接 -->
      <section class="space-y-6">
        <div
          class="rounded-3xl border border-[#EAE2DA] bg-white p-6 flex flex-col items-center shadow-[0_20px_50px_rgba(107,78,52,0.10)] relative overflow-hidden"
        >
          <div class="absolute inset-x-0 top-0 h-1.5 bg-gradient-to-r from-[#E08B6B] via-[#D17159] to-[#B85C44]"></div>
          <h2 class="text-sm font-bold text-[#3E3E42] self-start mb-4 flex items-center gap-2">
            <span class="w-7 h-7 rounded-lg bg-[#F6E3DD] grid place-items-center text-sm">📱</span> 手机扫码传文件
          </h2>
          <div
            class="w-56 h-56 rounded-2xl bg-white p-3 shadow-[0_0_40px_rgba(209,113,89,0.22),0_8px_24px_rgba(107,78,52,0.12)] ring-1 ring-[#E8C4B8] flex items-center justify-center"
          >
            <img v-if="qrDataUrl" :src="qrDataUrl" alt="扫码连接" class="w-full h-full" />
            <div v-else class="text-slate-400 text-sm">二维码生成中…</div>
          </div>
          <p class="mt-4 text-xs text-[#8A857F] text-center leading-5">
            手机连上<b class="text-[#C06047]">同一个 Wi-Fi / 热点</b>，用相机或微信扫码
            <br />即可互传文件，无需安装 App
          </p>
        </div>

        <div
          class="rounded-3xl border border-[#EAE2DA] bg-white p-5 shadow-[0_20px_50px_rgba(107,78,52,0.10)]"
        >
          <h3 class="text-sm font-bold text-[#3E3E42] mb-3">连接地址</h3>
          <div class="flex items-center gap-2">
            <code
              class="flex-1 text-xs text-[#B85C44] bg-[#FBF6F2] border border-[#EEE5DC] rounded-xl px-3 py-2.5 truncate font-mono"
            >
              {{ info?.url || '…' }}
            </code>
            <button class="shrink-0 btn btn-primary px-5" :disabled="!info" @click="copyUrl">
              {{ copied ? '✓ 已复制' : '复制' }}
            </button>
          </div>
          <p class="mt-3 text-[11px] text-[#A69E97] leading-4">
            链接含随机令牌，仅本次会话有效 · 只有扫码 / 知道链接的人能访问
          </p>
        </div>
      </section>

      <!-- 右列：收件目录 + 文件列表 -->
      <section class="space-y-6 min-w-0">
        <div class="rounded-3xl border border-[#EAE2DA] bg-white p-5 shadow-[0_20px_50px_rgba(107,78,52,0.10)]">
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-sm font-bold text-[#3E3E42] flex items-center gap-2">
              <span class="w-7 h-7 rounded-lg bg-[#F6E3DD] grid place-items-center text-sm">📂</span> 收件目录
            </h3>
            <div class="flex gap-2">
              <button class="btn btn-outline" @click="chooseDir" :disabled="picking">
                {{ picking ? '选择中…' : '选择目录' }}
              </button>
              <button class="btn btn-primary" @click="reveal" :disabled="!info">打开目录</button>
            </div>
          </div>
          <p class="text-xs text-[#6E6863] bg-[#FBF6F2] border border-[#EEE5DC] rounded-xl px-3 py-2.5 truncate">
            📍 {{ info?.dir || '加载中…' }}
          </p>
        </div>

        <div class="rounded-3xl border border-[#EAE2DA] bg-white p-5 shadow-[0_20px_50px_rgba(107,78,52,0.10)]">
          <div class="flex items-center justify-between mb-2">
            <h3 class="text-sm font-bold text-[#3E3E42] flex items-center gap-2">
              <span class="w-7 h-7 rounded-lg bg-[#F6E3DD] grid place-items-center text-sm">🗂️</span> 已收文件
              <span class="text-[11px] font-normal text-[#A69E97]">
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
            <p class="text-sm text-[#8A857F]">还没有文件，用手机扫左侧二维码传一个过来吧</p>
          </div>

          <ul v-else class="divide-y divide-[#F0E9E1]">
            <li v-for="f in files" :key="f.name" class="group flex items-center gap-3 py-2.5 px-2 -mx-2 rounded-xl hover:bg-[#FBF6F2] transition-colors">
              <div
                class="w-10 h-10 rounded-xl grid place-items-center text-lg shrink-0 border border-white shadow-sm"
                :class="fileTint(f.name)"
              >
                {{ fileIcon(f.name) }}
              </div>
              <div class="min-w-0 flex-1">
                <p class="text-sm font-semibold text-[#3E3E42] truncate">{{ f.name }}</p>
                <p class="text-[11px] text-[#A69E97] mt-0.5">{{ formatSize(f.size) }} · {{ formatTime(f.modified) }}</p>
              </div>
              <button
                class="btn btn-danger opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
                :disabled="busy"
                @click="remove(f.name)"
                title="删除"
              >
                <span class="i-ri-delete-bin-5-line text-base"></span>
              </button>
            </li>
          </ul>
        </div>
      </section>
    </main>

    <!-- 深炭灰底栏：呼应模板深色底部 -->
    <footer
      class="mt-4 bg-[#3E3E42] text-[#B9B4AE] px-6 py-4 flex items-center justify-between text-[11px]"
    >
      <span>文件保存在本地收件目录，不上传任何第三方服务器</span>
      <span class="text-[#8A857F]">关闭 AirBox 即断开连接</span>
    </footer>
  </div>
</template>
