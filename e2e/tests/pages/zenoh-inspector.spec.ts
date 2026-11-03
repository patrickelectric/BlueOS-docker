import { expect, test } from '../../fixtures/recorded-test.fixture'

test.describe.configure({ mode: 'serial' })

test.describe('Zenoh Inspector', { tag: '@zenoh-inspector' }, () => {
  test('Zenoh inspector lists topics and selects one', async ({ page, pageErrors }) => {
    await page.goto('/tools/zenoh-inspector')
    await expect(page.locator('.v-navigation-drawer').getByRole('link', { name: 'Zenoh Inspector' })).toBeVisible()
    await expect(page.getByRole('tab', { name: 'Topics' })).toBeVisible()
    await expect(page.getByRole('tab', { name: 'Network' })).toBeVisible()
    await expect(page.getByLabel(/Search Topics \(\d+\)/)).toBeVisible()
    const placeholder = page.getByText('Select a topic to view its messages.')
    await expect(placeholder).toBeVisible()
    await expect(page.getByLabel(/Search Topics \([1-9]\d*\)/)).toBeVisible()
    await page.getByRole('radio').first().click()
    await expect(placeholder).toBeHidden()
    await expect(page.getByText(/^(Alive|Dead|Unknown)$/).first()).toBeVisible()
    expect(pageErrors).toEqual([])
  })

  test('Zenoh network tab draws the graph and refreshes it', async ({ page, pageErrors }) => {
    await page.goto('/tools/zenoh-inspector')
    await page.getByRole('tab', { name: 'Network' }).click()
    await expect(page.getByText('Zenoh Network')).toBeVisible()
    await expect(page.getByText('Node Types:')).toBeVisible()
    await expect(page.locator('#cy')).toBeVisible()
    await page.locator('button:has(.mdi-refresh)').click()
    await expect(page.getByText('Node Types:')).toBeVisible()
    expect(pageErrors).toEqual([])
  })
})
