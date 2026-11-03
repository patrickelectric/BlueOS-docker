import { expect, test } from '../../fixtures/recorded-test.fixture'

test.describe.configure({ mode: 'serial' })

test.describe('Bag Editor', { tag: '@bag-editor' }, () => {
  test('Bag editor loads the database and switches between code and tree mode', async ({ page, pageErrors }) => {
    await page.goto('/tools/bag-editor')
    await expect(page.locator('.v-navigation-drawer').getByRole('link', { name: 'Bag Editor' })).toBeVisible()
    await expect(page.locator('.jsoneditor')).toBeVisible()
    await page.locator('button:has(.mdi-file-tree)').click()
    await expect(page.locator('button:has(.mdi-text-long)')).toBeVisible()
    await page.locator('button:has(.mdi-text-long)').click()
    await expect(page.locator('button:has(.mdi-file-tree)')).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(page.locator('button:has(.mdi-content-save-check-outline)')).toBeVisible()
    expect(pageErrors).toEqual([])
  })
})
