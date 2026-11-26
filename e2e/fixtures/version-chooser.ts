import { APIRequestContext } from '@playwright/test'

const API = '/version-chooser/v1.0/version'

export async function currentVersion(request: APIRequestContext): Promise<string> {
  const current = await (await request.get(`${API}/current`)).json()
  return `${current.repository}:${current.tag}`
}
