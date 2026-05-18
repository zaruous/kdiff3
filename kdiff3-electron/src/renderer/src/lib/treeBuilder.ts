import { DirEntry, EntryStatus } from './dirDiff'

export interface TreeRow {
  path: string        // e.g. "src/utils.ts"
  name: string        // e.g. "utils.ts"
  depth: number
  isDir: boolean
  status: EntryStatus
  hasInA: boolean     // exists on A side
  hasInB: boolean     // exists on B side
  sizeA?: number
  sizeB?: number
  mtimeA?: number
  mtimeB?: number
  childrenChanged: number   // # non-equal children (for dirs)
}

interface TreeNode {
  path: string
  name: string
  isDir: boolean
  status: EntryStatus
  hasInA: boolean
  hasInB: boolean
  sizeA?: number
  sizeB?: number
  mtimeA?: number
  mtimeB?: number
  children: TreeNode[]
}

// Derive directory status by rolling up children
function rollupStatus(node: TreeNode): EntryStatus {
  if (!node.isDir || node.children.length === 0) return node.status
  const childStatuses = node.children.map(c => rollupStatus(c))
  if (childStatuses.every(s => s === 'equal')) return 'equal'
  if (childStatuses.some(s => s === 'conflict')) return 'conflict'
  if (childStatuses.every(s => s === 'onlyA')) return 'onlyA'
  if (childStatuses.every(s => s === 'onlyB')) return 'onlyB'
  return 'modified'
}

// Count non-equal descendants
function countChanged(node: TreeNode): number {
  if (!node.isDir) return node.status === 'equal' ? 0 : 1
  return node.children.reduce((acc, c) => acc + countChanged(c), 0)
}

export function buildTree(entries: DirEntry[]): TreeNode[] {
  const nodeMap = new Map<string, TreeNode>()

  // Create a node for every entry
  for (const e of entries) {
    nodeMap.set(e.path, {
      path: e.path,
      name: e.path.split('/').pop()!,
      isDir: e.isDir,
      status: e.status,
      hasInA: e.status !== 'onlyB',
      hasInB: e.status !== 'onlyA',
      sizeA: e.sizeA,
      sizeB: e.sizeB,
      mtimeA: e.mtimeA,
      mtimeB: e.mtimeB,
      children: []
    })
  }

  // Ensure implicit parent directories exist
  const rootChildren: TreeNode[] = []
  for (const [path] of nodeMap) {
    const parts = path.split('/')
    if (parts.length === 1) {
      rootChildren.push(nodeMap.get(path)!)
    } else {
      const parentPath = parts.slice(0, -1).join('/')
      let parent = nodeMap.get(parentPath)
      if (!parent) {
        // implicit dir not in entries (shouldn't happen with our scanner, but be safe)
        parent = {
          path: parentPath,
          name: parts[parts.length - 2],
          isDir: true,
          status: 'equal',
          hasInA: true,
          hasInB: true,
          children: []
        }
        nodeMap.set(parentPath, parent)
        // Will be attached to its own parent later — add to root for now
        rootChildren.push(parent)
      }
      parent.children.push(nodeMap.get(path)!)
    }
  }

  // Sort: dirs first, then alphabetical
  function sortNode(children: TreeNode[]) {
    children.sort((a, b) =>
      a.isDir !== b.isDir ? (a.isDir ? -1 : 1) : a.name.localeCompare(b.name)
    )
    children.forEach(c => sortNode(c.children))
  }
  sortNode(rootChildren)

  // Roll up directory statuses
  rootChildren.forEach(rollupStatus)

  return rootChildren
}

export function flattenTree(
  nodes: TreeNode[],
  expandedSet: Set<string>,
  filter: string,
  showEqual: boolean
): TreeRow[] {
  const rows: TreeRow[] = []
  const f = filter.toLowerCase()

  // Pre-compute which paths should be visible (for filter + showEqual)
  const visible = new Set<string>()

  function markVisible(node: TreeNode): boolean {
    const nameMatch = !f || node.path.toLowerCase().includes(f)
    const equalOk = showEqual || node.status !== 'equal'

    let childVisible = false
    if (node.isDir) {
      for (const c of node.children) {
        if (markVisible(c)) childVisible = true
      }
    }

    if ((nameMatch && equalOk) || childVisible) {
      visible.add(node.path)
      return true
    }
    return false
  }

  nodes.forEach(markVisible)

  function flatten(node: TreeNode, depth: number) {
    if (!visible.has(node.path)) return
    rows.push({
      path: node.path,
      name: node.name,
      depth,
      isDir: node.isDir,
      status: rollupStatus(node),
      hasInA: node.hasInA,
      hasInB: node.hasInB,
      sizeA: node.sizeA,
      sizeB: node.sizeB,
      mtimeA: node.mtimeA,
      mtimeB: node.mtimeB,
      childrenChanged: countChanged(node)
    })
    if (node.isDir && expandedSet.has(node.path)) {
      node.children.forEach(c => flatten(c, depth + 1))
    }
  }

  nodes.forEach(n => flatten(n, 0))
  return rows
}
