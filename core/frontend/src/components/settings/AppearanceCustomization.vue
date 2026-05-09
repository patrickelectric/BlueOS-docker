<template>
  <div class="appearance-customization-root">
    <div
      class="customization-header d-flex align-center pa-4"
      role="button"
      tabindex="0"
      @click="expanded = !expanded"
      @keydown.enter="expanded = !expanded"
      @keydown.space.prevent="expanded = !expanded"
    >
      <v-avatar color="primary" size="32" class="mr-3">
        <v-icon color="white" size="18">
          mdi-palette
        </v-icon>
      </v-avatar>
      <div>
        <div class="text-subtitle-2 font-weight-medium">
          Customization
        </div>
        <div class="text-caption text--secondary">
          Theme color, company logo, vehicle image and 3D models
        </div>
      </div>
      <v-spacer />
      <v-chip v-if="theme.active" x-small color="primary" class="mr-2" outlined>
        Custom theme
      </v-chip>
      <v-icon>{{ expanded ? 'mdi-chevron-up' : 'mdi-chevron-down' }}</v-icon>
    </div>
    <div v-show="expanded">
      <v-divider />
      <div class="pa-4">
        <div class="customization-section">
          <div class="d-flex align-center mb-3">
            <v-icon left small>
              mdi-format-color-fill
            </v-icon>
            <span class="text-subtitle-2">Theme color</span>
          </div>
          <v-row>
            <v-col cols="12" md="6">
              <v-color-picker
                v-model="picked_color"
                mode="hexa"
                hide-mode-switch
                hide-inputs
                flat
              />
              <v-text-field
                v-model="picked_color"
                label="Primary color"
                prepend-inner-icon="mdi-eyedropper-variant"
                class="mt-2"
                dense
                outlined
              />
            </v-col>
            <v-col cols="12" md="6">
              <div class="text-caption text--secondary mb-2">
                Generated 3-anchor gradient (light → mid → dark)
              </div>
              <div class="gradient-preview rounded-lg" :style="gradient_preview_style" />
              <div class="d-flex justify-space-between mt-2">
                <swatch :color="preview_palette.light" label="Light" />
                <swatch :color="preview_palette.mid" label="Mid (Primary)" />
                <swatch :color="preview_palette.dark" label="Dark" />
              </div>
              <div class="d-flex flex-wrap mt-4" style="gap: 8px;">
                <v-btn
                  color="primary"
                  :loading="customization.loading_theme"
                  :disabled="!normalized_color"
                  @click="applyColor"
                >
                  <v-icon left>
                    mdi-check
                  </v-icon>
                  Apply
                </v-btn>
                <v-btn
                  outlined
                  :disabled="!theme.active || customization.loading_theme"
                  @click="resetColor"
                >
                  <v-icon left>
                    mdi-restore
                  </v-icon>
                  Reset to default
                </v-btn>
              </div>
              <v-alert
                v-if="theme.active && theme.primary_color"
                class="mt-3"
                type="info"
                dense
                text
                icon="mdi-information"
              >
                Currently active: <strong>{{ theme.primary_color }}</strong>. Reload to see it across the app.
              </v-alert>
            </v-col>
          </v-row>
        </div>

        <v-divider class="my-4" />

        <div class="customization-section">
          <div class="d-flex align-center mb-3">
            <v-icon left small>
              mdi-cube-outline
            </v-icon>
            <span class="text-subtitle-2">3D vehicle models</span>
            <v-chip x-small outlined class="ml-2">
              {{ customization.models.length }}
            </v-chip>
          </div>
          <div class="text-caption text--secondary mb-3">
            Custom <code>.glb</code> models stored under
            <code>/userdata/modeloverrides/</code>. An
            <code>ALL.glb</code> file overrides every vehicle; a per-vehicle file
            (<code>{{ vehicle_path_hint }}</code>) overrides one frame only.
          </div>
          <div v-if="customization.loading_models" class="d-flex justify-center py-4">
            <v-progress-circular indeterminate color="primary" />
          </div>
          <v-alert
            v-else-if="customization.models.length === 0"
            text
            type="info"
            dense
            icon="mdi-cube-outline"
          >
            No custom 3D models uploaded yet.
          </v-alert>
          <v-list v-else dense subheader two-line>
            <v-list-item v-for="model in customization.models" :key="model.path">
              <v-list-item-avatar>
                <v-avatar :color="model.scope === 'global' ? 'primary' : 'secondary'" size="36">
                  <v-icon dark small>
                    {{ model.scope === 'global' ? 'mdi-earth' : 'mdi-cube' }}
                  </v-icon>
                </v-avatar>
              </v-list-item-avatar>
              <v-list-item-content>
                <v-list-item-title>{{ model.name }}</v-list-item-title>
                <v-list-item-subtitle>
                  <span v-if="model.scope === 'global'">Global override (all vehicles)</span>
                  <span v-else>{{ model.vehicle }} / {{ model.frame }}</span>
                  <span class="text--secondary"> · {{ formatSize(model.size_bytes) }}</span>
                </v-list-item-subtitle>
              </v-list-item-content>
              <v-list-item-action class="d-flex flex-row align-center" style="gap: 4px;">
                <v-btn v-tooltip="'Download'" icon small :href="model.url" target="_blank">
                  <v-icon small>
                    mdi-download
                  </v-icon>
                </v-btn>
                <v-btn
                  v-tooltip="'Delete'"
                  icon
                  small
                  color="error"
                  @click="confirmDelete(model)"
                >
                  <v-icon small>
                    mdi-trash-can
                  </v-icon>
                </v-btn>
              </v-list-item-action>
            </v-list-item>
          </v-list>
          <div class="text-subtitle-2 mt-3 mb-2">
            Upload a new model
          </div>
          <v-row dense>
            <v-col cols="12" sm="4">
              <v-select
                v-model="upload_scope"
                :items="upload_scope_options"
                label="Scope"
                dense
                outlined
                hide-details
              />
            </v-col>
            <v-col v-if="upload_scope === 'vehicle'" cols="6" sm="4">
              <v-text-field
                v-model="upload_vehicle"
                label="Vehicle folder"
                placeholder="sub"
                dense
                outlined
                hide-details
              />
            </v-col>
            <v-col v-if="upload_scope === 'vehicle'" cols="6" sm="4">
              <v-text-field
                v-model="upload_frame"
                label="Frame name"
                placeholder="BLUEROV2"
                dense
                outlined
                hide-details
              />
            </v-col>
          </v-row>
          <v-file-input
            v-model="upload_file"
            accept=".glb"
            label="Choose a .glb file"
            prepend-icon="mdi-cube-send"
            dense
            outlined
            show-size
            class="mt-3"
          />
          <v-btn
            color="primary"
            :loading="customization.uploading"
            :disabled="!upload_ready"
            @click="uploadModel"
          >
            <v-icon left>
              mdi-upload
            </v-icon>
            Upload
          </v-btn>
        </div>
      </div>
    </div>

    <warning-dialog
      v-model="show_delete_confirm"
      title="Delete this 3D model?"
      :message="delete_confirm_message"
      confirm-label="Delete"
      @confirm="onConfirmDelete"
    />

    <v-snackbar
      v-model="show_error"
      color="error"
      timeout="6000"
      top
    >
      {{ customization.error }}
      <template #action="{ attrs }">
        <v-btn text v-bind="attrs" @click="show_error = false">
          Close
        </v-btn>
      </template>
    </v-snackbar>
  </div>
</template>

<script lang="ts">
import Vue from 'vue'

import WarningDialog from '@/components/common/WarningDialog.vue'
import customization from '@/store/customization'
import { ModelEntry, ThemePalette } from '@/types/customization'
import { prettifySize } from '@/utils/helper_functions'

import Swatch from './ColorSwatch.vue'

const DEFAULT_PRIMARY = '#135DA3'

export default Vue.extend({
  name: 'AppearanceCustomization',
  components: {
    Swatch,
    WarningDialog,
  },
  data() {
    return {
      customization,
      expanded: false,
      picked_color: DEFAULT_PRIMARY,
      preview_palette: { light: DEFAULT_PRIMARY, mid: DEFAULT_PRIMARY, dark: DEFAULT_PRIMARY } as ThemePalette,
      upload_scope: 'global' as 'global' | 'vehicle',
      upload_scope_options: [
        { text: 'Global (overrides every vehicle)', value: 'global' },
        { text: 'Specific vehicle / frame', value: 'vehicle' },
      ],
      upload_vehicle: '',
      upload_frame: '',
      upload_file: null as File | null,
      pending_delete: null as ModelEntry | null,
      show_delete_confirm: false,
      show_error: false,
    }
  },
  computed: {
    theme() {
      return customization.theme
    },
    normalized_color(): string {
      const value = (this.picked_color || '').trim()
      if (!value) return ''
      const stripped = value.startsWith('#') ? value : `#${value}`
      const hex = stripped.replace(/^#/, '').slice(0, 6)
      return /^[0-9a-fA-F]{6}$/.test(hex) ? `#${hex.toUpperCase()}` : ''
    },
    gradient_preview_style(): Record<string, string> {
      const { light, mid, dark } = this.preview_palette
      return {
        background: `linear-gradient(160deg, ${light} 0%, ${mid} 50%, ${dark} 100%)`,
      }
    },
    upload_ready(): boolean {
      if (!this.upload_file) return false
      if (this.upload_scope === 'vehicle' && (!this.upload_vehicle || !this.upload_frame)) return false
      return true
    },
    delete_confirm_message(): string {
      if (!this.pending_delete) return ''
      return `The file "${this.pending_delete.path}" will be permanently removed. This cannot be undone.`
    },
    vehicle_path_hint(): string {
      return '{sub|rover}/{FRAME_NAME}.glb'
    },
  },
  watch: {
    normalized_color(value: string) {
      if (!value) return
      this.refreshPreview(value)
    },
    'customization.error': function onError(value: string | null) {
      if (value) this.show_error = true
    },
  },
  mounted() {
    customization.fetchTheme()
      .then(() => {
        if (customization.theme.primary_color) {
          this.picked_color = customization.theme.primary_color
        }
        this.refreshPreview(this.normalized_color || DEFAULT_PRIMARY)
      })
      .catch(() => {
        this.refreshPreview(DEFAULT_PRIMARY)
      })
    customization.fetchModels().catch(() => undefined)
  },
  methods: {
    formatSize(bytes: number): string {
      return prettifySize(bytes / 1024)
    },
    async refreshPreview(color: string): Promise<void> {
      const palette = await customization.previewPalette(color)
      if (palette) this.preview_palette = palette
    },
    async applyColor(): Promise<void> {
      if (!this.normalized_color) return
      await customization.applyTheme(this.normalized_color)
    },
    async resetColor(): Promise<void> {
      await customization.resetTheme()
      this.picked_color = DEFAULT_PRIMARY
      await this.refreshPreview(DEFAULT_PRIMARY)
    },
    async uploadModel(): Promise<void> {
      if (!this.upload_file) return
      await customization.uploadModel({
        file: this.upload_file,
        scope: this.upload_scope,
        vehicle: this.upload_scope === 'vehicle' ? this.upload_vehicle.trim() : undefined,
        frame: this.upload_scope === 'vehicle' ? this.upload_frame.trim() : undefined,
      })
      if (!customization.error) {
        this.upload_file = null
      }
    },
    confirmDelete(model: ModelEntry): void {
      this.pending_delete = model
      this.show_delete_confirm = true
    },
    async onConfirmDelete(): Promise<void> {
      if (!this.pending_delete) return
      const target = this.pending_delete
      this.show_delete_confirm = false
      this.pending_delete = null
      await customization.deleteModel(target.path)
    },
  },
})
</script>

<style scoped>
.appearance-customization-root {
  border: 1px solid rgba(0, 0, 0, 0.12);
  border-radius: 4px;
  overflow: hidden;
}
.customization-header {
  cursor: pointer;
  user-select: none;
  transition: background-color 0.15s ease-in-out;
}
.customization-header:hover {
  background-color: rgba(0, 0, 0, 0.04);
}
.gradient-preview {
  width: 100%;
  height: 96px;
  border: 1px solid rgba(0, 0, 0, 0.12);
}
</style>
