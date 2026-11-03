import { expect, test } from '../../fixtures/recorded-test.fixture'
import { activeDialog } from '../../fixtures/vehicle'

test.describe.configure({ mode: 'serial' })

const LOADED = /No video-devices available\.|Device Controls/

test.describe('Video Streams', { tag: '@video' }, () => {
  test('Video page lists the devices or says none are available', async ({ page, pageErrors }) => {
    await page.goto('/vehicle/video-manager')
    await expect(page.locator('.v-navigation-drawer').getByRole('link', { name: 'Video Streams' })).toBeVisible()
    await expect(page.getByText(LOADED).first()).toBeVisible({ timeout: 30_000 })
    if (await page.getByRole('button', { name: 'Device Controls' }).count()) {
      await expect(page.getByRole('button', { name: 'Add stream' }).first()).toBeVisible()
    }
    // DESTRUCTIVE – not exercised: remove stream fabs, block source switches
    expect(pageErrors).toEqual([])
  })

  test('Video settings dialog opens and closes without resetting', async ({ page, pageErrors }) => {
    await page.goto('/vehicle/video-manager')
    await expect(page.getByText(LOADED).first()).toBeVisible({ timeout: 30_000 })
    await page.locator('.v-btn--fab:has(.mdi-cog)').click()
    const dialog = activeDialog(page)
    await expect(dialog.getByText('Video Manager Settings')).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(dialog.getByText('Raspberry legacy camera support')).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(dialog.getByRole('button', { name: 'Reset Settings' })).toBeVisible()
    await page.keyboard.press('Escape')
    await expect(dialog).toBeHidden()
    expect(pageErrors).toEqual([])
  })

  test('Stream creation dialog opens and is cancelled', async ({ page, pageErrors }) => {
    await page.goto('/vehicle/video-manager')
    await expect(page.getByText(LOADED).first()).toBeVisible({ timeout: 30_000 })
    const add = page.getByRole('button', { name: 'Add stream' }).and(page.locator(':enabled')).first()
    test.skip(!(await add.count()), 'No video source accepts a new stream')
    await add.click()
    const dialog = activeDialog(page)
    await expect(dialog.getByText('Stream creation')).toBeVisible()
    await expect(dialog.getByLabel('Stream nickname')).toBeVisible()
    await expect(dialog.getByLabel('Encoding')).toBeVisible()
    // DESTRUCTIVE – not exercised
    await expect(dialog.getByRole('button', { name: 'Create' })).toBeVisible()
    await dialog.getByRole('button', { name: 'Cancel' }).click()
    await expect(dialog).toBeHidden()
    expect(pageErrors).toEqual([])
  })
})
