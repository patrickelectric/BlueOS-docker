import { createWriteStream, existsSync, mkdirSync, renameSync } from 'node:fs'
import { join } from 'node:path'
import { Readable } from 'node:stream'
import { pipeline } from 'node:stream/promises'

import { Page, TestInfo } from '@playwright/test'

import { BACKEND_TIMEOUT } from '@/fixtures/ui'
import { expect, test } from '@/fixtures/vehicle'
import { remoteVersions } from '@/fixtures/version-chooser'

const REPOSITORY = 'bluerobotics/blueos-core'
// Latest stable, its release ships the core image of every architecture
const RELEASE = '1.4.7'
// Docker architecture of the vehicle to the suffix of the release asset
const ASSET_ARCHITECTURE: Record<string, string> = { amd64: 'amd64', arm64: 'arm64-v8', arm: 'arm-v7' }
// The backend gives the load 15 minutes
const LOAD_TIMEOUT = 15 * 60_000
// One more than a page of the remote list, sorted from the newest
const REMOTE_TAGS = [
  'alpha', 'bravo', 'charlie', 'delta', 'echo', 'foxtrot', 'golf', 'hotel', 'india', 'juliett', 'kilo',
]
const REMOTE_VERSIONS = /\/version-chooser\/v1\.0\/version\/available\/(?!local)/

// The image is about 500MB, it is downloaded once and kept in e2e/.cache
async function releaseImage(page: Page, testInfo: TestInfo): Promise<string> {
  const current = await (await page.request.get('/version-chooser/v1.0/version/current')).json()
  const suffix = ASSET_ARCHITECTURE[current.architecture]
  expect(suffix, `the ${current.architecture} architecture should have a release image`).toBeDefined()
  const release = await fetch(`https://api.github.com/repos/bluerobotics/BlueOS/releases/tags/${RELEASE}`)
  expect(release.ok, `the ${RELEASE} release should be found`).toBe(true)
  const assets: { name: string, browser_download_url: string }[] = (await release.json()).assets
  const asset = assets.find((candidate) => candidate.name.endsWith(`-${suffix}.tar`))
  expect(asset, `the ${RELEASE} release should have the ${suffix} image`).toBeDefined()
  const { name, browser_download_url: url } = asset as typeof assets[number]

  const folder = join(testInfo.config.rootDir, '..', '.cache')
  const file = join(folder, name)
  if (!existsSync(file)) {
    mkdirSync(folder, { recursive: true })
    const download = await fetch(url)
    expect(download.ok && download.body, `${name} should download`).toBeTruthy()
    // A download that stops halfway does not leave a broken file behind
    await pipeline(Readable.fromWeb(download.body as never), createWriteStream(`${file}.part`))
    renameSync(`${file}.part`, file)
  }
  return file
}

test('Version chooser shows the pirate mode sections', async ({ page }) => {
  await page.goto('/tools/version-chooser')
  for (const heading of ['Local Versions', 'Remote Versions', 'Manual Upload']) {
    await expect(page.getByRole('heading', { name: heading })).toBeVisible({ timeout: 30_000 })
  }
})

test(`Version chooser uploads the ${RELEASE} image and deletes it`, async ({ page }, testInfo) => {
  test.setTimeout(LOAD_TIMEOUT + 10 * 60_000)
  const file = await test.step(`get the ${RELEASE} release image`, () => releaseImage(page, testInfo))
  const local = page.getByTestId('version-card-local')
    .filter({ has: page.getByText(RELEASE, { exact: true }) })
    .filter({ has: page.getByText(REPOSITORY, { exact: true }) })
  const decompressing = page.getByText('Decompressing file, this usually takes up to one minute, please stand by...')

  await page.goto('/tools/version-chooser')
  await expect(page.getByRole('heading', { name: 'Manual Upload' })).toBeVisible({ timeout: 30_000 })

  await test.step('upload the image', async () => {
    await page.getByLabel('File input', { exact: true }).setInputFiles(file)
    const load = page.waitForResponse('**/version-chooser/v1.0/version/load/', { timeout: LOAD_TIMEOUT })
    await page.getByRole('button', { name: 'Upload', exact: true }).click()
    // The bar follows the upload, then the vehicle decompresses the image until the request answers
    await expect(decompressing, 'the upload should reach 100%').toBeVisible({ timeout: LOAD_TIMEOUT })
    expect((await load).status(), 'the vehicle should load the image').toBe(200)
    await expect(decompressing, 'the decompress message should go away').toBeHidden()
    await expect(page.getByRole('button', { name: 'Upload', exact: true }), 'the upload should be available again')
      .toBeVisible()
  })

  await test.step(`${RELEASE} is listed after a reload`, async () => {
    await page.reload()
    await expect(local, `${REPOSITORY}:${RELEASE} should be a local version`).toBeVisible({ timeout: 60_000 })
  })

  await test.step(`delete ${RELEASE}`, async () => {
    const deleted = page.waitForResponse('**/version-chooser/v1.0/version/delete')
    await local.getByRole('button', { name: 'Delete', exact: true }).click()
    expect((await deleted).status(), 'the vehicle should delete the image').toBe(200)
    await expect(local, `${RELEASE} should leave the list`).toHaveCount(0)
  })

  await test.step(`${RELEASE} is gone after a reload`, async () => {
    await page.reload()
    await expect(page.getByTestId('version-card-local').first(), 'the local versions should load')
      .toBeVisible({ timeout: 60_000 })
    await expect(local, `${RELEASE} should not come back`).toHaveCount(0)
  })
})

test('Version chooser pages and searches the remote versions', async ({ page }) => {
  await page.route(REMOTE_VERSIONS, (route) => {
    const repository = route.request().url().split('/available/')[1]
    return route.fulfill({ json: remoteVersions(repository, REMOTE_TAGS) })
  })
  const remote = page.getByTestId('version-chooser-remote')
  const tags = remote.getByTestId('version-card-tag')
  await page.goto('/tools/version-chooser')

  await test.step('the first page has the 10 newest versions', async () => {
    // The remote list waits for the vehicle internet check, slow after a boot
    await expect(tags).toHaveText(REMOTE_TAGS.slice(0, 10), { timeout: BACKEND_TIMEOUT })
    await expect(remote.getByTestId('version-card-download-and-apply')).toHaveCount(10)
  })

  await test.step('the second page has the rest', async () => {
    await remote.getByTestId('version-chooser-pagination').getByRole('button', { name: /2/ }).click()
    await expect(tags).toHaveText(REMOTE_TAGS.slice(10))
  })

  const repository = remote.getByTestId('version-chooser-repository')
  await test.step('the official images have no search', async () => {
    await repository.fill('bluerobotics/blueos-core')
    await repository.press('Enter')
    await expect(remote.getByTestId('version-card-repository').first()).toHaveText('bluerobotics/blueos-core')
    await expect(remote.getByTestId('version-chooser-search')).toHaveCount(0)
  })

  await test.step('another repository can be searched', async () => {
    await repository.fill('e2e/blueos-core')
    await repository.press('Enter')
    await expect(remote.getByTestId('version-card-repository').first()).toHaveText('e2e/blueos-core')
    await remote.getByTestId('version-chooser-search').fill('golf')
    await expect(tags, 'only golf should be left').toHaveText(['golf'])
  })
})

test('Version chooser retries the remote versions', async ({ page }) => {
  // The list loads again when the internet check finishes, so it fails until the retry
  let failing = true
  await page.route(REMOTE_VERSIONS, (route) => {
    const repository = route.request().url().split('/available/')[1]
    const error = { local: [], remote: [], error: 'Docker Hub is out for lunch' }
    return route.fulfill({ json: failing ? error : remoteVersions(repository, REMOTE_TAGS) })
  })
  const remote = page.getByTestId('version-chooser-remote')
  await page.goto('/tools/version-chooser')

  const error = remote.getByTestId('version-chooser-remote-error')
  await expect(error, 'the error should be shown')
    .toContainText('Docker Hub is out for lunch', { timeout: BACKEND_TIMEOUT })
  await expect(remote.getByTestId('version-card-remote')).toHaveCount(0)
  failing = false
  await error.getByTestId('version-chooser-remote-retry').click()
  await expect(error, 'the error should go away').toBeHidden()
  await expect(remote.getByTestId('version-card-remote')).toHaveCount(10)
})
