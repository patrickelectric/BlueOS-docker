import { expect, test } from '../../fixtures/recorded-test.fixture'

test.describe.configure({ mode: 'serial' })

test.describe('Ping Sonar Devices', { tag: '@pings' }, () => {
  test('Pings page lists detected devices or says none are available', async ({ page, pageErrors }) => {
    await page.goto('/vehicle/pings')
    await expect(page.locator('.v-navigation-drawer').getByRole('link', { name: 'Ping Sonar Devices' })).toBeVisible()
    await expect(page.getByText(/No Ping devices available\.|Ping1D|Ping360/).first()).toBeVisible()
    // DESTRUCTIVE – not exercised: the "MAVLink Distances" switch enables a driver on the vehicle
    expect(pageErrors).toEqual([])
  })
})
