import { expect, test } from '../../fixtures/recorded-test.fixture'

test.describe.configure({ mode: 'serial' })

test.describe('Dashboard', { tag: '@dashboard' }, () => {
  test('Dashboard shows the system cards and no backend warning', async ({ page, pageErrors }) => {
    await page.goto('/')
    for (const card of ['Digital Twin', 'System Information', 'Ethernet Status', 'WiFi Status']) {
      await expect(page.getByText(card, { exact: true }).first()).toBeVisible()
    }
    for (const usage of ['CPU Usage', 'Memory Usage', 'Disk Usage']) {
      await expect(page.getByText(usage)).toBeVisible()
    }
    await expect(page.getByText('Disconnected', { exact: true })).toHaveCount(0)
    await expect(page.getByRole('status')).toBeHidden()
    expect(pageErrors).toEqual([])
  })

  test('Unknown routes show the not found page', async ({ page, pageErrors }) => {
    await page.goto('/this-page-does-not-exist')
    await expect(page.getByText('Page not found!')).toBeVisible()
    expect(pageErrors).toEqual([])
  })
})
