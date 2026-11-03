import { expect, test } from '../../fixtures/recorded-test.fixture'

test.describe.configure({ mode: 'serial' })

test.describe('File Browser', { tag: '@file-browser' }, () => {
  test('File browser page embeds the file browser', async ({ page, pageErrors }) => {
    await page.goto('/tools/file-browser')
    await expect(page.locator('.v-navigation-drawer').getByRole('link', { name: 'File Browser' })).toBeVisible()
    // DESTRUCTIVE – not exercised: the frame contents can delete and edit files
    await expect(page.locator('iframe[title^="iframe-/file-browser/"]')).toBeVisible()
    expect(pageErrors).toEqual([])
  })
})
