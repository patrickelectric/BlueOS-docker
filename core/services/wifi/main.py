#! /usr/bin/env python3

import argparse
import asyncio
import logging
import os
import stat
from pathlib import Path
from typing import Any, Dict, List, Optional

from commonwealth.utils.apis import (
    GenericErrorHandlingRoute,
    PrettyJSONResponse,
    StackedHTTPException,
)
from commonwealth.utils.logs import InterceptHandler, init_logger
from commonwealth.utils.sentry_config import init_sentry_async
from exceptions import BusyError
from fastapi import FastAPI, HTTPException, status
from fastapi.staticfiles import StaticFiles
from fastapi_versioning import VersionedFastAPI, version
from loguru import logger
from tabulate import tabulate  # type: ignore
from typedefs import (
    HotspotStatus,
    InterfaceStatus,
    SavedWifiNetwork,
    ScannedWifiNetwork,
    WifiCredentials,
    WlanInterface,
)
from uvicorn import Config, Server
from wifi_handlers.AbstractWifiHandler import AbstractWifiManager
from wifi_handlers.networkmanager.networkmanager import NetworkManagerWifi
from wifi_handlers.wpa_supplicant.WifiManager import WifiManager

FRONTEND_FOLDER = Path.joinpath(Path(__file__).parent.absolute(), "frontend")
SERVICE_NAME = "wifi-manager"

logging.basicConfig(handlers=[InterceptHandler()], level=0)
init_logger(SERVICE_NAME)

logger.info("Starting Wifi Manager.")
# Primary wifi manager (for backwards compatibility)
wifi_manager: Optional[AbstractWifiManager] = None
# Dictionary of wifi managers per interface
wifi_managers: Dict[str, AbstractWifiManager] = {}


app = FastAPI(
    title="WiFi Manager API",
    description="WiFi Manager is responsible for managing WiFi connections on BlueOS.",
    default_response_class=PrettyJSONResponse,
)
app.router.route_class = GenericErrorHandlingRoute


def get_manager_for_interface(interface: Optional[str] = None) -> AbstractWifiManager:
    """Get the wifi manager for a specific interface, or the default one."""
    if interface is None:
        if wifi_manager is None:
            raise HTTPException(status_code=status.HTTP_503_SERVICE_UNAVAILABLE, detail="WiFi manager not initialized")
        return wifi_manager
    if interface not in wifi_managers:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=f"Interface '{interface}' not found")
    return wifi_managers[interface]


@app.get("/status", summary="Retrieve status of wifi manager.")
@version(1, 0)
async def network_status(interface: Optional[str] = None) -> Any:
    manager = get_manager_for_interface(interface)
    wifi_status = await manager.status()
    for line in tabulate(list(vars(wifi_status).items())).splitlines():
        logger.info(line)
    return wifi_status


async def _get_signal_for_network(manager: AbstractWifiManager, ssid: str) -> Optional[int]:
    """Get signal strength for a specific network from available networks list."""
    try:
        available = await manager.get_wifi_available()
        for net in available:
            if net.ssid == ssid:
                return net.signallevel
    except Exception:
        pass
    return None


@app.get("/all_status", response_model=List[InterfaceStatus], summary="Retrieve status of all wifi interfaces.")
@version(1, 0)
async def all_interfaces_status() -> List[InterfaceStatus]:
    """Get connection status for all available WiFi interfaces."""
    statuses: List[InterfaceStatus] = []
    for iface_name, manager in wifi_managers.items():
        try:
            wifi_status = await manager.status()
            current_network = await manager.get_current_network()
            signal = None
            if current_network:
                signal = await _get_signal_for_network(manager, current_network.ssid)
            statuses.append(
                InterfaceStatus(
                    interface=iface_name,
                    connected=wifi_status.wpa_state == "COMPLETED",
                    ssid=current_network.ssid if current_network else None,
                    signal=signal,
                )
            )
        except Exception as error:
            logger.warning(f"Could not get status for interface {iface_name}: {error}")
            statuses.append(InterfaceStatus(interface=iface_name, connected=False, ssid=None, signal=None))
    return statuses


@app.get("/scan", response_model=List[ScannedWifiNetwork], summary="Retrieve available wifi networks.")
@version(1, 0)
async def scan(interface: Optional[str] = None) -> Any:
    manager = get_manager_for_interface(interface)
    try:
        available_networks = await manager.get_wifi_available()
        return available_networks
    except BusyError as error:
        raise StackedHTTPException(status_code=status.HTTP_425_TOO_EARLY, error=error) from error


@app.get("/saved", response_model=List[SavedWifiNetwork], summary="Retrieve saved wifi networks.")
@version(1, 0)
async def saved(interface: Optional[str] = None) -> Any:
    manager = get_manager_for_interface(interface)
    saved_networks = await manager.get_saved_wifi_network()
    return saved_networks


@app.post("/connect", summary="Connect to wifi network.")
@version(1, 0)
async def connect(credentials: WifiCredentials, hidden: bool = False, interface: Optional[str] = None) -> Any:
    manager = get_manager_for_interface(interface)
    await manager.try_connect_to_network(credentials, hidden)


@app.post("/remove", summary="Remove saved wifi network.")
@version(1, 0)
async def remove(ssid: str, interface: Optional[str] = None) -> Any:
    manager = get_manager_for_interface(interface)
    logger.info(f"Processing remove request for SSID: {ssid} on interface {interface or 'default'}")
    try:
        await manager.remove_network(ssid)
    except StopIteration as error:
        logger.info(f"Network '{ssid}' is unknown.")
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail=f"Network '{ssid}' not saved.") from error
    logger.info(f"Successfully removed '{ssid}'.")


@app.get("/disconnect", summary="Disconnect from wifi network.")
@version(1, 0)
async def disconnect(interface: Optional[str] = None) -> Any:
    manager = get_manager_for_interface(interface)
    await manager.disconnect()
    logger.info("Successfully disconnected from network.")


@app.get("/hotspot", summary="Get hotspot state.")
@version(1, 0)
async def hotspot_state(interface: Optional[str] = None) -> Any:
    manager = get_manager_for_interface(interface)
    return await manager.hotspot_is_running()


@app.get("/hotspot_extended_status", summary="Get extended hotspot status.")
@version(1, 0)
async def hotspot_extended_state(interface: Optional[str] = None) -> HotspotStatus:
    manager = get_manager_for_interface(interface)
    return HotspotStatus(supported=await manager.supports_hotspot(), enabled=await manager.hotspot_is_running())


@app.post("/hotspot", summary="Enable/disable hotspot.")
@version(1, 0)
async def toggle_hotspot(enable: bool, interface: Optional[str] = None) -> Any:
    manager = get_manager_for_interface(interface)
    if enable:
        return await manager.enable_hotspot()
    return await manager.disable_hotspot()


@app.post("/smart_hotspot", summary="Enable/disable smart-hotspot.")
@version(1, 0)
def toggle_smart_hotspot(enable: bool, interface: Optional[str] = None) -> Any:
    manager = get_manager_for_interface(interface)
    if enable:
        manager.enable_smart_hotspot()
        return
    manager.disable_smart_hotspot()


@app.get("/smart_hotspot", summary="Check if smart-hotspot is enabled.")
@version(1, 0)
def check_smart_hotspot(interface: Optional[str] = None) -> Any:
    manager = get_manager_for_interface(interface)
    return manager.is_smart_hotspot_enabled()


@app.post("/hotspot_credentials", summary="Update hotspot credentials.")
@version(1, 0)
async def set_hotspot_credentials(credentials: WifiCredentials, interface: Optional[str] = None) -> Any:
    manager = get_manager_for_interface(interface)
    await manager.set_hotspot_credentials(credentials)


@app.get("/hotspot_credentials", summary="Get hotspot credentials.")
@version(1, 0)
def get_hotspot_credentials(interface: Optional[str] = None) -> Any:
    manager = get_manager_for_interface(interface)
    return manager.hotspot_credentials()


@app.get("/interfaces", response_model=List[WlanInterface], summary="Get available WLAN interfaces.")
@version(1, 0)
def get_interfaces() -> List[WlanInterface]:
    """Get list of all available WLAN interfaces and their status."""
    interfaces = []
    primary_interface = wifi_manager.interface_name if wifi_manager else None
    for iface_name in wifi_managers:
        interfaces.append(WlanInterface(name=iface_name, is_active=iface_name == primary_interface))
    return interfaces


@app.get("/interface", summary="Get current WLAN interface name.")
@version(1, 0)
def get_current_interface() -> str:
    if wifi_manager is None:
        raise HTTPException(status_code=status.HTTP_503_SERVICE_UNAVAILABLE, detail="WiFi manager not initialized")
    return wifi_manager.interface_name


app = VersionedFastAPI(app, version="1.0.0", prefix_format="/v{major}.{minor}", enable_latest=True)
app.mount("/", StaticFiles(directory=str(FRONTEND_FOLDER), html=True))


def _is_socket(file_path: str) -> bool:
    """Check if a path is a Unix socket."""
    try:
        mode = os.stat(file_path).st_mode
        return stat.S_ISSOCK(mode)
    except Exception:
        return False


async def initialize_wpa_supplicant_managers(args: argparse.Namespace) -> bool:
    """Initialize WifiManager instances for all available wpa_supplicant interfaces."""
    global wifi_manager  # pylint: disable=global-statement

    wpa_socket_folder = "/var/run/wpa_supplicant/"

    try:
        entries = os.scandir(wpa_socket_folder)
        available_sockets = sorted(
            [
                entry.name
                for entry in entries
                if entry.name.startswith(("wlan", "wifi", "wlp"))
                and not entry.name.startswith("uap")  # Exclude virtual AP interfaces
                and _is_socket(entry.path)
            ]
        )
    except Exception as error:
        logger.warning(f"Could not scan wpa_supplicant folder: {error}")
        return False

    if not available_sockets:
        logger.info("No wpa_supplicant sockets found.")
        return False

    for socket_name in available_sockets:
        try:
            manager = WifiManager()
            manager.configure(args)
            # Override the socket name to connect to specific interface
            manager.args.socket_name = socket_name
            await manager.start()
            wifi_managers[socket_name] = manager
            logger.info(f"Initialized WifiManager for interface: {socket_name}")
            # Set the first one as the primary manager for backwards compatibility
            if wifi_manager is None:
                wifi_manager = manager
        except Exception as error:
            logger.warning(f"Could not initialize WifiManager for {socket_name}: {error}")

    return len(wifi_managers) > 0


async def initialize_network_manager(args: argparse.Namespace) -> bool:
    """Initialize NetworkManager-based wifi manager."""
    global wifi_manager  # pylint: disable=global-statement

    manager = NetworkManagerWifi()
    manager.configure(args)

    if not await manager.can_work():
        return False

    await manager.start()
    interface_name = manager.interface_name
    wifi_managers[interface_name] = manager
    wifi_manager = manager
    logger.info(f"Initialized NetworkManagerWifi for interface: {interface_name}")
    return True


async def main() -> None:
    await init_sentry_async(SERVICE_NAME)

    parser = argparse.ArgumentParser(description="Abstraction CLI for WifiManager configuration.")

    # Add arguments from both implementations
    wpa_temp = WifiManager()
    nm_temp = NetworkManagerWifi()
    wpa_temp.add_arguments(parser)
    nm_temp.add_arguments(parser)
    args = parser.parse_args()

    # Running uvicorn with log disabled so loguru can handle it
    config = Config(app=app, host="0.0.0.0", port=9000, log_config=None)
    server = Server(config)

    # Try wpa_supplicant first (Bullseye), then NetworkManager (Bookworm)
    wpa_temp.configure(args)
    if await wpa_temp.can_work():
        logger.info("Using wpa_supplicant backend.")
        await initialize_wpa_supplicant_managers(args)
    else:
        logger.info("Trying NetworkManager backend.")
        await initialize_network_manager(args)

    if not wifi_managers:
        logger.warning("No wifi managers initialized. WiFi functionality will be unavailable.")

    await server.serve()


if __name__ == "__main__":
    asyncio.run(main())
