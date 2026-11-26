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

// cable_guy redoes the vehicle network when the core starts, a switch in the middle of it can lose the DHCP address
export async function waitNetwork(request: APIRequestContext): Promise<void> {
  let last = ''
  let stable = 0
  // Settled once the interfaces stay the same for 30 seconds, longer than the DHCP server discovery
  await expect.poll(async () => {
    const response = await request.get('/cable-guy/v1.0/ethernet').catch(() => undefined)
    const interfaces = response?.ok() ? await response.text() : ''
    stable = interfaces && interfaces === last ? stable + 1 : 0
    last = interfaces
    return stable
  }, { message: 'the vehicle network should settle', timeout: RESTART_TIMEOUT, intervals: [5_000] })
    .toBeGreaterThanOrEqual(6)
}

// Docker Hub is not what is tested, the remote list is made up, from the newest to the oldest
export function remoteVersions(repository: string, tags: string[]): object {
  const remote = tags.map((tag, index) => ({
    repository,
    tag,
    sha: `sha256:${index}`,
    last_modified: new Date(Date.UTC(2026, 0, 31 - index)).toISOString(),
  }))
  return { local: [], remote, error: null }
}
