<template>
  <div>
    <v-expansion-panels v-model="open_panel" multiple flat hover>
      <v-expansion-panel>
        <v-expansion-panel-header>
          <div class="d-flex align-center">
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
          </div>
        </v-expansion-panel-header>
        <v-expansion-panel-content>
          <v-expansion-panels accordion flat>
            <!-- Theme color -->
            <v-expansion-panel>
              <v-expansion-panel-header>
                <div class="d-flex align-center">
                  <v-icon left small>
                    mdi-format-color-fill
                  </v-icon>
                  <span class="text-subtitle-2">Theme color</span>
                </div>
              </v-expansion-panel-header>
              <v-expansion-panel-content>
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
              </v-expansion-panel-content>
            </v-expansion-panel>

            <!-- Company logo -->
            <v-expansion-panel>
              <v-expansion-panel-header>
                <div class="d-flex align-center">
                  <v-icon left small>
                    mdi-domain
                  </v-icon>
                  <span class="text-subtitle-2">Company logo</span>
                </div>
              </v-expansion-panel-header>
              <v-expansion-panel-content>
                <div class="d-flex align-center" style="gap: 16px;">
                  <image-picker
                    size="80px"
                    directory="/userdata/images/logo"
                    :default-image="default_logo"
                    :image="logo_image"
                    @image-selected="save_logo"
                  />
                  <div class="flex-grow-1">
                    <div class="text-caption text--secondary">
                      Click the logo to upload a new image, pick one of the previously uploaded files,
                      or remove the current selection. Files live under
                      <code>/userdata/images/logo/</code>.
                    </div>
                    <v-btn
                      v-if="logo_image"
                      class="mt-2"
                      x-small
                      outlined
                      color="error"
                      @click="clear_logo"
                    >
                      <v-icon left x-small>
                        mdi-close
                      </v-icon>
                      Reset to default
                    </v-btn>
                  </div>
                </div>
              </v-expansion-panel-content>
            </v-expansion-panel>

            <!-- Vehicle image -->
            <v-expansion-panel>
              <v-expansion-panel-header>
                <div class="d-flex align-center">
                  <v-icon left small>
                    mdi-image
                  </v-icon>
                  <span class="text-subtitle-2">Vehicle image</span>
                </div>
              </v-expansion-panel-header>
              <v-expansion-panel-content>
                <div class="d-flex align-center" style="gap: 16px;">
                  <image-picker
                    size="80px"
                    directory="/userdata/images/vehicle"
                    :readonly-files="readonly_vehicle_images"
                    :default-image="default_vehicle_image"
                    :image="vehicle_image"
                    @image-selected="save_vehicle_image"
                  />
                  <div class="flex-grow-1">
                    <div class="text-caption text--secondary">
                      The image displayed next to the vehicle name in the top banner.
                      Files live under <code>/userdata/images/vehicle/</code>.
                    </div>
                    <v-btn
                      v-if="vehicle_image"
                      class="mt-2"
                      x-small
                      outlined
                      color="error"
                      @click="clear_vehicle_image"
                    >
                      <v-icon left x-small>
                        mdi-close
                      </v-icon>
                      Reset to default
                    </v-btn>
                  </div>
                </div>
              </v-expansion-panel-content>
            </v-expansion-panel>

            <!-- 3D models -->
            <v-expansion-panel>
              <v-expansion-panel-header>
                <div class="d-flex align-center">
                  <v-icon left small>
                    mdi-cube-outline
                  </v-icon>
                  <span class="text-subtitle-2">3D vehicle models</span>
                  <v-spacer />
                  <v-chip x-small outlined class="mr-2">
                    {{ customization.models.length }}
                  </v-chip>
                </div>
              </v-expansion-panel-header>
              <v-expansion-panel-content>
                <div class="text-caption text--secondary mb-3">
                  Custom <code>.glb</code> models stored under
                  <code>/userdata/modeloverrides/</code>. A
                  <code>ALL.glb</code> file overrides every vehicle; a per-vehicle file
                  (<code>{{ '<sub|rover>/<FRAME_NAME>.glb' }}</code>) overrides one frame only.
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

                <v-divider class="my-3" />

                <div class="text-subtitle-2 mb-2">
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
              </v-expansion-panel-content>
            </v-expansion-panel>
          </v-expansion-panels>
        </v-expansion-panel-content>
      </v-expansion-panel>
    </v-expansion-panels>

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

import default_logo from '@/assets/img/blue-robotics-logo.svg'
import default_vehicle_image from '@/assets/vehicles/images/unknown.svg'
import ImagePicker from '@/components/app/ImagePicker.vue'
import WarningDialog from '@/components/common/WarningDialog.vue'
import bag from '@/store/bag'
import customization from '@/store/customization'
import { ModelEntry, ThemePalette } from '@/types/customization'
import { prettifySize } from '@/utils/helper_functions'

import Swatch from './ColorSwatch.vue'

const DEFAULT_PRIMARY = '#135DA3'

export default Vue.extend({
  name: 'AppearanceCustomization',
  components: {
    ImagePicker,
    Swatch,
    WarningDialog,
  },
  data() {
    return {
      customization,
      open_panel: [0] as number[],
      picked_color: DEFAULT_PRIMARY,
      preview_palette: { light: DEFAULT_PRIMARY, mid: DEFAULT_PRIMARY, dark: DEFAULT_PRIMARY } as ThemePalette,
      logo_image: null as string | null,
      vehicle_image: null as string | null,
      default_logo,
      default_vehicle_image,
      readonly_vehicle_images: [
        '/assets/vehicles/images/bluerov2.png',
        '/assets/vehicles/images/bb120.png',
      ],
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
      // v-color-picker returns #RRGGBBAA in hexa mode, drop the alpha component.
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
  async mounted() {
    await Promise.all([
      customization.fetchTheme(),
      customization.fetchModels(),
      this.loadLogo(),
      this.loadVehicleImage(),
    ])
    if (customization.theme.primary_color) {
      this.picked_color = customization.theme.primary_color
    }
    this.refreshPreview(this.normalized_color || DEFAULT_PRIMARY)
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
    async loadLogo(): Promise<void> {
      const data = await bag.getData('vehicle.logo_image_path')
      let url = data?.url as string | undefined ?? null
      if (url && !url.startsWith('/')) url = `/${url}`
      this.logo_image = url
    },
    async loadVehicleImage(): Promise<void> {
      const data = await bag.getData('vehicle.image_path')
      let url = data?.url as string | undefined ?? null
      if (url && !url.startsWith('/')) url = `/${url}`
      this.vehicle_image = url
    },
    save_logo(image: string): void {
      this.logo_image = image
      bag.setData('vehicle.logo_image_path', { url: image })
    },
    clear_logo(): void {
      this.logo_image = null
      bag.setData('vehicle.logo_image_path', { url: '' })
    },
    save_vehicle_image(image: string): void {
      this.vehicle_image = image
      bag.setData('vehicle.image_path', { url: image })
    },
    clear_vehicle_image(): void {
      this.vehicle_image = null
      bag.setData('vehicle.image_path', { url: '' })
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
.gradient-preview {
  width: 100%;
  height: 96px;
  border: 1px solid rgba(0, 0, 0, 0.12);
}
</style>
