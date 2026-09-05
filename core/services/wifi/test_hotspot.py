import sys
from pathlib import Path
from unittest.mock import MagicMock, patch

_WIFI_DIR = str(Path(__file__).resolve().parent)
sys.path.insert(0, _WIFI_DIR)
for _name in ("band", "exceptions", "typedefs"):
    _mod = sys.modules.get(_name)
    if _mod is not None and not str(getattr(_mod, "__file__", "")).startswith(_WIFI_DIR):
        del sys.modules[_name]

# Restore after import so other collected tests still see real commonwealth (pytest-xdist).
_STUBBED = (
    "pyroute2",
    "settings",
    "commonwealth",
    "commonwealth.utils",
    "commonwealth.utils.DHCPServerManager",
    "commonwealth.utils.general",
    "commonwealth.settings",
    "commonwealth.settings.manager",
    "commonwealth.settings.settings",
    "fastapi",
)
_previous = {name: sys.modules.get(name) for name in _STUBBED}
for name in _STUBBED:
    sys.modules[name] = MagicMock()

from band import (
    COUNTRY_CODE,
    FALLBACK_AP_FREQUENCY,
    channel_from_frequency,
    is_5ghz,
    radio_supports_5ghz,
)
from wifi_handlers.wpa_supplicant.Hotspot import HotspotManager

for name, previous in _previous.items():
    if previous is None:
        del sys.modules[name]
    else:
        sys.modules[name] = previous


def _bare_hotspot() -> HotspotManager:
    hotspot = HotspotManager.__new__(HotspotManager)
    hotspot.supports_hotspot = True
    hotspot._ap_interface_name = "uap0"
    hotspot._ap_ssid = "BlueOS (abc123)"
    hotspot._ap_passphrase = "blueosap"
    return hotspot


def test_is_5ghz_splits_the_bands() -> None:
    assert is_5ghz(FALLBACK_AP_FREQUENCY)
    assert is_5ghz(4915)  # 802.11j, also driven with hw_mode=a
    assert not is_5ghz(2437)
    assert not is_5ghz(0)  # An unassociated interface reports no channel


def test_channel_from_frequency_covers_the_whole_5ghz_band() -> None:
    assert channel_from_frequency(FALLBACK_AP_FREQUENCY) == 36
    assert channel_from_frequency(5745) == 149
    # Channels the lookup table this replaced was missing entirely
    assert channel_from_frequency(5650) == 130
    assert channel_from_frequency(5350) == 70


def test_desired_channel_frequency_follows_a_5ghz_station() -> None:
    # uap0 shares wlan0's radio, so the access point cannot pick its own channel while associated
    hotspot = _bare_hotspot()
    with patch.object(hotspot, "base_interface_channel_frequency", return_value=5745):
        assert hotspot.desired_channel_frequency() == 5745


def test_desired_channel_frequency_never_follows_the_station_to_24ghz() -> None:
    hotspot = _bare_hotspot()
    with patch.object(hotspot, "base_interface_channel_frequency", return_value=2437):
        assert hotspot.desired_channel_frequency() == FALLBACK_AP_FREQUENCY


def test_desired_channel_frequency_falls_back_when_station_is_unassociated() -> None:
    hotspot = _bare_hotspot()
    with patch.object(hotspot, "base_interface_channel_frequency", return_value=0):
        assert hotspot.desired_channel_frequency() == FALLBACK_AP_FREQUENCY


def test_desired_channel_frequency_falls_back_when_channel_lookup_times_out() -> None:
    hotspot = _bare_hotspot()
    with patch.object(hotspot, "base_interface_channel_frequency", side_effect=RuntimeError("timeout")):
        assert hotspot.desired_channel_frequency() == FALLBACK_AP_FREQUENCY


def test_hostapd_config_is_5ghz_only_and_leaves_ht_available() -> None:
    hotspot = _bare_hotspot()
    with patch.object(hotspot, "desired_channel_frequency", return_value=FALLBACK_AP_FREQUENCY):
        config = hotspot.hostapd_config()
    assert "hw_mode=a" in config
    assert "channel=36" in config
    # Without a regulatory domain hostapd is not allowed to beacon on 5GHz at all
    assert f"country_code={COUNTRY_CODE}" in config
    assert "ieee80211n=1" in config
    # Offering TKIP as a pairwise cipher makes hostapd silently disable HT
    assert "wpa_pairwise" not in config


def test_radio_supports_5ghz_rejects_a_24ghz_only_radio() -> None:
    phy_info = "\t\t\t* 2412.0 MHz [1] (20.0 dBm)\n\t\t\t* 2437.0 MHz [6] (20.0 dBm)\n"
    with patch("band.subprocess.check_output", return_value=phy_info):
        assert radio_supports_5ghz() is False


def test_radio_supports_5ghz_ignores_disabled_channels() -> None:
    disabled = "\t\t\t* 5180.0 MHz [36] (disabled)\n"
    with patch("band.subprocess.check_output", return_value=disabled):
        assert radio_supports_5ghz() is False
    with patch("band.subprocess.check_output", return_value=disabled + "\t\t\t* 5745 MHz [149] (30.0 dBm)\n"):
        assert radio_supports_5ghz() is True


def test_radio_supports_5ghz_is_false_when_iw_is_missing() -> None:
    with patch("band.subprocess.check_output", side_effect=FileNotFoundError("iw")):
        assert radio_supports_5ghz() is False
