import axios from 'axios'
import {
  Action, getModule, Module, Mutation, VuexModule,
} from 'vuex-module-decorators'

import store from '@/store'
import {
  ModelEntry, ModelsResponse, ThemePalette, ThemeStatus,
} from '@/types/customization'
import back_axios, { isBackendOffline } from '@/utils/api'

const API_URL = '/customization/v1.0'
const THEME_CSS_URL = '/userdata/styles/theme_style.css'

// Treat these as "service unreachable" — likely the customization service isn't
// running yet (older BlueOS image, still booting, restarting, etc.). The UI
// should silently fall back to "no overrides" instead of showing an error.
const UNREACHABLE_STATUSES = new Set([404, 502, 503, 504])

function isServiceUnreachable(error: any): boolean {
  if (isBackendOffline(error)) return true
  const status = error?.response?.status
  return typeof status === 'number' && UNREACHABLE_STATUSES.has(status)
}

function reloadThemeStylesheet(): void {
  const existing = document.querySelector<HTMLLinkElement>('link[href^="/userdata/styles/theme_style.css"]')
  const buster = `${THEME_CSS_URL}?t=${Date.now()}`
  if (existing) {
    existing.href = buster
    return
  }
  const link = document.createElement('link')
  link.rel = 'stylesheet'
  link.href = buster
  document.head.appendChild(link)
}

@Module({ dynamic: true, store, name: 'customization' })
class CustomizationStore extends VuexModule {
  theme: ThemeStatus = { active: false }

  models: ModelEntry[] = []

  loading_theme = false

  loading_models = false

  uploading = false

  error: string | null = null

  @Mutation
  setTheme(value: ThemeStatus): void {
    this.theme = value
  }

  @Mutation
  setModels(value: ModelEntry[]): void {
    this.models = value
  }

  @Mutation
  setLoadingTheme(value: boolean): void {
    this.loading_theme = value
  }

  @Mutation
  setLoadingModels(value: boolean): void {
    this.loading_models = value
  }

  @Mutation
  setUploading(value: boolean): void {
    this.uploading = value
  }

  @Mutation
  setError(value: string | null): void {
    this.error = value
  }

  @Action
  async fetchTheme(): Promise<void> {
    this.setLoadingTheme(true)
    this.setError(null)
    await back_axios({
      method: 'get',
      url: `${API_URL}/theme`,
      timeout: 5000,
    })
      .then((response) => {
        this.setTheme(response.data as ThemeStatus)
      })
      .catch((error) => {
        if (isServiceUnreachable(error)) {
          this.setTheme({ active: false })
          return
        }
        this.setError(`Failed to fetch theme: ${error.message}`)
      })
      .finally(() => this.setLoadingTheme(false))
  }

  @Action
  async applyTheme(primary_color: string): Promise<void> {
    this.setLoadingTheme(true)
    this.setError(null)
    await back_axios({
      method: 'post',
      url: `${API_URL}/theme`,
      data: { primary_color },
      timeout: 5000,
    })
      .then((response) => {
        this.setTheme(response.data as ThemeStatus)
        reloadThemeStylesheet()
      })
      .catch((error) => {
        if (isServiceUnreachable(error)) {
          this.setError('Customization service is unavailable. Please update BlueOS or wait for the service to start.')
          return
        }
        const detail = error?.response?.data?.detail ?? error.message
        this.setError(`Failed to apply theme: ${detail}`)
      })
      .finally(() => this.setLoadingTheme(false))
  }

  @Action
  async resetTheme(): Promise<void> {
    this.setLoadingTheme(true)
    this.setError(null)
    await back_axios({
      method: 'delete',
      url: `${API_URL}/theme`,
      timeout: 5000,
    })
      .then((response) => {
        this.setTheme(response.data as ThemeStatus)
        reloadThemeStylesheet()
      })
      .catch((error) => {
        if (isServiceUnreachable(error)) {
          this.setTheme({ active: false })
          return
        }
        this.setError(`Failed to reset theme: ${error.message}`)
      })
      .finally(() => this.setLoadingTheme(false))
  }

  @Action
  async previewPalette(primary_color: string): Promise<ThemePalette | null> {
    try {
      const response = await back_axios({
        method: 'get',
        url: `${API_URL}/theme/palette`,
        params: { primary_color },
        timeout: 5000,
      })
      return response.data as ThemePalette
    } catch (error) {
      return null
    }
  }

  @Action
  async fetchModels(): Promise<void> {
    this.setLoadingModels(true)
    this.setError(null)
    await back_axios({
      method: 'get',
      url: `${API_URL}/models`,
      timeout: 10000,
    })
      .then((response) => {
        const data = response.data as ModelsResponse
        this.setModels(data.models ?? [])
      })
      .catch((error) => {
        if (isServiceUnreachable(error)) {
          this.setModels([])
          return
        }
        this.setError(`Failed to fetch models: ${error.message}`)
      })
      .finally(() => this.setLoadingModels(false))
  }

  @Action
  async uploadModel(payload: { file: File; scope: 'global' | 'vehicle'; vehicle?: string; frame?: string }): Promise<void> {
    this.setUploading(true)
    this.setError(null)
    const form = new FormData()
    form.append('file', payload.file)
    try {
      await axios.post(`${API_URL}/models`, form, {
        params: {
          scope: payload.scope,
          vehicle: payload.vehicle,
          frame: payload.frame,
        },
        headers: { 'Content-Type': 'multipart/form-data' },
        timeout: 60000,
      })
      await this.fetchModels()
    } catch (error: any) {
      const detail = error?.response?.data?.detail ?? error.message
      this.setError(`Failed to upload model: ${detail}`)
    } finally {
      this.setUploading(false)
    }
  }

  @Action
  async deleteModel(path: string): Promise<void> {
    this.setError(null)
    await back_axios({
      method: 'delete',
      url: `${API_URL}/models`,
      params: { path },
      timeout: 10000,
    })
      .then(() => this.fetchModels())
      .catch((error) => {
        if (isServiceUnreachable(error)) {
          this.setError('Customization service is unavailable.')
          return
        }
        const detail = error?.response?.data?.detail ?? error.message
        this.setError(`Failed to delete model: ${detail}`)
      })
  }
}

export { CustomizationStore }
const customization = getModule(CustomizationStore)
export default customization
