<template>
  <v-card
    elevation="1"
    width="500"
    max-height="80vh"
  >
    <v-app-bar
      v-if="showTopBar"
      elevate-on-scroll
    >
      <v-toolbar-title>Wifi</v-toolbar-title>
      <v-select
        v-if="available_interfaces.length > 1"
        v-model="selected_interface"
        :items="available_interfaces"
        item-text="name"
        item-value="name"
        dense
        hide-details
        class="ml-4 interface-selector"
        style="max-width: 120px;"
      />
      <v-spacer />
      <v-btn
        v-tooltip="'Toggle hotspot'"
        icon
        :color="hotspot_status ? 'success' : 'gray'"
        hide-details="auto"
        :loading="hotspot_status_loading"
        :disabled="hotspot_supported === false"
        @click="toggleHotspot"
      >
        <v-icon>{{ hotspot_status ? 'mdi-access-point' : 'mdi-access-point-off' }}</v-icon>
      </v-btn>
      <v-btn
        v-if="hotspot_status"
        v-tooltip="'Hotspot QR code'"
        icon
        color="primary"
        hide-details="auto"
        @click="toggleQrCodeDialog"
      >
        <v-icon>mdi-qrcode-scan</v-icon>
      </v-btn>
      <v-btn
        v-tooltip="'Settings'"
        icon
        color="gray"
        hide-details="auto"
        @click="toggleSettingsMenu"
      >
        <v-icon>mdi-cog</v-icon>
      </v-btn>
    </v-app-bar>

    <v-sheet v-if="!wifi_is_loading">
      <wifi-network-card
        v-if="current_network"
        connected
        :network="current_network"
        :ip-address="wifi_status.ip_address"
        @click="openDisconnectionDialog"
      />

      <v-sheet
        max-height="600"
        class="overflow-y-auto"
      >
        <div v-if="filtered_networks !== null">
          <v-text-field
            v-if="show_search"
            v-model="ssid_filter"
            append-icon="mdi-magnify"
            label="Search"
            single-line
            hide-details
            clearable
            class="ml-7 mr-7"
          />
          <div v-if="!filtered_networks.isEmpty()">
            <wifi-network-card
              v-for="(network, key) in filtered_networks"
              :key="key"
              class="available-network"
              :network="network"
              @click="openConnectionDialog"
            />
          </div>
          <v-card-text
            v-else
            flat
            class="text-body-1 text-center"
          >
            No wifi networks available :( <br>
            Rescanning...
          </v-card-text>
        </div>
        <div v-else>
          <spinning-logo
            size="30%"
            subtitle="Scanning for wifi networks..."
          />
        </div>
      </v-sheet>
    </v-sheet>
    <v-sheet v-else>
      <spinning-logo
        size="30%"
        subtitle="Waiting for wifi networks..."
      />
    </v-sheet>

    <connection-dialog
      v-if="selected_network"
      v-model="show_connection_dialog"
      :network="selected_network"
      :interface-name="active_interface"
      @forget="forgetNetwork"
    />

    <disconnection-dialog
      v-if="current_network"
      v-model="show_disconnection_dialog"
      :network="current_network"
      :status="wifi_status"
      :interface-name="active_interface"
    />

    <wifi-settings-dialog v-model="show_settings_menu" />

    <v-dialog
      v-model="show_qr_code_dialog"
      width="300"
    >
      <v-card>
        <v-card-title class="text-h5">
          Connect to the hotspot
        </v-card-title>
        <v-card-text>
          <div class="d-flex align-center justify-center">
            <img
              :src="wifi_qr_code_img"
              width="200"
              height="200"
            >
          </div>
          <span>Scan this QR code with your phone to connect to BlueOS's hotspot.</span>
        </v-card-text>
      </v-card>
    </v-dialog>
  </v-card>
</template>

<script lang="ts">
import { uniqBy } from 'lodash'
import Vue from 'vue'
import { generateWifiQRCode } from 'wifi-qr-code-generator'

import Notifier from '@/libs/notifier'
import wifi from '@/store/wifi'
import { wifi_service } from '@/types/frontend_services'
import { Network, WifiStatus, WlanInterface } from '@/types/wifi'
import back_axios from '@/utils/api'

import SpinningLogo from '../common/SpinningLogo.vue'
import ConnectionDialog from './ConnectionDialog.vue'
import DisconnectionDialog from './DisconnectionDialog.vue'
import WifiNetworkCard from './WifiNetworkCard.vue'
import WifiSettingsDialog from './WifiSettingsDialog.vue'

const notifier = new Notifier(wifi_service)

export default Vue.extend({
  name: 'WifiManager',
  components: {
    WifiNetworkCard,
    SpinningLogo,
    ConnectionDialog,
    DisconnectionDialog,
    WifiSettingsDialog,
  },
  props: {
    showTopBar: {
      type: Boolean,
      default: true,
    },
    interfaceName: {
      type: String,
      default: null,
    },
  },
  data() {
    return {
      selected_network: null as Network | null,
      show_connection_dialog: false,
      show_disconnection_dialog: false,
      hotspot_status_loading: false,
      show_settings_menu: false,
      show_qr_code_dialog: false,
      wifi_qr_code_img: '',
      ssid_filter: undefined as string | undefined,
    }
  },
  computed: {
    active_interface(): string | null {
      return this.interfaceName || wifi.current_interface
    },
    wifi_is_loading(): boolean {
      if (this.active_interface) {
        const data = wifi.interface_data[this.active_interface]
        return !data || data.available_networks === null
      }
      return wifi.is_loading
    },
    wifi_status(): WifiStatus | null {
      if (this.active_interface) {
        return wifi.interface_data[this.active_interface]?.network_status ?? null
      }
      return wifi.network_status
    },
    current_network(): Network | null {
      let network: Network | null = null
      if (this.active_interface) {
        network = wifi.interface_data[this.active_interface]?.current_network ?? null
      } else {
        network = wifi.current_network
      }
      this.$emit('current-network', network)
      return network
    },
    interface_available_networks(): Network[] | null {
      if (this.active_interface) {
        return wifi.interface_data[this.active_interface]?.available_networks ?? null
      }
      return wifi.available_networks
    },
    interface_connectable_networks(): Network[] | null {
      const networks = this.interface_available_networks
      if (networks === null) {
        return null
      }
      const currentSsid = this.current_network?.ssid
      return networks.filter((network: Network) => network.ssid !== currentSsid)
    },
    connectable_networks(): Network[] | undefined {
      const networks = this.interface_connectable_networks
      if (networks === null) {
        return undefined
      }
      return uniqBy(networks, 'ssid')
        // Move known networks to the top
        .sort((a: Network, b: Network) => Number(b.saved) - Number(a.saved))
    },
    filtered_networks(): Network[] | undefined {
      // eslint-disable-next-line eqeqeq
      if (this.ssid_filter == undefined || this.ssid_filter.trim() === '') {
        return this.connectable_networks ?? undefined
      }
      const filter = this.ssid_filter
      return this.connectable_networks?.filter(
        (network) => network.ssid.toLowerCase().includes(filter.toLowerCase()),
      )
    },
    hotspot_status(): boolean | null {
      return wifi.hotspot_status?.enabled ?? null
    },
    hotspot_supported(): boolean | null {
      return wifi.hotspot_status?.supported ?? null
    },
    show_search(): boolean {
      if (!this.connectable_networks) {
        return false
      }
      return this.connectable_networks.length > 12
    },
    available_interfaces(): WlanInterface[] {
      return wifi.available_interfaces
    },
    selected_interface: {
      get(): string | null {
        return wifi.current_interface
      },
      set(value: string) {
        wifi.setCurrentInterface(value)
      },
    },
  },
  watch: {
    hotspot_status(): void {
      this.hotspot_status_loading = false
    },
  },
  methods: {
    forgetNetwork(network: Network): void {
      wifi.forgettNetwork(network)
    },
    openConnectionDialog(network: Network): void {
      this.selected_network = network
      this.show_connection_dialog = true
    },
    openDisconnectionDialog(): void {
      this.show_disconnection_dialog = true
    },
    toggleSettingsMenu(): void {
      this.show_settings_menu = true
    },
    async toggleQrCodeDialog(): Promise<void> {
      await this.updateQrCode()
      this.show_qr_code_dialog = true
    },
    async updateQrCode(): Promise<void> {
      if (wifi.hotspot_credentials === null) { return }
      const qrCodePromise = generateWifiQRCode({
        ssid: wifi.hotspot_credentials.ssid,
        password: wifi.hotspot_credentials.password,
        encryption: 'WPA',
        hiddenSSID: false,
        outputFormat: { type: 'image/png' },
      })
      const data = await qrCodePromise
      console.log(data)
      this.wifi_qr_code_img = data
    },
    async toggleHotspot(): Promise<void> {
      this.hotspot_status_loading = true
      const params: Record<string, unknown> = { enable: !this.hotspot_status }
      if (this.active_interface) {
        params.interface = this.active_interface
      }
      await back_axios({
        method: 'post',
        url: `${wifi.API_URL}/hotspot`,
        params,
        timeout: 20000,
      })
        .then(() => {
          notifier.pushSuccess('HOTSPOT_STATUS_TOGGLE_SUCCESS', 'Successfully toggled hotspot state.')
        })
        .catch((error) => {
          notifier.pushBackError('HOTSPOT_STATUS_TOGGLE_FAIL', error, true)
          this.hotspot_status_loading = false
        })
    },
  },
})
</script>
