import { expect, test } from '../../fixtures/recorded-test.fixture'

test.describe.configure({ mode: 'serial' })

test.describe('MAVLink Inspector', { tag: '@mavlink-inspector' }, () => {
  test('MAVLink inspector page embeds mavlink2rest', async ({ page, pageErrors }) => {
    await page.goto('/tools/mavlink-inspector')
    await expect(page.locator('.v-navigation-drawer').getByRole('link', { name: 'MAVLink Inspector' })).toBeVisible()
    // DESTRUCTIVE – not exercised: mavlink2rest can send MAVLink commands
    const frame = page.locator('iframe[title^="iframe-http"]')
    await expect(frame).toBeVisible()
    await expect(frame).toHaveAttribute('src', /:8080/)
    expect(pageErrors).toEqual([])
  })
})
