import { contextBridge, ipcRenderer } from 'electron'

export type ScanProgressEvent = { scanId: string; count: number; done?: boolean }
export type ScanResult =
  | { ok: true; entries: { path: string; isDir: boolean; size: number; mtime: number }[] }
  | { ok: false; error?: string; cancelled?: true }

export const api = {
  // ── Dialogs ────────────────────────────────────────────────────────────────
  openFileDialog: (opts?: { title?: string }) =>
    ipcRenderer.invoke('open-file-dialog', opts) as Promise<string | null>,
  openDirDialog: (opts?: { title?: string }) =>
    ipcRenderer.invoke('open-dir-dialog', opts) as Promise<string | null>,
  saveFileDialog: () =>
    ipcRenderer.invoke('save-file-dialog') as Promise<string | null>,

  // ── File I/O ───────────────────────────────────────────────────────────────
  readFile: (path: string) =>
    ipcRenderer.invoke('read-file', path) as Promise<
      { ok: true; content: string } | { ok: false; error: string }
    >,
  writeFile: (path: string, content: string) =>
    ipcRenderer.invoke('write-file', path, content) as Promise<
      { ok: true } | { ok: false; error: string }
    >,
  copyPath: (src: string, dst: string) =>
    ipcRenderer.invoke('copy-path', src, dst) as Promise<
      { ok: true } | { ok: false; error: string }
    >,

  // ── Directory scan ─────────────────────────────────────────────────────────
  // Start an async scan. Returns when the full scan is complete.
  // Use onScanProgress to receive intermediate progress updates.
  scanDirStart: (scanId: string, dirPath: string) =>
    ipcRenderer.invoke('scan-dir-start', scanId, dirPath) as Promise<ScanResult>,

  // Cancel a running scan (fire-and-forget)
  scanDirCancel: (scanId: string) =>
    ipcRenderer.send('scan-dir-cancel', scanId),

  // Subscribe to progress events. Returns a cleanup function.
  onScanProgress: (cb: (data: ScanProgressEvent) => void): (() => void) => {
    const handler = (_evt: unknown, data: ScanProgressEvent) => cb(data)
    ipcRenderer.on('scan-progress', handler)
    return () => ipcRenderer.off('scan-progress', handler)
  }
}

contextBridge.exposeInMainWorld('api', api)
