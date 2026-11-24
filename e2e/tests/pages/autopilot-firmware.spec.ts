// Tests for the BlueOS Autopilot Firmware page. They change the board to SITL and install firmware on the vehicle,
// then put the original board back, and the original firmware too when the board was already SITL.
// How to run: BLUEOS_URL=http://<ip> bun --cwd e2e e2e autopilot-firmware.spec.ts
import { Page } from '@playwright/test'

import {
  BACKEND_TIMEOUT, changeBoard, currentVehicle, currentVersion, expectVehicle, INSTALL_TIMEOUT, installFirmware,
  openPanel, openSelect, selectStableFirmware, selectVehicle, Vehicle, waitSerialsLoaded,
} from '@/fixtures/autopilot'
import { expect, preparePage, test } from '@/fixtures/vehicle'

// Each board keeps its own firmware and parameters, so only SITL needs its vehicle and version back
let original: { board: string, vehicle?: Vehicle, version?: string } | undefined

async function restoreOriginal(page: Page): Promise<void> {
  if (!original) {
    return
  }
  const board = page.getByTestId('autopilot-board-name')
  if ((await board.innerText()).trim() !== original.board) {
    await changeBoard(page, original.board)
    await expect(board, `the board should be back to ${original.board}`)
      .toHaveText(original.board, { timeout: BACKEND_TIMEOUT })
  }
  const { vehicle, version } = original
  if (!vehicle || !version) {
    return
  }
  if (await currentVehicle(page) === vehicle && await currentVersion(page) === version) {
    return
  }
  await selectVehicle(page, vehicle)
  const firmwares = await openSelect(page, 'firmware-version-select')
  await firmwares.getByRole('option', { name: `STABLE-${version}`, exact: true }).click()
  await installFirmware(page)
  await expectVehicle(page, vehicle)
}

test.describe.serial('Autopilot Firmware', () => {
  test.setTimeout(300_000)

  // The board is changed here so that running only some scenarios still starts on SITL
  test.beforeAll(async ({ browser }, testInfo) => {
    testInfo.setTimeout(BACKEND_TIMEOUT * 3)
    const page = await browser.newPage({ baseURL: testInfo.project.use.baseURL })
    try {
      await preparePage(page)
      await page.goto('/vehicle/autopilot')
      const board = page.getByTestId('autopilot-board-name')
      await expect(board, 'the current board should be known').toHaveText(/^\s*(?!Unknown)\S/, { timeout: BACKEND_TIMEOUT })
      const name = (await board.innerText()).trim()
      if (name === 'SITL') {
        original = { board: name, vehicle: await currentVehicle(page), version: await currentVersion(page) }
      } else {
        original = { board: name }
        await changeBoard(page, /SITL|Software In The Loop/)
      }
      await expect(board, 'the board should become SITL').toHaveText(/^\s*SITL\s*$/, { timeout: BACKEND_TIMEOUT })
      await openPanel(page, 'serial-configuration-panel')
      await waitSerialsLoaded(page)
    } finally {
      await page.close()
    }
  })

  // A failed scenario skips the ones after it, afterAll still runs
  test.afterAll(async ({ browser }, testInfo) => {
    testInfo.setTimeout(INSTALL_TIMEOUT + BACKEND_TIMEOUT * 2)
    const page = await browser.newPage({ baseURL: testInfo.project.use.baseURL })
    await preparePage(page)
    await page.goto('/vehicle/autopilot')
    await expect(page.getByTestId('autopilot-board-name'), 'the current board should be known')
      .toHaveText(/^\s*(?!Unknown)\S/, { timeout: BACKEND_TIMEOUT })
    await restoreOriginal(page)
    await page.close()
  })

  test.beforeEach(async ({ page }) => {
    await page.goto('/vehicle/autopilot')
    await expect(page.getByTestId('autopilot-board-name'), 'the current board should be known')
      .toHaveText(/^\s*(?!Unknown)\S/, { timeout: BACKEND_TIMEOUT })
  })

  test('Scenario 1: banner and vehicle type match the current firmware', async ({ page }) => {
    await expectVehicle(page, await currentVehicle(page))
  })

  test('Scenario 2: firmware list shows the vehicles and the stable versions', async ({ page }) => {
    const running = await currentVersion(page)
    await openPanel(page, 'firmware-update-panel')
    const vehicles = await openSelect(page, 'firmware-vehicle-select')
    for (const vehicle of ['Sub', 'Rover', 'Plane', 'Copter']) {
      const option = vehicles.getByRole('option', { name: vehicle, exact: true })
      await expect(option, `the ${vehicle} vehicle should be listed`).toBeVisible()
    }
    await vehicles.getByRole('option', { name: 'Sub', exact: true }).click()
    await selectStableFirmware(page, running)
    // Nothing is installed here
    await expect(page.getByTestId('install-firmware-button'), 'Install firmware should be available').toBeEnabled()
  })

  test('Scenario 3: the Vectored or Vectored 6DOF frame is available', async ({ page }) => {
    await openPanel(page, 'sitl-configuration-panel')
    const frames = await openSelect(page, 'sitl-frame-select')
    const vectored = frames.getByRole('option', { name: /^vectored(_6dof)?$/ })
    // The list only renders more items as it scrolls
    await expect(async () => {
      await frames.evaluate((menu) => menu.scrollTo(0, menu.scrollHeight))
      await expect(vectored.first()).toBeAttached({ timeout: 1000 })
    }, 'the vectored or vectored_6dof frame should exist').toPass({ timeout: BACKEND_TIMEOUT })
    await page.keyboard.press('Escape')
  })

  test('Scenario 4: a serial combobox lists the Navigator paths', async ({ page }) => {
    await openPanel(page, 'serial-configuration-panel')
    await waitSerialsLoaded(page)
    await page.getByTestId('serial-port-C').click()
    const navigatorPort = page.getByTestId('serial-port-option')
      .filter({ hasText: /\/dev\/ttyAMA\d+/ })
      .filter({ has: page.locator('[data-testid="device-path-image"][data*="navigator"]') })
    await expect(navigatorPort.first(), 'a ttyAMA* path with the Navigator image should be listed')
      .toBeVisible({ timeout: BACKEND_TIMEOUT })
    // Close without changing anything, Save and restart is not clicked
    await page.keyboard.press('Escape')
    await expect(navigatorPort.first(), 'the list should close').toBeHidden()
    await expect(page.getByTestId('serial-port-C'), 'the serial should not be changed').toHaveValue('')
  })

  test('Scenario 5: installs the firmware alternating vehicle and version', async ({ page }) => {
    // Anything that is not ArduSub, like ArduCopter or ArduPlane, goes back to ArduSub
    const target: Vehicle = await currentVehicle(page) === 'ArduSub' ? 'ArduRover' : 'ArduSub'
    const running = await currentVersion(page)

    await selectVehicle(page, target)
    const version = await selectStableFirmware(page, running)
    await installFirmware(page)

    await expectVehicle(page, target)
    await expect(page.getByTestId('autopilot-firmware-version'), `the installed version should be ${version}`)
      .toContainText(version, { timeout: BACKEND_TIMEOUT })
  })
})
