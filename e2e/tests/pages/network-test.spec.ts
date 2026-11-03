import { expect, test } from '../../fixtures/recorded-test.fixture'

test.describe.configure({ mode: 'serial' })

test.describe('Network Test', { tag: '@network-test' }, () => {
  test('Network test page shows the local and internet tests', async ({ page, pageErrors }) => {
    await page.goto('/tools/network-test')
    await expect(page.locator('.v-navigation-drawer').getByRole('link', { name: 'Network Test' })).toBeVisible()
    await expect(page.getByRole('tab', { name: 'Local network test' })).toBeVisible()
    await expect(page.getByRole('tab', { name: 'Internet speed test' })).toBeVisible()
    await expect(page.getByText('Local Network Speed and Latency Test')).toBeVisible()
    await expect(page.getByText('Click to start').first()).toBeVisible()
    // Not exercised: saturates the link for a minute
    await expect(page.locator('button:has(.mdi-play)').first()).toBeEnabled()
    await page.getByRole('tab', { name: 'Internet speed test' }).click()
    await expect(page.getByText(/Internet Speed and Latency Test|Waiting for internet connection/)).toBeVisible()
    expect(pageErrors).toEqual([])
  })
})
