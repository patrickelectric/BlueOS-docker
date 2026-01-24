import {
  Action, getModule, Module, Mutation, VuexModule,
} from 'vuex-module-decorators'

import store from '@/store'
import {
  CreateMissionRequest, LinkFilesRequest, Mission, MissionFile, MissionsResponse,
} from '@/types/missions'
import back_axios, { isBackendOffline } from '@/utils/api'

@Module({ dynamic: true, store, name: 'missions' })
class MissionsStore extends VuexModule {
  API_URL = '/recorder-extractor/v1.0/missions'

  missions: Mission[] = []

  orphaned_files: MissionFile[] = []

  loading = false

  error: string | null = null

  @Mutation
  setLoading(value: boolean): void {
    this.loading = value
  }

  @Mutation
  setMissions(missions: Mission[]): void {
    this.missions = missions
  }

  @Mutation
  setOrphanedFiles(files: MissionFile[]): void {
    this.orphaned_files = files
  }

  @Mutation
  setError(message: string | null): void {
    this.error = message
  }

  @Action
  async fetchMissions(): Promise<void> {
    this.setLoading(true)
    this.setError(null)
    await back_axios({
      method: 'get',
      url: this.API_URL,
      timeout: 30000,
    })
      .then((response) => {
        const data = response.data as MissionsResponse
        this.setMissions(data.missions)
        this.setOrphanedFiles(data.orphaned_files)
      })
      .catch((error) => {
        this.setMissions([])
        this.setOrphanedFiles([])
        if (isBackendOffline(error)) {
          return
        }
        this.setError(`Failed to fetch missions: ${error.message}`)
      })
      .finally(() => {
        this.setLoading(false)
      })
  }

  @Action
  async createMission(request: CreateMissionRequest): Promise<Mission | null> {
    try {
      const response = await back_axios({
        method: 'post',
        url: this.API_URL,
        data: request,
        timeout: 10000,
      })
      await this.fetchMissions()
      return response.data as Mission
    } catch (error: unknown) {
      const err = error as Error
      this.setError(`Failed to create mission: ${err.message}`)
      return null
    }
  }

  @Action
  async linkFilesToMission(request: LinkFilesRequest): Promise<boolean> {
    try {
      await back_axios({
        method: 'post',
        url: `${this.API_URL}/${request.mission_id}/files`,
        data: request,
        timeout: 10000,
      })
      await this.fetchMissions()
      return true
    } catch (error: unknown) {
      const err = error as Error
      this.setError(`Failed to link files: ${err.message}`)
      return false
    }
  }

  @Action
  async renameMission(payload: { missionId: string; name: string }): Promise<boolean> {
    try {
      await back_axios({
        method: 'patch',
        url: `${this.API_URL}/${payload.missionId}`,
        params: { name: payload.name },
        timeout: 10000,
      })
      await this.fetchMissions()
      return true
    } catch (error: unknown) {
      const err = error as Error
      this.setError(`Failed to rename mission: ${err.message}`)
      return false
    }
  }

  @Action
  async deleteMission(missionId: string): Promise<boolean> {
    try {
      await back_axios({
        method: 'delete',
        url: `${this.API_URL}/${missionId}`,
        timeout: 10000,
      })
      await this.fetchMissions()
      return true
    } catch (error: unknown) {
      const err = error as Error
      this.setError(`Failed to delete mission: ${err.message}`)
      return false
    }
  }

  @Action
  async deleteMissionWithFiles(missionId: string): Promise<boolean> {
    try {
      await back_axios({
        method: 'delete',
        url: `${this.API_URL}/${missionId}/all`,
        timeout: 30000,
      })
      await this.fetchMissions()
      return true
    } catch (error: unknown) {
      const err = error as Error
      this.setError(`Failed to delete mission: ${err.message}`)
      return false
    }
  }
}

const missions_store = getModule(MissionsStore)

export { MissionsStore }
export default missions_store
