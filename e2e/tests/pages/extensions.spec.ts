import { expect, test } from '../../fixtures/recorded-test.fixture'
import { activeDialog } from '../../fixtures/vehicle'

test.describe.configure({ mode: 'serial' })

const STORE_LOADED = /No Extensions Available|not connected to the internet|Failed to fetch extension manifest/

test.describe('Extensions', { tag: '@extensions' }, () => {
  test('Extension store loads and opens the filter and sort menus', async ({ page, pageErrors }) => {
    await page.goto('/tools/extensions-manager')
    await expect(page.locator('.v-navigation-drawer').getByRole('link', { name: 'Extensions', exact: true })).toBeVisible()
    await expect(page.getByRole('tab', { name: 'Store' })).toBeVisible()
    await expect(page.getByRole('tab', { name: 'Installed' })).toBeVisible()
    const cards = page.getByRole('button', { name: /^(GET|INSTALLED|UPDATE)$/ })
    await expect(cards.or(page.getByText(STORE_LOADED)).first()).toBeVisible({ timeout: 60_000 })
    if (await cards.count()) {
      await expect(page.getByPlaceholder('Search Extensions')).toBeVisible()
      await page.getByRole('button', { name: 'Filters' }).click()
      await expect(page.getByText('Clear', { exact: true })).toBeVisible()
      await page.keyboard.press('Escape')
      await page.getByRole('button', { name: 'Sort By' }).click()
      await expect(page.getByText('Popularity')).toBeVisible()
      await page.keyboard.press('Escape')
    }
    // DESTRUCTIVE – not exercised: UPDATE buttons on store cards
    expect(pageErrors).toEqual([])
  })

  test('Extension manifest dialog opens and is cancelled', async ({ page, pageErrors }) => {
    await page.goto('/tools/extensions-manager')
    await page.locator('button:has(.mdi-cog)').first().click()
    const dialog = activeDialog(page)
    await expect(dialog.getByText('Extensions Manifest')).toBeVisible()
    await expect(dialog.getByText('Drag manifest sources to set priority.')).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(dialog.getByRole('button', { name: 'Apply' })).toBeVisible()
    await dialog.getByRole('button', { name: 'Cancel' }).click()
    await expect(dialog).toBeHidden()
    expect(pageErrors).toEqual([])
  })

  test('Installed tab lists extensions and opens the add menu', async ({ page, pageErrors }) => {
    await page.goto('/tools/extensions-manager')
    await page.getByRole('tab', { name: 'Installed' }).click()
    const installed = page.getByRole('button', { name: 'Uninstall' })
    const empty = page.getByText(/No Extensions installed\.|Failed to fetch installed extensions/)
    await expect(installed.or(empty).first()).toBeVisible({ timeout: 60_000 })
    // DESTRUCTIVE – not exercised: Uninstall, Update, Disable, Enable and start, Restart
    await page.locator('button:has(.mdi-plus)').click()
    await expect(page.locator('button:has(.mdi-file-upload-outline)')).toBeVisible()
    await expect(page.locator('button:has(.mdi-code-braces)')).toBeVisible()
    await page.locator('button:has(.mdi-close)').click()
    await expect(page.locator('button:has(.mdi-code-braces)')).toBeHidden()
    expect(pageErrors).toEqual([])
  })
})
