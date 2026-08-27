import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

export interface ServerInfo {
  url: string
  lan_ip: string
  port: number
  token: string
  dir: string
}

export interface FileInfo {
  name: string
  size: number
  modified: number
}

let base = ''

export async function getServerInfo(): Promise<ServerInfo> {
  const info = await invoke<ServerInfo>('get_server_info')
  base = `http://127.0.0.1:${info.port}/t/${info.token}`
  return info
}

export function apiBase(): string {
  return base
}

export async function listFiles(): Promise<FileInfo[]> {
  const r = await fetch(`${base}/api/files`, { cache: 'no-store' })
  const d = await r.json()
  return (d.files ?? []) as FileInfo[]
}

export async function deleteFile(name: string): Promise<void> {
  await fetch(`${base}/api/files/${encodeURIComponent(name)}`, { method: 'DELETE' })
}

export async function pickDir(): Promise<string | null> {
  const dir = await open({ directory: true, multiple: false, title: '选择收件目录' })
  return typeof dir === 'string' ? dir : null
}

export async function setDir(dir: string): Promise<void> {
  await invoke('set_dir', { dir })
}

export async function openDir(dir: string): Promise<void> {
  await invoke('open_dir', { dir })
}

/** 桌面拖拽导入：把文件复制进共享目录，返回成功数 */
export async function importFiles(paths: string[]): Promise<number> {
  return await invoke<number>('import_files', { paths })
}

export function formatSize(n: number): string {
  if (n == null) return '-'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let i = 0
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024
    i++
  }
  return (i === 0 ? n : n.toFixed(1)) + ' ' + units[i]
}

export function formatTime(ts: number): string {
  if (!ts) return '-'
  const d = new Date(ts * 1000)
  const p = (x: number) => (x < 10 ? '0' + x : '' + x)
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
}

export interface ClipboardEntry {
  text: string
  updated_at: number
}

export async function getClipboard(): Promise<ClipboardEntry> {
  const r = await fetch(`${base}/api/clipboard`, { cache: 'no-store' })
  return (await r.json()) as ClipboardEntry
}

export async function setClipboard(text: string): Promise<void> {
  await fetch(`${base}/api/clipboard`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ text }),
  })
}

/** 读取系统剪贴板（用于电脑→手机的自动同步） */
export async function readSystemClipboard(): Promise<string | null> {
  return await invoke<string | null>('read_clipboard')
}
