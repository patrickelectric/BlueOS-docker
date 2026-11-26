import { execFileSync } from 'node:child_process'

import { APIRequestContext, expect } from '@playwright/test'

const API = '/version-chooser/v1.0/version'

// The vehicle goes offline and the frontend gives it 2 minutes to come back
export const RESTART_TIMEOUT = 3 * 60_000

export async function currentVersion(request: APIRequestContext): Promise<string> {
  const current = await (await request.get(`${API}/current`)).json()
  return `${current.repository}:${current.tag}`
}

// docker save keeps the repository and tag of the image in its manifest
export function imageName(file: string): string {
  const manifest = JSON.parse(execFileSync('tar', ['-xOf', file, 'manifest.json'], { encoding: 'utf8' }))
  return manifest[0].RepoTags[0]
}

// Puts the vehicle back on a version when a test stops halfway through a switch
export async function setVersion(request: APIRequestContext, name: string): Promise<void> {
  const [repository, tag] = name.split(':')
  // The answer can be lost, the service that sends it is inside the core being restarted
  await request.post(`${API}/current`, { data: { repository, tag } }).catch(() => undefined)
  await expect.poll(() => currentVersion(request).catch(() => ''), { timeout: RESTART_TIMEOUT, intervals: [5_000] })
    .toBe(name)
}
