import { expect, test } from '../../fixtures/recorded-test.fixture'
import { activeDialog } from '../../fixtures/vehicle'

test.describe.configure({ mode: 'serial' })

const PARAMETERS_TIMEOUT = 90_000

test.describe('Vehicle Setup', { tag: '@vehicle-setup' }, () => {
  test.beforeEach(() => test.setTimeout(PARAMETERS_TIMEOUT + 60_000))

  test('Overview tab shows the vehicle cards', async ({ page, pageErrors }) => {
    await page.goto('/vehicle/setup')
    await expect(page.locator('.v-navigation-drawer').getByRole('link', { name: 'Vehicle Setup' })).toBeVisible()
    for (const tab of ['Overview', 'PWM Outputs', 'Configure']) {
      await expect(page.getByRole('tab', { name: tab })).toBeVisible()
    }
    await expect(page.getByText('Flight Controller')).toBeVisible({ timeout: PARAMETERS_TIMEOUT })
    await expect(page.getByText('Firmware', { exact: true })).toBeVisible()
    const cards = ['Autopilot Sensors', 'Ping Sensors', 'Video', 'Lights', 'Gripper', 'Leak', 'Battery Monitor']
    for (const card of cards) {
      await expect(page.getByText(card, { exact: true })).toBeVisible()
    }
    expect(pageErrors).toEqual([])
  })

  test('PWM outputs tab shows the motor test without arming', async ({ page, pageErrors }) => {
    await page.goto('/vehicle/setup/pwm_outputs')
    await expect(page.getByText('Motor Test')).toBeVisible({ timeout: PARAMETERS_TIMEOUT })
    for (const header of ['Name', 'Value', 'Output']) {
      await expect(page.getByRole('columnheader', { name: header })).toBeVisible()
    }
    // DESTRUCTIVE – not exercised: arms the vehicle
    await expect(page.getByText(/^Disarmed/)).toBeVisible()
    expect(pageErrors).toEqual([])
  })

  test('Configure parameters subtab lists the parameter sets without loading one', async ({ page, pageErrors }) => {
    await page.goto('/vehicle/setup/configure/parameters')
    await expect(page.getByText('Load Recommended Parameter sets')).toBeVisible({ timeout: PARAMETERS_TIMEOUT })
    const subtabs = ['Parameters', 'Vehicle Body', 'Gyroscope', 'Accelerometer', 'Compass', 'Baro', 'Failsafes',
      'Power', 'Camera Gimbal', 'Vehicle ID']
    for (const subtab of subtabs) {
      await expect(page.getByRole('tab', { name: subtab, exact: true })).toBeVisible()
    }
    await expect(page.getByText('Reset Parameters to Firmware Defaults')).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(page.getByRole('button', { name: 'Reset All Parameters' })).toBeVisible()
    expect(pageErrors).toEqual([])
  })

  test('Sensor subtabs show their calibration controls without calibrating', async ({ page, pageErrors }) => {
    await page.goto('/vehicle/setup/configure/gyroscope')
    await expect(page.getByText('Calibrate Gyroscopes').first()).toBeVisible({ timeout: PARAMETERS_TIMEOUT })
    // DESTRUCTIVE – not exercised
    await expect(page.getByRole('button', { name: 'Calibrate Gyroscopes' })).toBeVisible()

    await page.getByRole('tab', { name: 'Accelerometer' }).click()
    for (const button of ['Start Full Calibration', 'Quick Calibration', 'Level Horizon']) {
      await expect(page.getByRole('button', { name: button })).toBeVisible()
    }
    await page.getByRole('button', { name: 'Start Full Calibration' }).click()
    const dialog = activeDialog(page)
    await expect(dialog.getByText('Accelerometer Calibration')).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(dialog.getByRole('button', { name: 'Start Calibration' })).toBeVisible()
    await page.keyboard.press('Escape')
    await expect(dialog).toBeHidden()

    await page.getByRole('tab', { name: 'Compass' }).click()
    await expect(page.getByText('Declination', { exact: true })).toBeVisible()
    await expect(page.getByText('Compass Calibration', { exact: true })).toBeVisible()

    await page.getByRole('tab', { name: 'Baro' }).click()
    await expect(page.getByText('Calibrate Barometer')).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(page.getByRole('button', { name: 'Calibrate', exact: true })).toBeVisible()
    expect(pageErrors).toEqual([])
  })

  test('Vehicle subtabs show their configuration without changing it', async ({ page, pageErrors }) => {
    await page.goto('/vehicle/setup/configure/vehiclebody')
    await expect(page.getByText('Frame', { exact: true })).toBeVisible({ timeout: PARAMETERS_TIMEOUT })
    await expect(page.getByText('Board Orientation')).toBeVisible()

    await page.getByRole('tab', { name: 'Failsafes' }).click()
    await expect(page.getByText(/Control Station Heartbeat Loss|Low Battery|Crash Detection/).first()).toBeVisible()

    await page.getByRole('tab', { name: 'Power' }).click()
    await expect(page.getByText('Battery 1')).toBeVisible()
    await page.getByRole('button', { name: 'Show Advanced' }).first().click()
    await expect(page.getByText('Voltage Multiplier').first()).toBeVisible()
    await page.getByRole('button', { name: 'Hide Advanced' }).first().click()

    await page.getByRole('tab', { name: 'Camera Gimbal' }).click()
    await expect(page.getByText('Camera Gimbal Configuration')).toBeVisible()

    await page.getByRole('tab', { name: 'Vehicle ID' }).click()
    await expect(page.getByText('Configure MAVLink system ID')).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(page.getByRole('button', { name: 'Save', exact: true })).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(page.getByRole('button', { name: 'Restart Core' })).toBeVisible()
    expect(pageErrors).toEqual([])
  })
})
