import { expect, test } from '../../fixtures/recorded-test.fixture'

test.describe.configure({ mode: 'serial' })

test.describe('Log Browser', { tag: '@logs' }, () => {
  test('Log browser lists the autopilot logs and refreshes the list', async ({ page, pageErrors }) => {
    await page.goto('/vehicle/logs')
    await expect(page.locator('.v-navigation-drawer').getByRole('link', { name: 'Log Browser' })).toBeVisible()
    await expect(page.getByText('Autopilot logs')).toBeVisible({ timeout: 30_000 })
    for (const header of ['Name', 'Size', 'Type', 'Modified', 'Actions']) {
      await expect(page.getByRole('columnheader', { name: header })).toBeVisible()
    }
    await page.locator('button:has(.mdi-update)').click()
    await expect(page.getByRole('table')).toBeVisible()
    await expect(page.locator('button:has(.mdi-download-multiple)')).toBeDisabled()
    // DESTRUCTIVE – not exercised
    await expect(page.locator('button:has(.mdi-trash-can)')).toBeDisabled()
    expect(pageErrors).toEqual([])
  })
})
