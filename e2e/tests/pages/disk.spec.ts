import { expect, test } from '../../fixtures/recorded-test.fixture'

test.describe.configure({ mode: 'serial' })

test.describe('Disk', { tag: '@disk' }, () => {
  test('Disk page shows the usage table for the root path', async ({ page, pageErrors }) => {
    await page.goto('/tools/disk')
    await expect(page.locator('.v-navigation-drawer').getByRole('link', { name: 'Disk', exact: true })).toBeVisible()
    await expect(page.getByRole('tab', { name: 'Disk Usage' })).toBeVisible()
    await expect(page.getByRole('tab', { name: 'Speed Test' })).toBeVisible()
    await expect(page.getByText(/^\/( \(.*\))?$/)).toBeVisible()
    for (const header of ['Name', 'Usage', 'Size']) {
      await expect(page.getByRole('columnheader', { name: header })).toBeVisible()
    }
    await expect(page.locator('button:has(.mdi-arrow-up)')).toBeDisabled()
    // DESTRUCTIVE – not exercised
    await expect(page.locator('button:has(.mdi-delete)')).toBeDisabled()
    expect(pageErrors).toEqual([])
  })

  test('Speed test tab shows the idle state without running it', async ({ page, pageErrors }) => {
    await page.goto('/tools/disk')
    await page.getByRole('tab', { name: 'Speed Test' }).click()
    await expect(page.getByText('Disk Speed Test')).toBeVisible()
    await expect(page.getByText('Click to start').first()).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(page.locator('button:has(.mdi-play)')).toBeEnabled()
    expect(pageErrors).toEqual([])
  })
})
