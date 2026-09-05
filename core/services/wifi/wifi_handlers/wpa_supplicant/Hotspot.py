import asyncio
import hashlib
import pathlib
import shlex
import shutil
import subprocess
import tempfile
import time
from ipaddress import IPv4Address
from typing import Any, Callable, List, Optional

import psutil
from commonwealth.utils.DHCPServerManager import Dnsmasq as DHCPServerManager
from commonwealth.utils.general import HostOs, device_id, get_host_os
from loguru import logger
from pyroute2 import IW, IPRoute

from band import (
    COUNTRY_CODE,
    FALLBACK_AP_FREQUENCY,
    channel_from_frequency,
    is_5ghz,
    radio_supports_5ghz,
)
from typedefs import WifiCredentials


class HotspotManager:
    # pylint: disable=too-many-instance-attributes
    # pylint: disable=too-many-arguments
    def __init__(
        self,
        base_interface: str,
        ipv4_gateway: IPv4Address,
        ap_interface_name: str = "uap0",
        ap_ssid: Optional[str] = None,
        ap_passphrase: Optional[str] = None,
    ) -> None:
        self.iw = IW()
        self.ipr = IPRoute()

        self._ap_interface_name = ap_interface_name
        self.supports_hotspot = self.check_hotspot_support()
        try:
            dev_id = device_id()
        except Exception:
            dev_id = "000000"
        hashed_id = hashlib.md5(dev_id.encode()).hexdigest()[:6]
        self._ap_ssid = ap_ssid or f"BlueOS ({hashed_id})"

        self._ap_passphrase = ap_passphrase or "blueosap"

        self._subprocess: Optional[Any] = None

        if base_interface not in psutil.net_if_stats():
            raise ValueError(f"Base interface '{base_interface}' not found.")
        self._base_interface = base_interface

        self._ipv4_gateway = ipv4_gateway

        self._include_interface_on_dhcpcd()

        self._dhcp_server: Optional[DHCPServerManager] = None

        binary_path = shutil.which(self.binary_name())
        if binary_path is None:
            raise ValueError("Hostapd binary not found on system's PATH.")

        self._binary = pathlib.Path(binary_path)
        assert self.is_binary_working()

    @staticmethod
    def binary_name() -> str:
        return "hostapd"

    def binary(self) -> pathlib.Path:
        return self._binary

    def check_hotspot_support(self) -> bool:
        # Support for Bookworm should arrive with NetworkManager support
        return bool(get_host_os() == HostOs.Bullseye) and radio_supports_5ghz()

    def set_credentials(self, credentials: WifiCredentials) -> None:
        logger.debug(f"Changing hotspot ssid to '{credentials.ssid}' and passphrase to '{credentials.password}'.")
        self._ap_ssid = credentials.ssid
        self._ap_passphrase = credentials.password

    @property
    def credentials(self) -> WifiCredentials:
        return WifiCredentials(ssid=self._ap_ssid, password=self._ap_passphrase)

    def is_binary_working(self) -> bool:
        try:
            subprocess.check_output([self.binary(), "-h"])
            return True
        except subprocess.CalledProcessError as error:
            if error.returncode == 1:
                return True
            logger.error(f"Invalid binary: {error}")
            return False

    def base_interface_channel_frequency(self) -> int:
        wireless_interfaces = self.iw.get_interfaces_dict()
        if self._base_interface not in wireless_interfaces:
            raise RuntimeError("Could not find base interface.")
        last_channel = -1
        time_last_channel_change = time.time()
        time_start_searching = time.time()
        while True:
            current_channel = int(wireless_interfaces[self._base_interface][3])
            if current_channel != last_channel:
                time_last_channel_change = time.time()
                last_channel = current_channel
            seconds_in_same_channel = time.time() - time_last_channel_change
            seconds_searching = time.time() - time_start_searching
            if seconds_in_same_channel > 2:
                return current_channel
            if seconds_searching > 15:
                raise RuntimeError("Could not find base interface channel. Timeout exceeded.")

    def desired_channel_frequency(self) -> int:
        # uap0 shares wlan0's radio, so the access point has to beacon on whatever channel the station
        # already holds. The station is restricted to 5 GHz, but it can be unassociated or still settling,
        # in which case we pick a channel ourselves.
        try:
            frequency = self.base_interface_channel_frequency()
        except RuntimeError as error:
            logger.debug(f"Could not read base interface channel, falling back to default: {error}")
            return FALLBACK_AP_FREQUENCY
        return frequency if is_5ghz(frequency) else FALLBACK_AP_FREQUENCY

    def _create_virtual_interface(self) -> None:
        logger.debug("Deleting virtual access point interface (if exists).")
        wireless_interfaces = self.iw.get_interfaces_dict()
        if self._ap_interface_name in wireless_interfaces:
            interface_index = int(self.ipr.link_lookup(ifname=self._ap_interface_name)[0])
            self.iw.del_interface(interface_index)
        self._reach_condition_or_timeout(
            lambda self: self._ap_interface_name not in self.iw.get_interfaces_dict(),
            "Could not delete virtual interface. Timeout exceeded.",
        )

        logger.debug("Creating virtual access point interface.")
        # pylint: disable=consider-using-with
        subprocess.Popen(
            shlex.split(f"iw dev {self._base_interface} interface add {self._ap_interface_name} type __ap")
        )
        # Following 2 lines are an alternative I could not get to work since its docs are not very clear
        # base_interface_index = int(self.ipr.link_lookup(ifname=self._base_interface)[0])
        # self.iw.add_interface(ifname=self._ap_interface_name, iftype="ap_vlan", dev=base_interface_index)
        self._reach_condition_or_timeout(
            lambda self: self._ap_interface_name in psutil.net_if_stats(),
            "Could not create virtual interface. Timeout exceeded.",
        )

        logger.debug("Starting virtual access point interface.")
        # pylint: disable=consider-using-with
        subprocess.Popen(shlex.split(f"ifconfig {self._ap_interface_name} up"))
        # Following 2 lines are an alternative I could not get to work since its docs are not very clear
        # virtual_interface_index = int(self.ipr.link_lookup(ifname=self._ap_interface_name)[0])
        # self.ipr.link("set", index=virtual_interface_index, state="up")
        self._reach_condition_or_timeout(
            lambda self: self._ap_interface_name in psutil.net_if_stats() and psutil.net_if_stats()[self._ap_interface_name][0],  # fmt: skip
            "Could not start virtual interface. Timeout exceeded.",
        )

    def command_list(self) -> List[str]:
        return shlex.split(f"{self.binary()} {self.config_path()}")

    async def start(self) -> None:
        logger.info("Starting hotspot.")
        if not self.supports_hotspot:
            raise RuntimeError("Hotspot not supported on this device.")
        try:
            self._create_temp_config_file()
            self._create_virtual_interface()
            # pylint: disable=consider-using-with
            if not self.is_running():
                self._subprocess = subprocess.Popen(self.command_list(), shell=False, encoding="utf-8", errors="ignore")
                await asyncio.sleep(3)
                if not self.is_running():
                    exit_code = self._subprocess.returncode
                    raise RuntimeError(f"Failed to initialize Hostapd ({exit_code}).")
            if not self._dhcp_server:
                self._dhcp_server = DHCPServerManager(self._ap_interface_name, self._ipv4_gateway)
                return
            await self._dhcp_server.restart()
        except Exception as error:
            raise RuntimeError(f"Unable to start hotspot. {error}") from error

    def stop(self) -> None:
        logger.info("Stopping hotspot.")
        if self.is_running():
            assert self._subprocess is not None
            self._subprocess.kill()
            if not self._dhcp_server:
                logger.warning("Cannot stop DHCP server for hotspot, as was already not running.")
                return
            self._dhcp_server.stop()
        else:
            logger.info("Tried to stop hostpot, but it was already not running.")

    async def restart(self) -> None:
        self.stop()
        await self.start()

    def is_running(self) -> bool:
        if not self.supports_hotspot:
            return False
        return self._subprocess is not None and self._subprocess.poll() is None

    @staticmethod
    def config_path() -> pathlib.Path:
        config_dir = pathlib.Path(tempfile.tempdir or "/")
        return config_dir.joinpath("hostapd.conf")

    def hostapd_config(self) -> str:
        return (
            "# WiFi interface to be used (in this case a virtual one)\n"
            f"interface={self._ap_interface_name}\n"
            "# Channel (frequency) of the access point\n"
            f"channel={channel_from_frequency(self.desired_channel_frequency())}\n"
            "# SSID broadcasted by the access point\n"
            f'ssid2="{self._ap_ssid}"\n'
            "# Passphrase for the access point\n"
            f"wpa_passphrase={self._ap_passphrase}\n"
            "# Operation mode. 'a' is 5GHz, the only band we serve.\n"
            "hw_mode=a\n"
            "# Regulatory domain. Without it the world domain marks every 5GHz channel as no-IR\n"
            "# and hostapd is not allowed to beacon at all.\n"
            f"country_code={COUNTRY_CODE}\n"
            "ieee80211d=1\n"
            "# Without HT/VHT a 5GHz access point is limited to 802.11a rates (54 Mbps)\n"
            "ieee80211n=1\n"
            "ieee80211ac=1\n"
            "ht_capab=[HT40+][SHORT-GI-20][SHORT-GI-40]\n"
            "# Accept all MAC addresses\n"
            "macaddr_acl=0\n"
            "# Use WPA authentication\n"
            "auth_algs=1\n"
            "# Require clients to know the network name\n"
            "ignore_broadcast_ssid=0\n"
            "# Use WPA2\n"
            "wpa=2\n"
            "# Use a pre-shared key\n"
            "wpa_key_mgmt=WPA-PSK\n"
            # wpa_pairwise is deliberately unset: offering TKIP as a pairwise cipher makes hostapd
            # silently disable HT, which would cap the hotspot at 802.11a rates.
            "rsn_pairwise=CCMP\n"
        )

    def _create_temp_config_file(self) -> None:
        logger.info(f"Saving temporary hostapd config file on {self.config_path()}")
        with open(self.config_path(), "w", encoding="utf-8") as f:
            f.write(self.hostapd_config())

    def _include_interface_on_dhcpcd(self) -> None:
        if not self.supports_hotspot:
            return
        with open("/etc/dhcpcd.conf", "r", encoding="utf-8") as f:
            original_lines = f.readlines()

            start_line = -1
            end_line = -1
            for i, line in enumerate(original_lines):
                if "blueos-start" in line:
                    start_line = i
                if "blueos-end" in line:
                    end_line = i
            lines_to_remove: List[int] = []
            if start_line != -1 and end_line != -1:
                lines_to_remove = list(range(start_line, end_line + 1))

            new_lines = []
            for i, line in enumerate(original_lines):
                if i not in lines_to_remove:
                    new_lines.append(line)

            if not str(new_lines[-1]).endswith("\n"):
                new_lines.append("\n")

            if str(new_lines[-1]) != "\n":
                new_lines.append("\n")
            new_lines.append("#blueos-start\n")
            new_lines.append(f"interface {self._ap_interface_name}\n")
            new_lines.append(f"    static ip_address={self._ipv4_gateway}/24\n")
            new_lines.append("    nohook wpa_supplicant\n")
            new_lines.append("END\n")
            new_lines.append("#blueos-end\n")

        with open("/etc/dhcpcd.conf", "w", encoding="utf-8") as f:
            f.writelines(new_lines)

    def _reach_condition_or_timeout(self, condition: Callable[["HotspotManager"], bool], timeout_message: str) -> None:
        time_start = time.time()
        while True:
            if condition(self):
                time.sleep(0.3)
                break
            if time.time() - time_start > 5:
                raise RuntimeError(timeout_message)
            time.sleep(0.1)
