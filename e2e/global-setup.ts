import { FullConfig } from '@playwright/test'

import { setPirateMode, withBrowser } from './fixtures/vehicle'

export default async function globalSetup(config: FullConfig): Promise<void> {
  const { storageState } = config.projects[0].use
  await withBrowser(config, async (page) => {
    await setPirateMode(page, true)
    // The frontend keeps pirate mode in the browser storage, so hand it to the test contexts
    await page.context().storageState({ path: storageState as string })
  })
}
