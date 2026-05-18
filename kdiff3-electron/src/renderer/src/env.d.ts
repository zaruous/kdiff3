/// <reference types="vite/client" />

import type { api } from '../../preload/index'

declare global {
  interface Window {
    api: typeof api
  }
}
