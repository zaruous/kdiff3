import { contextBridge, ipcRenderer } from 'electron'

export const api = {
  openFileDialog: (opts?: { title?: string }) =>
    ipcRenderer.invoke('open-file-dialog', opts) as Promise<string | null>,
  openDirDialog: (opts?: { title?: string }) =>
    ipcRenderer.invoke('open-dir-dialog', opts) as Promise<string | null>,
  saveFileDialog: () =>
    ipcRenderer.invoke('save-file-dialog') as Promise<string | null>,
  readFile: (path: string) =>
    ipcRenderer.invoke('read-file', path) as Promise<{ ok: true; content: string } | { ok: false; error: string }>,
  writeFile: (path: string, content: string) =>
    ipcRenderer.invoke('write-file', path, content) as Promise<{ ok: true } | { ok: false; error: string }>,
  copyPath: (src: string, dst: string) =>
    ipcRenderer.invoke('copy-path', src, dst) as Promise<{ ok: true } | { ok: false; error: string }>,
  scanDir: (dirPath: string) =>
    ipcRenderer.invoke('scan-dir', dirPath) as Promise<
      { ok: true; entries: { path: string; isDir: boolean; size: number; mtime: number }[] }
      | { ok: false; error: string }
    >
}

contextBridge.exposeInMainWorld('api', api)
