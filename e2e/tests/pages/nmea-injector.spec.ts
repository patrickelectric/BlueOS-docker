import { expect, test } from '../../fixtures/recorded-test.fixture'
import { activeDialog } from '../../fixtures/vehicle'

test.describe.configure({ mode: 'serial' })

test.describe('NMEA Injector', { tag: '@nmea-injector' }, () => {
  test('NMEA injector page shows the intro and opens the new socket dialog', async ({ page, pageErrors }) => {
    await page.goto('/tools/nmea-injector')
    await expect(page.locator('.v-navigation-drawer').getByRole('link', { name: 'NMEA Injector' })).toBeVisible()
    await expect(page.getByText('The NMEA Injector receives NMEA via UDP or TCP')).toBeVisible()
    await page.locator('button:has(.mdi-plus)').click()
    const dialog = activeDialog(page)
    await expect(dialog.getByText('New NMEA socket')).toBeVisible()
    await expect(dialog.getByLabel('Socket kind')).toBeVisible()
    await expect(dialog.getByLabel('Socket port')).toBeVisible()
    await expect(dialog.getByLabel('Mavlink component ID', { exact: true })).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(dialog.getByRole('button', { name: 'Create' })).toBeEnabled()
    await dialog.getByRole('button', { name: 'Cancel' }).click()
    await expect(dialog).toBeHidden()
    expect(pageErrors).toEqual([])
  })
})
