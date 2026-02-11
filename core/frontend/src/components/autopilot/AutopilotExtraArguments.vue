<template>
  <v-card class="mt-5">
    <v-card-text>
      <v-alert
        v-if="has_crash"
        type="error"
        dense
        class="mb-4"
      >
        ArduPilot process exited with code {{ process_status.exit_code }}.
        Check your arguments and restart.
      </v-alert>
      <div
        v-for="(_, index) in entries"
        :key="index"
        class="d-flex align-center mb-2"
      >
        <v-text-field
          v-model="entries[index].name"
          label="Argument"
          placeholder="e.g. --home"
          outlined
          dense
          hide-details
          class="mr-2"
        />
        <v-text-field
          v-model="entries[index].value"
          label="Value"
          placeholder="e.g. -27.563,-48.459,0.0,270.0"
          outlined
          dense
          hide-details
          class="mr-2"
        />
        <v-btn
          icon
          color="error"
          @click="removeEntry(index)"
        >
          <v-icon>mdi-close-circle</v-icon>
        </v-btn>
      </div>
      <v-btn
        text
        color="primary"
        class="mt-2"
        @click="addEntry"
      >
        <v-icon left>
          mdi-plus
        </v-icon>
        Add argument
      </v-btn>
    </v-card-text>
    <v-card-actions>
      <v-btn
        style="margin: auto;"
        color="primary"
        :loading="restarting"
        :disabled="!has_changes"
        @click="saveAndRestart"
      >
        Save and restart
      </v-btn>
    </v-card-actions>
  </v-card>
</template>

<script lang="ts">
import Vue from 'vue'

import * as AutopilotManager from '@/components/autopilot/AutopilotManagerUpdater'
import { fetchExtraArguments, fetchProcessStatus } from '@/components/autopilot/AutopilotManagerUpdater'
import Notifier from '@/libs/notifier'
import { OneMoreTime } from '@/one-more-time'
import autopilot from '@/store/autopilot_manager'
import { ProcessStatus } from '@/types/autopilot'
import { autopilot_service } from '@/types/frontend_services'
import back_axios from '@/utils/api'

const notifier = new Notifier(autopilot_service)

interface ArgumentEntry {
  name: string
  value: string
}

export default Vue.extend({
  name: 'AutopilotExtraArguments',
  data() {
    return {
      entries: [] as ArgumentEntry[],
      fetch_extra_arguments_task: new OneMoreTime({ delay: 10000, disposeWith: this }),
      fetch_process_status_task: new OneMoreTime({ delay: 5000, disposeWith: this }),
      loaded: false,
    }
  },
  computed: {
    restarting(): boolean {
      return autopilot.restarting
    },
    stored_arguments(): Record<string, string> {
      return autopilot.extra_arguments
    },
    process_status(): ProcessStatus | null {
      return autopilot.process_status
    },
    has_crash(): boolean {
      const status = this.process_status
      return status !== null && !status.running
        && status.exit_code !== null && status.exit_code !== 0
    },
    has_changes(): boolean {
      const current = this.entriesToDict()
      const stored = this.stored_arguments
      return JSON.stringify(current) !== JSON.stringify(stored)
    },
  },
  watch: {
    stored_arguments: {
      handler(new_args: Record<string, string>) {
        if (!this.loaded) {
          this.loadEntries(new_args)
          this.loaded = true
        }
      },
      immediate: true,
    },
  },
  mounted() {
    this.fetch_extra_arguments_task.setAction(fetchExtraArguments)
    this.fetch_process_status_task.setAction(fetchProcessStatus)
  },
  methods: {
    loadEntries(args: Record<string, string>): void {
      this.entries = Object.entries(args).map(([name, value]) => ({ name, value }))
    },
    entriesToDict(): Record<string, string> {
      const result: Record<string, string> = {}
      for (const entry of this.entries) {
        const name = entry.name.trim()
        if (name) {
          result[name] = entry.value.trim()
        }
      }
      return result
    },
    addEntry(): void {
      this.entries.push({ name: '', value: '' })
    },
    removeEntry(index: number): void {
      this.entries.splice(index, 1)
    },
    async saveAndRestart(): Promise<void> {
      const args = this.entriesToDict()
      try {
        await back_axios({
          method: 'put',
          url: `${autopilot.API_URL}/extra_arguments`,
          timeout: 10000,
          data: args,
        })
        autopilot.setExtraArguments(args)
        this.loaded = false
      } catch (error) {
        notifier.pushBackError('AUTOPILOT_EXTRA_ARGS_SAVE_FAIL', error)
        return
      }
      await AutopilotManager.restart()
    },
  },
})
</script>
