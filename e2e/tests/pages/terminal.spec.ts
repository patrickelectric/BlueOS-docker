import { expect, test } from '../../fixtures/recorded-test.fixture'

test.describe.configure({ mode: 'serial' })

test.describe('Terminal', { tag: '@terminal' }, () => {
  test('Terminal page embeds the web terminal', async ({ page, pageErrors }) => {
    await page.goto('/tools/web-terminal')
    await expect(page.locator('.v-navigation-drawer').getByRole('link', { name: 'Terminal' })).toBeVisible()
    // DESTRUCTIVE – not exercised: the frame is a shell
    await expect(page.locator('iframe[title="iframe-/terminal/"]')).toBeVisible()
    expect(pageErrors).toEqual([])
  })
})
