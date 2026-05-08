export interface ThemePalette {
  light: string
  mid: string
  dark: string
}

export interface ThemeStatus {
  active: boolean
  primary_color?: string | null
  palette?: ThemePalette | null
}

export type ModelScope = 'global' | 'vehicle'

export interface ModelEntry {
  name: string
  path: string
  url: string
  scope: ModelScope
  vehicle?: string | null
  frame?: string | null
  size_bytes: number
}

export interface ModelsResponse {
  models: ModelEntry[]
}
