import { expect, test } from '../../fixtures/recorded-test.fixture'
import { activeDialog } from '../../fixtures/vehicle'

test.describe.configure({ mode: 'serial' })

test.describe('Settings', { tag: '@settings' }, () => {
  test('Settings page shows every section', async ({ page, pageErrors }) => {
    await page.goto('/settings')
    await expect(page.getByRole('heading', { name: 'Settings', level: 1 })).toBeVisible()
    for (const section of ['Appearance', 'Advanced Mode', 'Data Management', 'System']) {
      await expect(page.getByText(section, { exact: true })).toBeVisible()
    }
    for (const item of ['Dark Theme', 'Light Theme', 'Pirate Mode', 'Developer Mode']) {
      await expect(page.getByText(item, { exact: true })).toBeVisible()
    }
    await expect(page.getByRole('button', { name: 'Download' }).first()).toBeEnabled()
    // DESTRUCTIVE – not exercised
    await expect(page.getByRole('button', { name: 'Clear', exact: true }).first()).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(page.getByRole('button', { name: 'Run Wizard' })).toBeEnabled()
    // DESTRUCTIVE – not exercised
    await expect(page.getByRole('button', { name: 'Reset Settings' })).toBeEnabled()
    expect(pageErrors).toEqual([])
  })

  test('Reset settings dialog opens and is cancelled', async ({ page, pageErrors }) => {
    await page.goto('/settings')
    await page.getByRole('button', { name: 'Reset Settings' }).click()
    const dialog = activeDialog(page)
    await expect(dialog.getByText('WARNING')).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(dialog.getByRole('button', { name: 'Yes, reset settings' })).toBeVisible()
    await dialog.getByRole('button', { name: 'Cancel' }).click()
    await expect(dialog).toBeHidden()
    expect(pageErrors).toEqual([])
  })

  test('Customization panel expands without applying', async ({ page, pageErrors }) => {
    await page.goto('/settings')
    await page.getByText('Customization', { exact: true }).click()
    // DESTRUCTIVE – not exercised
    await expect(page.getByRole('button', { name: 'Apply', exact: true })).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(page.getByRole('button', { name: 'Reset', exact: true })).toBeVisible()
    await page.getByRole('button', { name: 'Upload model' }).click()
    const dialog = activeDialog(page)
    await expect(dialog.getByText('Upload 3D model override')).toBeVisible()
    await dialog.getByRole('button', { name: 'Cancel' }).click()
    await expect(dialog).toBeHidden()
    expect(pageErrors).toEqual([])
  })
})
