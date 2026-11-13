// Tests for the BlueOS System Information page. They only read, nothing is written on the vehicle.
// How to run: BLUEOS_URL=http://<ip> bun --cwd e2e e2e system-information.spec.ts
import { Locator, Page } from '@playwright/test'

import { expect, test } from '@/fixtures/vehicle'

// Same as the parameters test, the first system information answer can take a while on a busy vehicle
const BACKEND_TIMEOUT = 65_000
const PERCENT = /^\d+\.\d%$/
const SIZE = String.raw`\d+\.\d [kMG]B`
const TEMPERATURE = String.raw`-?\d+\.\dºC`
// formatRFC7231, E.g: Wed, 15 Nov 2023 14:05:00 GMT
const DATE = /\w{3}, \d{2} \w{3} \d{4} \d{2}:\d{2}:\d{2} GMT/

const PROCESS_COLUMNS = [
  { value: 'pid', name: 'PID' },
  { value: 'name', name: 'Name' },
  { value: 'cpu_usage', name: 'CPU (%)' },
  { value: 'parent_process', name: 'Parent PID' },
  { value: 'status', name: 'Status' },
  { value: 'running_time', name: 'Running time' },
  { value: 'used_memory_kB', name: 'Memory (kB)' },
  { value: 'virtual_memory_kB', name: 'Virt. Memory (kB)' },
  { value: 'working_directory', name: 'Working directory' },
  { value: 'root_directory', name: 'Root directory' },
  { value: 'command', name: 'Command' },
  { value: 'environment', name: 'Environment' },
  { value: 'executable_path', name: 'Executable path' },
]

async function openTab(page: Page, name: string): Promise<void> {
  const tab = page.getByRole('tab', { name })
  await tab.click()
  await expect(tab, `the ${name} tab should be selected`).toHaveAttribute('aria-selected', 'true')
}

// Vuetify draws its ripple over the checkbox input, so the click has to go through it
async function setChecked(checkbox: Locator, checked: boolean): Promise<void> {
  // A forced click does not wait for the element to finish moving
  await (await checkbox.elementHandle())?.waitForElementState('stable')
  await checkbox.setChecked(checked, { force: true })
}

function conditionCard(page: Page, title: RegExp): Locator {
  return page.getByTestId('system-condition-card')
    .filter({ has: page.getByTestId('system-condition-title').filter({ hasText: title }) })
}

function columnHeader(page: Page, name: string): Locator {
  // Exact, PID should not match Parent PID
  return page.getByRole('columnheader').getByText(name, { exact: true })
}

async function processCell(page: Page, row: Locator, name: string): Promise<Locator> {
  const headers = await page.getByRole('columnheader').allInnerTexts()
  const index = headers.findIndex((header) => header.trim() === name)
  expect(index, `the ${name} column should be shown`).not.toBe(-1)
  return row.getByRole('cell').nth(index)
}

// The list is virtual, it only renders what is around the scroll position
async function expectScrolledLines(list: Locator, lines: Locator, name: string): Promise<void> {
  await expect.poll(async () => {
    await list.evaluate((element) => element.scrollTo(0, element.scrollHeight))
    return lines.count()
  }, { message: `the ${name} should have more than 10 lines`, timeout: BACKEND_TIMEOUT }).toBeGreaterThan(10)
  // New lines keep arriving, so scroll again before checking it is at the bottom
  await expect.poll(() => list.evaluate((element) => {
    element.scrollTo(0, element.scrollHeight)
    return element.scrollTop + element.clientHeight >= element.scrollHeight - 1
  }), { message: `the ${name} should scroll to the bottom` }).toBe(true)
  await expect(lines.last(), `the last ${name} line should not be empty`).toHaveText(/\S/)
}

function aboutValue(page: Page, title: string): Locator {
  return page.getByTestId('about-item')
    .filter({ has: page.getByTestId('about-title').getByText(title, { exact: true }) })
    .getByTestId('about-value')
}

test.describe.serial('System information', () => {
  test.setTimeout(300_000)

  test.beforeEach(async ({ page }) => {
    await page.goto('/tools/system-information')
  })

  test('Scenario 1: System Monitor shows CPU, memory, disk and temperature', async ({ page }) => {
    await openTab(page, 'System Monitor')
    const cpu = conditionCard(page, /^\s*CPU /)
    const memory = conditionCard(page, /^\s*Memory\s*$/)
    const disk = conditionCard(page, /^\s*Disk\s*$/)
    const temperature = conditionCard(page, /^\s*Temperature\s*$/)

    await test.step('each block shows its reading', async () => {
      for (const [name, card, reading] of [
        ['CPU', cpu, PERCENT],
        ['memory', memory, PERCENT],
        ['disk', disk, PERCENT],
        ['temperature', temperature, new RegExp(`^${TEMPERATURE}$`)],
      ] as const) {
        await expect(card.getByTestId('system-condition-value'), `the ${name} block should show a reading`)
          .toHaveText(reading, { timeout: BACKEND_TIMEOUT })
      }
    })

    await test.step('CPU 0 shows its usage and clock', async () => {
      // E.g: cpu0: 12% (1500MHz)
      await expect(cpu.getByTestId('system-condition-text'), 'CPU 0 should show % and MHz')
        .toHaveText(/^[^:\n]*0: \d+% \(\d+MHz\)/, { timeout: BACKEND_TIMEOUT })
    })

    await test.step('memory shows RAM and swap used out of total', async () => {
      const text = memory.getByTestId('system-condition-text')
      await expect(text, 'RAM should show used / total').toContainText(new RegExp(`RAM: ${SIZE}/${SIZE}`))
      await expect(text, 'swap should show used / total').toContainText(new RegExp(`SWAP: ${SIZE}/${SIZE}`))
    })

    await test.step('disk shows the overlay root used out of total', async () => {
      // E.g: overlay '/' 12.345GB/29GB (overlay)
      await expect(disk.getByTestId('system-condition-text'), 'the overlay root should be listed')
        .toContainText(/overlay '\/'\s+\d+\.\d{3}GB\/\d+GB \(overlay\)/, { timeout: BACKEND_TIMEOUT })
    })

    await test.step('there is a temperature sensor with current, max and critical', async () => {
      await expect(temperature.getByTestId('system-condition-text'), 'a sensor should be listed')
        .toContainText(new RegExp(`\\S ${TEMPERATURE}, Max: ${TEMPERATURE}, Crit: ${TEMPERATURE}`))
    })
  })

  test('Scenario 2: Processes finds Python and toggles every column', async ({ page }) => {
    await openTab(page, 'Processes')
    const rows = page.getByRole('row').filter({ has: page.getByRole('cell') })

    await test.step('searching Python lists a Python 3 process', async () => {
      for (const value of ['root_directory', 'executable_path']) {
        await setChecked(page.getByTestId(`process-column-${value}`), true)
      }
      await page.getByTestId('process-search').fill('Python')
      const first = rows.first()
      await expect(first, 'a Python process should be listed').toContainText(/python/i, { timeout: BACKEND_TIMEOUT })
      await expect(first, 'it should run Python 3').toContainText(/python3/i)
      await expect(await processCell(page, first, 'PID'), 'PID should be a number').toHaveText(/^\s*\d+\s*$/)
      await expect(await processCell(page, first, 'CPU (%)'), 'CPU should be a percentage')
        .toHaveText(/^\s*\d+\.\d\s*$/)
      await expect(await processCell(page, first, 'Status'), 'status should be shown').toHaveText(/\w/)
      await expect(await processCell(page, first, 'Root directory'), 'root directory should be a path')
        .toHaveText(/^\s*\//)
      await expect(await processCell(page, first, 'Command'), 'command should be shown').toHaveText(/\S/)
    })

    for (const { value, name } of PROCESS_COLUMNS) {
      await test.step(`the ${name} column shows and hides`, async () => {
        const checkbox = page.getByTestId(`process-column-${value}`)
        const header = columnHeader(page, name)
        const shown = await checkbox.isChecked()
        for (const checked of [!shown, shown]) {
          await setChecked(checkbox, checked)
          await expect(header, `${name} should be ${checked ? 'shown' : 'hidden'}`).toHaveCount(checked ? 1 : 0)
        }
      })
    }
  })

  test('Scenario 3: Network shows an enabled interface with its traffic', async ({ page }) => {
    await openTab(page, 'Network')
    const card = page.getByTestId('network-card')
      .filter({ has: page.getByTestId('network-ip') })
      .filter({ has: page.getByTestId('network-state').filter({ hasText: /^\s*Enabled\s*$/ }) })
      .first()
    await expect(card, 'an enabled interface with an IP should be listed').toBeVisible({ timeout: BACKEND_TIMEOUT })

    await test.step('it has an IP', async () => {
      await expect(card.getByTestId('network-ip').first(), 'the IP should be an address')
        .toHaveText(/\d+\.\d+\.\d+\.\d+|[\da-f]*:[\da-f:]+/i)
    })

    await test.step('the MAC is hidden until clicked', async () => {
      const mac = card.getByTestId('network-mac')
      await expect(mac, 'the MAC should be blurred').toHaveAttribute('style', /blur/)
      await expect(mac, 'the MAC should be a MAC address').toHaveText(/^\s*([\da-f]{2}:){5}[\da-f]{2}\s*$/i)
      await mac.click()
      await expect(mac, 'the MAC should be shown after a click').not.toHaveAttribute('style', /blur/)
    })

    await test.step('packets and bytes show done / total', async () => {
      for (const direction of ['received', 'transmitted']) {
        await expect(card.getByTestId(`network-packets-${direction}`), `packets ${direction} should be counted`)
          .toHaveText(/^\s*\d+ \/ \d+ total\s*$/)
        await expect(card.getByTestId(`network-bytes-${direction}`), `bytes ${direction} should be counted`)
          .toHaveText(new RegExp(`^\\s*${SIZE} / ${SIZE} total\\s*$`))
      }
    })
  })

  test('Scenario 4: USB lists the buses and their controllers', async ({ page }) => {
    await openTab(page, 'USB')
    const buses = page.getByTestId('usb-bus')
    await expect(buses.first(), 'a USB bus should be listed').toBeVisible({ timeout: BACKEND_TIMEOUT })
    for (const bus of await buses.all()) {
      const name = bus.getByTestId('usb-bus-name')
      await expect(name, 'the bus should have a number').toHaveText(/^\s*USB Bus \d+\s*$/)
      // The root of each bus is its host controller
      await expect(bus.getByTestId('usb-device-name').first(), `${await name.innerText()} should show its controller`)
        .toHaveText(/\S/)
    }
  })

  test('Scenario 5: Kernel has more than 10 lines', async ({ page }) => {
    await openTab(page, 'Kernel')
    await expectScrolledLines(page.getByTestId('kernel-messages'), page.getByTestId('kernel-message'), 'kernel log')
  })

  test('Scenario 6: Journal has more than 10 lines', async ({ page }) => {
    await openTab(page, 'Journal')
    await expectScrolledLines(page.getByTestId('journal-entries'), page.getByTestId('journal-entry'), 'journal')
  })

  test('Scenario 7: Firmware shows the Raspberry Pi firmware, bootloader and USB controller', async ({ page }) => {
    await openTab(page, 'Firmware')
    const date = page.getByTestId('raspberry-firmware-date')
    const error = page.getByTestId('raspberry-firmware-error')
    await expect(date.or(error), 'the firmware should load').toBeVisible({ timeout: BACKEND_TIMEOUT })
    // vcgencmd only exists on a Raspberry Pi
    test.skip(await error.isVisible(), 'this board is not a Raspberry Pi')

    await test.step('the firmware has a date and version', async () => {
      await expect(date, 'the firmware should have a date').toContainText(DATE)
      // E.g: version 30aa0d70ab280427ba04ebc718c81d4350b9d394 (clean) (release) (start)
      await expect(page.getByTestId('raspberry-firmware-version'), 'the firmware should have a version')
        .toHaveText(/Version:\s*[\da-f]{7,}/)
    })

    const current = page.getByTestId('bootloader-current')
    await expect(current, 'the bootloader should load').toBeVisible({ timeout: BACKEND_TIMEOUT })
    // Only boards with an EEPROM (Pi 4 and later) report a bootloader
    test.skip(!/Current:\s*\S/.test(await current.innerText()), 'this board has no bootloader EEPROM')

    await test.step('the bootloader has a current and available version', async () => {
      await expect(current, 'the current bootloader should have a date').toContainText(DATE)
      await expect(page.getByTestId('bootloader-available'), 'the available bootloader should be shown')
        .toHaveText(new RegExp(`Latest:\\s*${DATE.source}|Up to date`))
    })

    const controller = page.getByTestId('usb-controller-firmware')
    // Only the Pi 4 has the VL085 USB controller
    test.skip(!/:\s*\S/.test(await controller.innerText()), 'this board has no VL085 USB controller')
    await expect(controller, 'the USB controller firmware should have a version').toHaveText(/:\s*[\da-f]+/i)
  })

  test('Scenario 8: About shows the OS, kernel, model, hostname and time', async ({ page }) => {
    await openTab(page, 'About')
    await expect(page.getByTestId('about-item').first(), 'the system info should load')
      .toBeVisible({ timeout: BACKEND_TIMEOUT })
    await expect(aboutValue(page, 'OS Type'), 'the OS type should be shown').toHaveText(/\S/)
    await expect(aboutValue(page, 'Kernel'), 'the kernel should be a 6.x one').toHaveText(/\b6\.\d+/)
    // E.g: Raspberry Pi 4 Model B Rev 1.4 - aarch64 (BCM2711)
    await expect(aboutValue(page, 'Model'), 'the model should be shown').toHaveText(/\S+ - \S+ \(.+\)/)
    await expect(aboutValue(page, 'Hostname'), 'the hostname should be shown').toHaveText(/\S/)
    await expect(aboutValue(page, 'Time'), 'the time should be a valid hh:mm:ss')
      .toHaveText(/^\s*([01]\d|2[0-3]):[0-5]\d:[0-5]\d\b/)
  })
})
