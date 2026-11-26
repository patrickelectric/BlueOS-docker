// Tests for the network test page. The local test moves 100 MB between the browser and the vehicle, the internet test
// asks the vehicle to run a speed test against the closest public server, so the vehicle needs internet.
// How to run: BLUEOS_URL=http://<ip> bun --cwd e2e e2e network-test.spec.ts
import { Locator, Page } from '@playwright/test'

import { expect, test } from '@/fixtures/vehicle'

// The page gives up on each direction of the local test after 20 seconds
const LOCAL_TIMEOUT = 20_000
// And on each step of the internet test after 15 seconds
const INTERNET_TIMEOUT = 15_000
// The offline overlay stays in the page and only shows while it is active
const ACTIVE_OVERLAY = /v-overlay--active/

let internetResult: Record<string, string> | undefined

async function openTab(page: Page, tab: string): Promise<void> {
  await page.goto('/tools/network-test')
  await page.getByTestId(`${tab}-tab`).click()
}

// Speeds are shown as "12.34 Mbps", and as "... Mbps" until there is one
async function expectSpeed(speed: Locator, name: string): Promise<void> {
  await expect(speed, `the ${name} speed should be shown`).toHaveText(/^\s*\d+\.\d{2} Mbps\s*$/)
  expect(parseFloat(await speed.innerText()), `the ${name} speed should not be zero`).toBeGreaterThan(0)
}

test.describe('Network test', () => {
  test('Scenario 1: the latency to the vehicle is measured', async ({ page }) => {
    await openTab(page, 'network-speed-test')
    // -1 means the websocket failed
    await expect(page.getByTestId('network-speed-test-latency'), 'the latency should be shown')
      .toHaveText(/^\s*\d+ ms\s*$/)
  })

  test('Scenario 2: the local network test downloads from and uploads to the vehicle', async ({ page }) => {
    test.setTimeout(LOCAL_TIMEOUT * 2 + 30_000)
    // Chromium hands the upload body to Playwright as text, an ArrayBuffer always and a Blob while requests are
    // routed. 100 MB of it is more than Node can hold and the page stops answering, so the same bytes go as a Blob
    // and the routes, only needed to load the page, are removed before the test starts
    await page.addInitScript(() => {
      const send = XMLHttpRequest.prototype.send
      XMLHttpRequest.prototype.send = function (body) {
        send.call(this, body instanceof ArrayBuffer ? new Blob([body]) : body)
      }
    })
    await openTab(page, 'network-speed-test')
    const start = page.getByTestId('network-speed-test-start')
    const state = page.getByTestId('network-speed-test-state')
    await expect(state).toHaveText('Click to start')
    await page.unrouteAll({ behavior: 'wait' })
    // A failed direction only goes to the notifications and the console, the test still ends as done
    const failures: string[] = []
    page.on('console', (message) => {
      if (message.type() === 'error' && message.text().startsWith('Failed to do speed test')) {
        failures.push(message.text())
      }
    })
    await start.click()
    await expect(state, 'the test should start with the download').toHaveText('Downloading from vehicle..')
    await expect(start, 'the test should not start twice').toBeDisabled()
    await expect(state, 'the test should end').toHaveText('Test done', { timeout: LOCAL_TIMEOUT * 2 })
    expect(failures, 'the download and the upload should finish').toEqual([])
    await expect(start, 'the test can run again').toBeEnabled()
    await expectSpeed(page.getByTestId('network-speed-test-download'), 'download')
    await expectSpeed(page.getByTestId('network-speed-test-upload'), 'upload')
  })

  test('Scenario 3: the internet test finds a server and measures both directions', async ({ page }) => {
    test.setTimeout(INTERNET_TIMEOUT * 3 + 30_000)
    await openTab(page, 'internet-speed-test')
    const start = page.getByTestId('internet-speed-test-start')
    const message = page.getByTestId('internet-speed-test-message')
    await expect(page.getByTestId('internet-speed-test-offline'), 'the vehicle should have internet')
      .not.toHaveClass(ACTIVE_OVERLAY)
    // A failed step only goes to the notifications and the test carries on, so each answer is kept here
    const steps: Record<string, string> = {}
    const step = (url: string) => /\/network-test\/(internet_(best_server|download_speed|upload_speed))/.exec(url)?.[1]
    page.on('response', (response) => {
      const name = step(response.url())
      if (name) {
        steps[name] = String(response.status())
      }
    })
    page.on('requestfailed', (request) => {
      const name = step(request.url())
      if (name) {
        steps[name] = request.failure()?.errorText ?? 'failed'
      }
    })
    await start.click()
    await expect(start, 'the test should not start twice').toBeDisabled()
    await expect(message, 'the test should look for a server').toHaveText('Starting.. Looking for best server..')
    // The page moves on by itself when a step times out
    await expect(message, 'the test should measure the download')
      .toHaveText('Checking download speed..', { timeout: INTERNET_TIMEOUT + 5_000 })
    await expect(message, 'the test should measure the upload')
      .toHaveText('Checking upload speed..', { timeout: INTERNET_TIMEOUT + 5_000 })
    await expect(message, 'the test should end').toHaveText('Done!', { timeout: INTERNET_TIMEOUT + 5_000 })
    expect(steps, 'every step should succeed').toEqual({
      internet_best_server: '200',
      internet_download_speed: '200',
      internet_upload_speed: '200',
    })
    await expect(start, 'the test can run again').toBeEnabled()
    const server = page.getByTestId('internet-speed-test-server')
    const provider = page.getByTestId('internet-speed-test-provider')
    const latency = page.getByTestId('internet-speed-test-latency')
    await expect(server, 'the server should be shown as country - name').toHaveText(/^\s*\S.* - .*\S\s*$/)
    await expect(provider, 'the provider should be shown').not.toHaveText(/^\s*\.\.\./)
    await expect(latency, 'the latency should be shown').toHaveText(/^\s*\d+ ms\s*$/)
    await expectSpeed(page.getByTestId('internet-speed-test-download'), 'download')
    await expectSpeed(page.getByTestId('internet-speed-test-upload'), 'upload')
    internetResult = Object.fromEntries(await Promise.all(['server', 'provider', 'download', 'upload', 'latency']
      .map(async (name) => [name, await page.getByTestId(`internet-speed-test-${name}`).innerText()])))
  })

  test('Scenario 4: the last internet result is shown again after a reload', async ({ page }) => {
    test.skip(!internetResult, 'needs the internet test from scenario 3')
    await openTab(page, 'internet-speed-test')
    for (const [name, value] of Object.entries(internetResult!)) {
      await expect(page.getByTestId(`internet-speed-test-${name}`), `the ${name} should be kept`).toHaveText(value)
    }
  })

  test('Scenario 5: the internet test waits for a connection', async ({ page }) => {
    // Every site the vehicle checks answers as unreachable, the vehicle itself is not touched
    await page.route('**/helper/latest/check_internet_access', async (route) => {
      const sites: Record<string, { online: boolean }> = await (await route.fetch()).json()
      for (const site of Object.values(sites)) {
        Object.assign(site, { online: false, error: 'unreachable', error_kind: null })
      }
      await route.fulfill({ json: sites })
    })
    await openTab(page, 'internet-speed-test')
    const offline = page.getByTestId('internet-speed-test-offline')
    await expect(offline, 'the test should wait for internet').toHaveClass(ACTIVE_OVERLAY)
    await expect(offline).toContainText('Waiting for internet connection..')
  })
})
