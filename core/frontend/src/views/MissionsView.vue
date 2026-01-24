<template>
  <v-container fluid class="missions-view pa-4">
    <v-alert
      v-if="error"
      type="error"
      dense
      dismissible
      class="mb-4"
      @input="clearError"
    >
      {{ error }}
    </v-alert>

    <div class="d-flex align-center mb-4">
      <h1 class="text-h5 font-weight-medium">
        Missions
      </h1>
      <v-spacer />
      <v-btn
        v-tooltip="'Refresh missions'"
        icon
        :loading="loading"
        @click="refresh"
      >
        <v-icon>mdi-refresh</v-icon>
      </v-btn>
    </div>

    <v-progress-linear
      v-if="loading && missions.length === 0"
      indeterminate
      color="primary"
      class="mb-4"
    />

    <v-alert
      v-if="!loading && missions.length === 0 && orphanedFiles.length === 0"
      type="info"
      dense
      class="mb-4"
    >
      No missions or files found yet. Arm your vehicle to start recording.
    </v-alert>

    <v-row>
      <v-col
        v-for="mission in missions"
        :key="mission.id"
        cols="12"
        sm="6"
        md="4"
        lg="3"
      >
        <v-card
          class="mission-card d-flex flex-column"
          :class="{ 'processing-card': mission.is_processing, 'incomplete-card': !mission.is_complete }"
        >
          <div
            class="thumbnail-mosaic"
            role="button"
            tabindex="0"
            @click="openMissionDetail(mission)"
            @keydown.enter="openMissionDetail(mission)"
          >
            <template v-if="mission.thumbnails.length > 0">
              <div
                v-for="(thumb, idx) in mission.thumbnails.slice(0, 4)"
                :key="idx"
                class="mosaic-tile"
                :class="getMosaicClass(mission.thumbnails.length, idx)"
              >
                <v-img
                  :src="thumb"
                  height="100%"
                  class="grey lighten-3"
                  cover
                >
                  <template #placeholder>
                    <div class="d-flex align-center justify-center fill-height">
                      <v-progress-circular indeterminate color="primary" size="24" />
                    </div>
                  </template>
                </v-img>
              </div>
            </template>
            <div v-else class="mosaic-placeholder d-flex flex-column align-center justify-center">
              <v-icon v-if="mission.is_processing" size="48" color="primary">
                mdi-loading mdi-spin
              </v-icon>
              <v-icon v-else size="48" color="grey">
                mdi-folder-multiple-outline
              </v-icon>
              <span v-if="mission.is_processing" class="caption grey--text mt-2">
                Processing video...
              </span>
            </div>
          </div>

          <v-card-title class="py-2 text-subtitle-1">
            <v-text-field
              v-if="editingMissionId === mission.id"
              v-model="editingName"
              dense
              hide-details
              single-line
              autofocus
              class="mission-name-input"
              @blur="saveRename(mission)"
              @keyup.enter="saveRename(mission)"
              @keyup.esc="cancelRename"
            />
            <div
              v-else
              class="text-truncate mission-name"
              @dblclick="startRename(mission)"
            >
              {{ mission.name }}
            </div>
          </v-card-title>

          <v-card-subtitle class="py-1">
            <div class="d-flex align-center flex-wrap">
              <v-chip x-small class="mr-1 mb-1" color="grey lighten-2">
                {{ formatDate(mission.date) }}
              </v-chip>
              <v-chip v-if="mission.duration_seconds" x-small class="mr-1 mb-1" color="grey lighten-2">
                {{ formatDuration(mission.duration_seconds) }}
              </v-chip>
            </div>
          </v-card-subtitle>

          <v-card-text class="py-1 flex-grow-1">
            <div class="file-chips d-flex flex-wrap">
              <v-chip
                v-for="fileType in getFileTypes(mission)"
                :key="fileType.type"
                x-small
                :color="getFileTypeColor(fileType.type)"
                class="mr-1 mb-1"
                dark
              >
                <v-icon x-small left>
                  {{ getFileTypeIcon(fileType.type) }}
                </v-icon>
                {{ fileType.count }} {{ fileType.type.toUpperCase() }}
              </v-chip>
              <v-chip
                v-if="!mission.is_complete"
                x-small
                color="warning"
                class="mr-1 mb-1"
              >
                Incomplete
              </v-chip>
            </div>
          </v-card-text>

          <v-divider />

          <v-card-actions class="pa-2">
            <v-btn
              v-tooltip="'View mission details'"
              icon
              small
              color="primary"
              @click="openMissionDetail(mission)"
            >
              <v-icon small>
                mdi-eye
              </v-icon>
            </v-btn>
            <v-btn
              v-if="getFirstVideo(mission)"
              v-tooltip="'Play video'"
              icon
              small
              color="success"
              @click="playVideo(getFirstVideo(mission))"
            >
              <v-icon small>
                mdi-play
              </v-icon>
            </v-btn>
            <v-btn
              v-if="getFirstLog(mission)"
              v-tooltip="'Open log in viewer'"
              icon
              small
              color="info"
              @click="openLog(getFirstLog(mission))"
            >
              <v-icon small>
                mdi-chart-line
              </v-icon>
            </v-btn>
            <v-spacer />
            <v-btn
              v-tooltip="'Delete mission and files'"
              icon
              small
              color="error"
              @click="confirmDelete(mission)"
            >
              <v-icon small>
                mdi-delete
              </v-icon>
            </v-btn>
          </v-card-actions>
        </v-card>
      </v-col>
    </v-row>

    <template v-if="orphanedFiles.length > 0">
      <v-divider class="my-6" />
      <div class="d-flex align-center mb-4">
        <h2 class="text-h6 font-weight-medium">
          Unlinked Files
        </h2>
        <v-chip small class="ml-2" color="warning">
          {{ orphanedFiles.length }}
        </v-chip>
        <v-spacer />
        <v-btn
          v-if="selectedOrphans.length > 0"
          small
          color="primary"
          @click="createMissionFromSelected"
        >
          <v-icon left small>
            mdi-plus
          </v-icon>
          Create Mission ({{ selectedOrphans.length }})
        </v-btn>
      </div>

      <v-data-table
        v-model="selectedOrphans"
        :headers="orphanHeaders"
        :items="orphanedFiles"
        item-key="path"
        show-select
        dense
        class="elevation-1"
      >
        <template #item.type="{ item }">
          <v-chip x-small :color="getFileTypeColor(item.type)" dark>
            {{ item.type.toUpperCase() }}
          </v-chip>
        </template>
        <template #item.size_bytes="{ item }">
          {{ formatSize(item.size_bytes) }}
        </template>
        <template #item.modified="{ item }">
          {{ formatDate(item.modified) }}
        </template>
        <template #item.actions="{ item }">
          <v-btn
            v-tooltip="'Download'"
            icon
            x-small
            :href="item.download_url"
            target="_blank"
          >
            <v-icon x-small>
              mdi-download
            </v-icon>
          </v-btn>
        </template>
      </v-data-table>
    </template>

    <v-dialog v-model="detailDialog" max-width="900" scrollable>
      <v-card v-if="selectedMission">
        <v-card-title class="d-flex align-center">
          <span>{{ selectedMission.name }}</span>
          <v-spacer />
          <v-btn icon @click="detailDialog = false">
            <v-icon>mdi-close</v-icon>
          </v-btn>
        </v-card-title>
        <v-card-subtitle>
          {{ formatDate(selectedMission.date) }}
          <span v-if="selectedMission.duration_seconds">
            · {{ formatDuration(selectedMission.duration_seconds) }}
          </span>
        </v-card-subtitle>
        <v-divider />
        <v-card-text class="pa-4">
          <div v-for="(group, type) in groupFilesByType(selectedMission.files)" :key="type" class="mb-4">
            <h3 class="text-subtitle-1 font-weight-medium mb-2">
              <v-icon small :color="getFileTypeColor(type)" class="mr-1">
                {{ getFileTypeIcon(type) }}
              </v-icon>
              {{ type.toUpperCase() }} Files
            </h3>
            <v-list dense class="pa-0">
              <v-list-item
                v-for="file in group"
                :key="file.path"
                class="px-2"
              >
                <v-list-item-content>
                  <v-list-item-title class="text-body-2">
                    {{ file.name }}
                  </v-list-item-title>
                  <v-list-item-subtitle class="text-caption">
                    {{ formatSize(file.size_bytes) }} · {{ formatDate(file.modified) }}
                  </v-list-item-subtitle>
                </v-list-item-content>
                <v-list-item-action class="flex-row">
                  <v-btn
                    v-if="file.stream_url"
                    v-tooltip="'Play'"
                    icon
                    small
                    color="success"
                    @click="playVideo(file)"
                  >
                    <v-icon small>
                      mdi-play
                    </v-icon>
                  </v-btn>
                  <v-btn
                    v-if="file.type === 'bin' || file.type === 'tlog'"
                    v-tooltip="'Open in Log Viewer'"
                    icon
                    small
                    color="info"
                    @click="openLog(file)"
                  >
                    <v-icon small>
                      mdi-chart-line
                    </v-icon>
                  </v-btn>
                  <v-btn
                    v-tooltip="'Download'"
                    icon
                    small
                    :href="file.download_url"
                    target="_blank"
                  >
                    <v-icon small>
                      mdi-download
                    </v-icon>
                  </v-btn>
                </v-list-item-action>
              </v-list-item>
            </v-list>
          </div>
        </v-card-text>
      </v-card>
    </v-dialog>

    <v-dialog v-model="playerDialog" max-width="1080">
      <v-card class="player-card">
        <v-btn
          icon
          small
          class="dialog-close"
          color="primary"
          @click="closePlayer"
        >
          <v-icon>mdi-close</v-icon>
        </v-btn>
        <v-card-title class="headline">
          {{ activeVideo?.name }}
        </v-card-title>
        <v-card-text>
          <div class="player-wrapper">
            <video
              v-if="activeVideo"
              ref="player"
              controls
              autoplay
              class="player"
              :src="activeVideo.stream_url"
            >
              <track
                kind="captions"
                srclang="en"
                label="Captions not available"
                :src="emptyCaptions"
                default
              />
            </video>
          </div>
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn
            icon
            color="primary"
            :href="activeVideo?.download_url"
            :download="activeVideo?.name"
          >
            <v-icon>mdi-download</v-icon>
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-dialog v-model="deleteDialog" max-width="400">
      <v-card>
        <v-card-title>Delete Mission?</v-card-title>
        <v-card-text>
          This will permanently delete the mission "{{ missionToDelete?.name }}" and all its associated files.
          This action cannot be undone.
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn text @click="deleteDialog = false">
            Cancel
          </v-btn>
          <v-btn color="error" @click="deleteMission">
            Delete
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </v-container>
</template>

<script lang="ts">
import Vue from 'vue'

import { OneMoreTime } from '@/one-more-time'
import missions_store from '@/store/missions'
import { Mission, MissionFile } from '@/types/missions'
import { prettifySize } from '@/utils/helper_functions'

export default Vue.extend({
  name: 'MissionsView',
  data() {
    return {
      detailDialog: false,
      playerDialog: false,
      deleteDialog: false,
      selectedMission: null as Mission | null,
      activeVideo: null as MissionFile | null,
      missionToDelete: null as Mission | null,
      selectedOrphans: [] as MissionFile[],
      editingMissionId: null as string | null,
      editingName: '',
      emptyCaptions: 'data:text/vtt,WEBVTT',
      statusPoller: null as OneMoreTime | null,
      orphanHeaders: [
        { text: 'Name', value: 'name', sortable: true },
        { text: 'Type', value: 'type', sortable: true },
        { text: 'Size', value: 'size_bytes', sortable: true },
        { text: 'Modified', value: 'modified', sortable: true },
        {
          text: 'Actions', value: 'actions', sortable: false, align: 'end',
        },
      ],
    }
  },
  computed: {
    missions(): Mission[] {
      return missions_store.missions
    },
    orphanedFiles(): MissionFile[] {
      return missions_store.orphaned_files
    },
    loading(): boolean {
      return missions_store.loading
    },
    error(): string | null {
      return missions_store.error
    },
  },
  mounted() {
    this.refresh()
    this.statusPoller = new OneMoreTime(
      { delay: 10000, disposeWith: this },
      async () => {
        await missions_store.fetchMissions()
      },
    )
  },
  methods: {
    async refresh(): Promise<void> {
      await missions_store.fetchMissions()
    },
    clearError(): void {
      missions_store.setError(null)
    },
    formatDate(timestamp: number): string {
      const date = new Date(timestamp * 1000)
      return date.toLocaleString()
    },
    formatDuration(seconds: number): string {
      const hrs = Math.floor(seconds / 3600)
      const mins = Math.floor(seconds % 3600 / 60)
      const secs = Math.floor(seconds % 60)
      if (hrs > 0) {
        return `${hrs}h ${mins}m`
      }
      if (mins > 0) {
        return `${mins}m ${secs}s`
      }
      return `${secs}s`
    },
    formatSize(bytes: number): string {
      return prettifySize(bytes / 1024)
    },
    getFileTypes(mission: Mission): { type: string; count: number }[] {
      const counts: Record<string, number> = {}
      mission.files.forEach((f) => {
        counts[f.type] = (counts[f.type] || 0) + 1
      })
      return Object.entries(counts).map(([type, count]) => ({ type, count }))
    },
    getFileTypeColor(type: string): string {
      const colors: Record<string, string> = {
        mp4: 'purple',
        mcap: 'teal',
        bin: 'blue',
        tlog: 'indigo',
      }
      return colors[type] || 'grey'
    },
    getFileTypeIcon(type: string): string {
      const icons: Record<string, string> = {
        mp4: 'mdi-video',
        mcap: 'mdi-database',
        bin: 'mdi-file-document',
        tlog: 'mdi-file-chart',
      }
      return icons[type] || 'mdi-file'
    },
    getMosaicClass(total: number, idx: number): string {
      if (total === 1) return 'mosaic-full'
      if (total === 2) return 'mosaic-half'
      if (total === 3) {
        return idx === 0 ? 'mosaic-half' : 'mosaic-quarter'
      }
      return 'mosaic-quarter'
    },
    getFirstVideo(mission: Mission): MissionFile | undefined {
      return mission.files.find((f) => f.type === 'mp4')
    },
    getFirstLog(mission: Mission): MissionFile | undefined {
      return mission.files.find((f) => f.type === 'bin' || f.type === 'tlog')
    },
    groupFilesByType(files: MissionFile[]): Record<string, MissionFile[]> {
      const groups: Record<string, MissionFile[]> = {}
      files.forEach((f) => {
        if (!groups[f.type]) groups[f.type] = []
        groups[f.type].push(f)
      })
      return groups
    },
    openMissionDetail(mission: Mission): void {
      this.selectedMission = mission
      this.detailDialog = true
    },
    playVideo(file: MissionFile | undefined): void {
      if (!file) return
      this.activeVideo = file
      this.playerDialog = true
    },
    closePlayer(): void {
      const player = this.$refs.player as HTMLVideoElement | undefined
      if (player) {
        player.pause()
        player.currentTime = 0
      }
      this.playerDialog = false
    },
    async openLog(file: MissionFile | undefined): Promise<void> {
      if (!file) return
      const logUrl = encodeURIComponent(file.download_url)
      window.open(`/logviewer/#/?file=${logUrl}`)
    },
    confirmDelete(mission: Mission): void {
      this.missionToDelete = mission
      this.deleteDialog = true
    },
    async deleteMission(): Promise<void> {
      if (!this.missionToDelete) return
      await missions_store.deleteMissionWithFiles(this.missionToDelete.id)
      this.deleteDialog = false
      this.missionToDelete = null
    },
    startRename(mission: Mission): void {
      if (mission.id.startsWith('auto-')) return
      this.editingMissionId = mission.id
      this.editingName = mission.name
    },
    async saveRename(mission: Mission): Promise<void> {
      if (this.editingName && this.editingName !== mission.name) {
        await missions_store.renameMission({ missionId: mission.id, name: this.editingName })
      }
      this.cancelRename()
    },
    cancelRename(): void {
      this.editingMissionId = null
      this.editingName = ''
    },
    async createMissionFromSelected(): Promise<void> {
      if (this.selectedOrphans.length === 0) return
      const filePaths = this.selectedOrphans.map((f) => f.path)
      await missions_store.createMission({ file_paths: filePaths })
      this.selectedOrphans = []
    },
  },
})
</script>

<style scoped>
.missions-view {
  min-height: 100%;
}

.mission-card {
  height: 100%;
  transition: transform 0.2s, box-shadow 0.2s;
}

.mission-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.processing-card {
  opacity: 0.85;
}

.incomplete-card {
  border-left: 3px solid #fb8c00;
}

.thumbnail-mosaic {
  height: 160px;
  display: flex;
  flex-wrap: wrap;
  cursor: pointer;
  overflow: hidden;
}

.mosaic-tile {
  overflow: hidden;
}

.mosaic-full {
  width: 100%;
  height: 100%;
}

.mosaic-half {
  width: 50%;
  height: 100%;
}

.mosaic-quarter {
  width: 50%;
  height: 50%;
}

.mosaic-placeholder {
  width: 100%;
  height: 100%;
  background: #eceff1;
}

.mission-name {
  cursor: pointer;
  max-width: 100%;
}

.mission-name-input {
  max-width: 200px;
}

.file-chips {
  min-height: 24px;
}

.player-card {
  position: relative;
}

.dialog-close {
  position: absolute;
  top: 8px;
  right: 8px;
  z-index: 1;
}

.player-wrapper {
  position: relative;
  width: 100%;
  padding-top: 56.25%;
  background: #111827;
}

.player {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
}
</style>
