import { expect, test } from '../../fixtures/recorded-test.fixture'
import { activeDialog } from '../../fixtures/vehicle'

test.describe.configure({ mode: 'serial' })

test.describe('Autopilot Firmware', { tag: '@autopilot' }, () => {
  test('Autopilot page shows the board information', async ({ page, pageErrors }) => {
    await page.goto('/vehicle/autopilot')
    await expect(page.locator('.v-navigation-drawer').getByRole('link', { name: 'Autopilot Firmware' })).toBeVisible()
    await expect(page.getByText('Autopilot', { exact: true })).toBeVisible()
    await expect(page.getByText(/Board name:|No board running/)).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(page.getByRole('button', { name: 'Start autopilot', exact: true })).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(page.getByRole('button', { name: 'Stop autopilot' })).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(page.getByRole('button', { name: 'Restart autopilot' })).toBeVisible()
    // DESTRUCTIVE – not exercised: re-enables the setup wizard
    await expect(page.locator('button:has(.mdi-wizard-hat)')).toBeVisible()
    expect(pageErrors).toEqual([])
  })

  test('Change board dialog opens and closes without applying', async ({ page, pageErrors }) => {
    await page.goto('/vehicle/autopilot')
    await page.getByRole('button', { name: 'Change board' }).click()
    const dialog = activeDialog(page)
    await expect(dialog.getByText('Change running board')).toBeVisible()
    await expect(dialog.getByLabel('Available boards')).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(dialog.getByRole('button', { name: 'Set' })).toBeVisible()
    await page.keyboard.press('Escape')
    await expect(dialog).toBeHidden()
    expect(pageErrors).toEqual([])
  })

  test('Firmware update panel shows the upload options without installing', async ({ page, pageErrors }) => {
    await page.goto('/vehicle/autopilot')
    await expect(page.getByText(/Board name:|No board running/)).toBeVisible()
    const panel = page.getByRole('button', { name: 'Firmware update' })
    test.skip(!(await panel.isVisible()), 'Board has no firmware update panel')
    await panel.click()
    for (const option of ['Retrieve from Cloud', 'Upload firmware file', 'Restore default firmware']) {
      await expect(page.getByText(option)).toBeVisible()
    }
    // DESTRUCTIVE – not exercised
    await expect(page.getByRole('button', { name: 'Install firmware' })).toBeDisabled()
    expect(pageErrors).toEqual([])
  })
})
