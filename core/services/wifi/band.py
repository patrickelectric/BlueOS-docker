"""5 GHz band policy shared by the wpa_supplicant and NetworkManager handlers."""

import re
import subprocess

from loguru import logger

COUNTRY_CODE = "US"
FALLBACK_AP_FREQUENCY = 5180  # Channel 36, non-DFS in every regulatory domain
FIVE_GHZ_FREQUENCIES = range(5170, 5900, 5)

_PHY_5GHZ_CHANNEL = re.compile(r"\s*\* (49\d\d|5\d{3})(\.\d+)? MHz")


def is_5ghz(frequency: int) -> bool:
    """Frequencies in MHz. 4900-4999 is the 802.11j band, which hostapd also drives with hw_mode=a."""
    return frequency >= 4900


def channel_from_frequency(frequency: int) -> int:
    return (frequency - 5000) // 5


def radio_supports_5ghz() -> bool:
    """2.4 GHz-only radios (Pi 3B, Zero 2 W) can never serve our 5 GHz-only hotspot."""
    try:
        phy_info = subprocess.check_output(["iw", "phy"], encoding="utf-8", errors="ignore")
    except (subprocess.CalledProcessError, FileNotFoundError, OSError) as error:
        logger.error(f"Could not read radio capabilities: {error}")
        return False
    return any(_PHY_5GHZ_CHANNEL.match(line) and "disabled" not in line for line in phy_info.splitlines())
