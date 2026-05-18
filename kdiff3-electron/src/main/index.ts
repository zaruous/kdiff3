import { app, BrowserWindow, ipcMain, dialog, Menu, WebContents } from 'electron'
import { join, dirname } from 'path'
import { readFileSync, statSync, writeFileSync, cpSync, mkdirSync } from 'fs'
import { promises as fsp, Dirent } from 'fs'

function createWindow(): BrowserWindow {
  const win = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 800,
    minHeight: 600,
    backgroundColor: '#1e1e1e',
    titleBarStyle: 'hiddenInset',
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      sandbox: false
    },
    title: 'KDiff3'
  })

  if (process.env['ELECTRON_RENDERER_URL']) {
    win.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    win.loadFile(join(__dirname, '../renderer/index.html'))
  }

  return win
}

// ── IPC: dialogs ──────────────────────────────────────────────────────────────

ipcMain.handle('open-file-dialog', async (_, opts: { title?: string }) => {
  const result = await dialog.showOpenDialog({ title: opts?.title ?? 'Open File', properties: ['openFile'] })
  return result.canceled ? null : result.filePaths[0]
})

ipcMain.handle('open-dir-dialog', async (_, opts: { title?: string }) => {
  const result = await dialog.showOpenDialog({ title: opts?.title ?? 'Open Directory', properties: ['openDirectory'] })
  return result.canceled ? null : result.filePaths[0]
})

ipcMain.handle('save-file-dialog', async () => {
  const result = await dialog.showSaveDialog({ title: 'Save Output' })
  return result.canceled ? null : result.filePath
})

// ── IPC: file read / write ────────────────────────────────────────────────────

ipcMain.handle('read-file', (_evt, path: string) => {
  try {
    const buf = readFileSync(path)
    let text: string
    if (buf[0] === 0xef && buf[1] === 0xbb && buf[2] === 0xbf) {
      text = buf.slice(3).toString('utf8')
    } else if (buf[0] === 0xff && buf[1] === 0xfe) {
      text = buf.slice(2).toString('utf16le')
    } else if (buf[0] === 0xfe && buf[1] === 0xff) {
      const swapped = Buffer.alloc(buf.length - 2)
      for (let i = 0; i < swapped.length; i += 2) {
        swapped[i] = buf[i + 3]; swapped[i + 1] = buf[i + 2]
      }
      text = swapped.toString('utf16le')
    } else {
      text = buf.toString('utf8')
    }
    return { ok: true, content: text }
  } catch (e) { return { ok: false, error: String(e) } }
})

ipcMain.handle('write-file', (_evt, path: string, content: string) => {
  try { writeFileSync(path, content, 'utf8'); return { ok: true } }
  catch (e) { return { ok: false, error: String(e) } }
})

ipcMain.handle('copy-path', (_evt, src: string, dst: string) => {
  try {
    mkdirSync(dirname(dst), { recursive: true })
    cpSync(src, dst, { recursive: statSync(src).isDirectory() })
    return { ok: true }
  } catch (e) { return { ok: false, error: String(e) } }
})

// ── IPC: directory scan (async BFS, parallel stat, streaming progress) ────────

type ScanEntry = { path: string; isDir: boolean; size: number; mtime: number }
type ScanResult = { ok: true; entries: ScanEntry[] } | { ok: false; error?: string; cancelled?: true }

// Active scans keyed by scanId — set cancelled=true to abort
const activeScanners = new Map<string, { cancelled: boolean }>()

// How many directories to process concurrently within a single scan
const DIR_CONCURRENCY = 16
// Minimum ms between progress events (avoids flooding IPC)
const PROGRESS_INTERVAL_MS = 120

function safeSend(sender: WebContents, channel: string, payload: unknown) {
  if (!sender.isDestroyed()) sender.send(channel, payload)
}

ipcMain.handle('scan-dir-start', async (evt, scanId: string, dirPath: string): Promise<ScanResult> => {
  const ctrl = { cancelled: false }
  activeScanners.set(scanId, ctrl)

  const entries: ScanEntry[] = []
  // Pending directories still to visit (BFS queue)
  const pending: string[] = [dirPath]
  const baseLen = dirPath.length + 1  // length of base path prefix to strip
  let lastReport = 0

  try {
    while (pending.length > 0) {
      if (ctrl.cancelled) {
        activeScanners.delete(scanId)
        return { ok: false, cancelled: true }
      }

      // Take up to DIR_CONCURRENCY directories from the queue
      const batch = pending.splice(0, DIR_CONCURRENCY)

      // Process each directory in the batch concurrently
      await Promise.all(batch.map(async (dir) => {
        let items: Dirent[]
        try {
          // withFileTypes avoids an extra stat() per entry to detect dirs
          items = await fsp.readdir(dir, { withFileTypes: true })
        } catch {
          return  // permission denied etc. — skip silently
        }

        // Stat all files in this directory in parallel
        await Promise.all(items.map(async (item) => {
          if (ctrl.cancelled) return
          const full = `${dir}/${item.name}`
          const rel  = full.slice(baseLen)

          if (item.isDirectory()) {
            entries.push({ path: rel, isDir: true, size: 0, mtime: 0 })
            pending.push(full)
          } else if (item.isFile()) {
            try {
              const st = await fsp.stat(full)
              entries.push({ path: rel, isDir: false, size: st.size, mtime: st.mtimeMs })
            } catch {
              // unreadable file — skip
            }
          }
          // symlinks, sockets, etc. are intentionally skipped
        }))
      }))

      // Throttled progress event
      const now = Date.now()
      if (now - lastReport >= PROGRESS_INTERVAL_MS) {
        lastReport = now
        safeSend(evt.sender, 'scan-progress', { scanId, count: entries.length })
      }
    }

    activeScanners.delete(scanId)
    safeSend(evt.sender, 'scan-progress', { scanId, count: entries.length, done: true })
    return { ok: true, entries }
  } catch (e) {
    activeScanners.delete(scanId)
    return { ok: false, error: String(e) }
  }
})

// Renderer sends this to abort an in-progress scan
ipcMain.on('scan-dir-cancel', (_evt, scanId: string) => {
  const ctrl = activeScanners.get(scanId)
  if (ctrl) ctrl.cancelled = true
  activeScanners.delete(scanId)
})

// ── App lifecycle ─────────────────────────────────────────────────────────────

app.whenReady().then(() => {
  Menu.setApplicationMenu(Menu.buildFromTemplate([
    { label: 'File', submenu: [{ label: 'Quit', accelerator: 'CmdOrCtrl+Q', click: () => app.quit() }] }
  ]))
  createWindow()
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})
