import { execFileSync } from 'node:child_process'

import { APIRequestContext } from '@playwright/test'

const API = '/version-chooser/v1.0/version'

export async function currentVersion(request: APIRequestContext): Promise<string> {
  const current = await (await request.get(`${API}/current`)).json()
  return `${current.repository}:${current.tag}`
}

// docker save keeps the repository and tag of the image in its manifest
export function imageName(file: string): string {
  const manifest = JSON.parse(execFileSync('tar', ['-xOf', file, 'manifest.json'], { encoding: 'utf8' }))
  return manifest[0].RepoTags[0]
}
