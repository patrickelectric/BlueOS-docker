export interface MissionFile {
  name: string
  path: string
  size_bytes: number
  modified: number
  type: 'bin' | 'tlog' | 'mcap' | 'mp4'
  download_url: string
  thumbnail_url?: string
  stream_url?: string
}

export interface Mission {
  id: string
  name: string
  date: number
  duration_seconds: number | null
  files: MissionFile[]
  thumbnails: string[]
  is_complete: boolean
  is_processing: boolean
}

export interface MissionsResponse {
  missions: Mission[]
  orphaned_files: MissionFile[]
}

export interface LinkFilesRequest {
  mission_id: string
  file_paths: string[]
}

export interface CreateMissionRequest {
  name?: string
  file_paths: string[]
}
