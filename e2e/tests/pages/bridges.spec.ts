import { expect, test } from '../../fixtures/recorded-test.fixture'
import { activeDialog } from '../../fixtures/vehicle'

test.describe.configure({ mode: 'serial' })

test.describe('Serial Bridges', { tag: '@bridges' }, () => {
  test('Serial bridges page opens the new bridge dialog and switches modes', async ({ page, pageErrors }) => {
    await page.goto('/tools/bridges')
    await expect(page.locator('.v-navigation-drawer').getByRole('link', { name: 'Serial Bridges' })).toBeVisible()
    await page.locator('button:has(.mdi-plus)').click()
    const dialog = activeDialog(page)
    await expect(dialog.getByText('New bridge')).toBeVisible()
    await expect(dialog.getByLabel('Serial baudrate')).toBeVisible()
    await expect(dialog.getByLabel('IP address (Server mode)')).toBeVisible()
    await dialog.getByRole('tab', { name: 'Client Mode' }).click()
    await expect(dialog.getByLabel('Topside/Target port')).toBeVisible()
    await dialog.getByRole('tab', { name: 'Server Mode' }).click()
    await expect(dialog.getByLabel('Topside/Target port')).toBeHidden()
    // DESTRUCTIVE – not exercised
    await expect(dialog.getByRole('button', { name: 'Create' })).toBeVisible()
    await dialog.getByRole('button', { name: 'Cancel' }).click()
    await expect(dialog).toBeHidden()
    expect(pageErrors).toEqual([])
  })
})
