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
  const opts = { width: 240, margin: 1, color: { dark: '#020617ff', light: '#ffffffff' } }
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

onMounted(init)
onUnmounted(() => {
  if (timer) window.clearInterval(timer)
})
</script>

<template>
  <div class="min-h-screen bg-[#0b1220] text-slate-200">
    <header class="border-b border-[#1a2740] bg-[#0d1626]/80 px-6 py-4 flex items-center gap-3">
      <div
        class="w-9 h-9 rounded-lg bg-gradient-to-br from-cyan-400 to-indigo-500 grid place-items-center text-slate-950 font-extrabold text-sm shadow-lg shadow-cyan-500/20"
      >
        AB
      </div>
      <div>
        <h1 class="text-base font-bold tracking-wide">AirBox <span class="text-cyan-300">·</span> 局域网快传</h1>
        <p class="text-xs text-slate-500 mt-0.5">手机扫码即连 · 双向互传 · 文件不出局域网</p>
      </div>
      <div class="ml-auto flex items-center gap-2 text-xs">
        <span class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-emerald-500/10 text-emerald-300 border border-emerald-500/20">
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
          {{ statusText }}
        </span>
      </div>
    </header>

    <main class="p-6 grid grid-cols-[minmax(300px,380px)_1fr] gap-6">
      <!-- 左列：二维码 + 链接 -->
      <section class="space-y-6">
        <div class="card p-6 flex flex-col items-center">
          <h2 class="text-sm font-semibold text-slate-300 self-start mb-4 flex items-center gap-2">
            <span class="text-base">📱</span> 手机扫码传文件
          </h2>
          <div
            class="w-56 h-56 rounded-2xl bg-white p-3 shadow-[0_0_40px_rgba(34,211,238,0.15)] flex items-center justify-center"
          >
            <img v-if="qrDataUrl" :src="qrDataUrl" alt="扫码连接" class="w-full h-full" />
            <div v-else class="text-slate-400 text-sm">二维码生成中…</div>
          </div>
          <p class="mt-4 text-xs text-slate-500 text-center leading-5">
            手机连上<b class="text-slate-300">同一个 Wi-Fi</b>，用相机 / 浏览器
            <br />扫码即可互传文件，无需安装 App
          </p>
        </div>

        <div class="card p-5">
          <h3 class="text-sm font-semibold text-slate-300 mb-3">连接地址</h3>
          <div class="flex items-center gap-2">
            <code class="flex-1 text-xs text-cyan-300 bg-[#0a0f1d] border border-[#1a2740] rounded-lg px-3 py-2 truncate">
              {{ info?.url || '…' }}
            </code>
            <button class="btn btn-primary" :disabled="!info" @click="copyUrl">
              {{ copied ? '✓ 已复制' : '复制' }}
            </button>
          </div>
          <p class="mt-3 text-[11px] text-slate-600 leading-4">
            链接含随机令牌，仅本次会话有效 · 扫码 / 知道链接的人才能访问，其他人无法连入
          </p>
        </div>
      </section>

      <!-- 右列：收件目录 + 文件列表 -->
      <section class="space-y-6 min-w-0">
        <div class="card p-5">
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-sm font-semibold text-slate-300 flex items-center gap-2">
              <span>📂</span> 收件目录
            </h3>
            <div class="flex gap-2">
              <button class="btn btn-outline" @click="chooseDir" :disabled="picking">
                {{ picking ? '选择中…' : '选择目录' }}
              </button>
              <button class="btn btn-outline" @click="reveal" :disabled="!info">打开目录</button>
            </div>
          </div>
          <p class="text-xs text-slate-400 bg-[#0a0f1d] border border-[#1a2740] rounded-lg px-3 py-2 truncate">
            {{ info?.dir || '加载中…' }}
          </p>
        </div>

        <div class="card p-5">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-sm font-semibold text-slate-300 flex items-center gap-2">
              <span>🗂️</span> 已收文件
              <span class="text-[11px] font-normal text-slate-500">
                {{ files.length ? `${files.length} 个 · 共 ${formatSize(totalSize)}` : '' }}
              </span>
            </h3>
            <button class="btn btn-ghost text-xs" @click="refresh">⟳ 刷新</button>
          </div>

          <div v-if="files.length === 0" class="py-14 text-center">
            <div class="text-4xl mb-3">📭</div>
            <p class="text-sm text-slate-500">还没有文件，用手机扫左侧二维码传一个过来吧</p>
          </div>

          <ul v-else class="divide-y divide-[#1a2740]">
            <li v-for="f in files" :key="f.name" class="flex items-center gap-3 py-3">
              <div class="w-9 h-9 rounded-lg bg-[#16233f] grid place-items-center text-lg shrink-0">📦</div>
              <div class="min-w-0 flex-1">
                <p class="text-sm font-medium truncate">{{ f.name }}</p>
                <p class="text-[11px] text-slate-500 mt-0.5">{{ formatSize(f.size) }} · {{ formatTime(f.modified) }}</p>
              </div>
              <button class="btn btn-ghost text-xs text-rose-300 hover:text-rose-200" :disabled="busy" @click="remove(f.name)">
                删除
              </button>
            </li>
          </ul>
        </div>
      </section>
    </main>

    <footer class="px-6 pb-5 text-[11px] text-slate-700">
      文件保存在本地收件目录，不上传任何第三方服务器 · 关闭 AirBox 即断开连接
    </footer>
  </div>
</template>
