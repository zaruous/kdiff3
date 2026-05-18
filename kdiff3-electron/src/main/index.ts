import { app, BrowserWindow, ipcMain, dialog, Menu } from 'electron'
import { join } from 'path'
import { readFileSync, statSync, readdirSync, writeFileSync } from 'fs'

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

// ── IPC handlers ──────────────────────────────────────────────────────────────

ipcMain.handle('open-file-dialog', async (_, opts: { title?: string }) => {
  const result = await dialog.showOpenDialog({
    title: opts?.title ?? 'Open File',
    properties: ['openFile']
  })
  return result.canceled ? null : result.filePaths[0]
})

ipcMain.handle('open-dir-dialog', async (_, opts: { title?: string }) => {
  const result = await dialog.showOpenDialog({
    title: opts?.title ?? 'Open Directory',
    properties: ['openDirectory']
  })
  return result.canceled ? null : result.filePaths[0]
})

ipcMain.handle('save-file-dialog', async () => {
  const result = await dialog.showSaveDialog({ title: 'Save Merge Output' })
  return result.canceled ? null : result.filePath
})

ipcMain.handle('read-file', (_evt, path: string) => {
  try {
    const buf = readFileSync(path)
    // BOM detection
    let text: string
    if (buf[0] === 0xef && buf[1] === 0xbb && buf[2] === 0xbf) {
      text = buf.slice(3).toString('utf8')
    } else if (buf[0] === 0xff && buf[1] === 0xfe) {
      text = buf.slice(2).toString('utf16le')
    } else if (buf[0] === 0xfe && buf[1] === 0xff) {
      // UTF-16 BE
      const swapped = Buffer.alloc(buf.length - 2)
      for (let i = 0; i < swapped.length; i += 2) {
        swapped[i] = buf[i + 3]
        swapped[i + 1] = buf[i + 2]
      }
      text = swapped.toString('utf16le')
    } else {
      text = buf.toString('utf8')
    }
    return { ok: true, content: text }
  } catch (e) {
    return { ok: false, error: String(e) }
  }
})

ipcMain.handle('write-file', (_evt, path: string, content: string) => {
  try {
    writeFileSync(path, content, 'utf8')
    return { ok: true }
  } catch (e) {
    return { ok: false, error: String(e) }
  }
})

ipcMain.handle('scan-dir', (_evt, dirPath: string) => {
  try {
    const entries: { path: string; isDir: boolean; size: number; mtime: number }[] = []
    function walk(dir: string, base: string) {
      let items: string[]
      try { items = readdirSync(dir) } catch { return }
      for (const name of items) {
        const full = `${dir}/${name}`
        const rel = base ? `${base}/${name}` : name
        let st: ReturnType<typeof statSync>
        try { st = statSync(full) } catch { continue }
        entries.push({ path: rel, isDir: st.isDirectory(), size: st.size, mtime: st.mtimeMs })
        if (st.isDirectory()) walk(full, rel)
      }
    }
    walk(dirPath, '')
    return { ok: true, entries }
  } catch (e) {
    return { ok: false, error: String(e) }
  }
})

// ── App lifecycle ─────────────────────────────────────────────────────────────

app.whenReady().then(() => {
  Menu.setApplicationMenu(Menu.buildFromTemplate([
    {
      label: 'File',
      submenu: [
        { label: 'Quit', accelerator: 'CmdOrCtrl+Q', click: () => app.quit() }
      ]
    }
  ]))
  createWindow()
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})
