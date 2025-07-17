from typing import Any

import aiodocker
from fastapi import APIRouter, Request, status
from fastapi_versioning import versioned_api_route

from utils.chooser import VersionChooser

version_router_v1 = APIRouter(
    prefix="/version",
    tags=["version_v1"],
    route_class=versioned_api_route(1, 0),
    responses={status.HTTP_404_NOT_FOUND: {"description": "Not found"}},
)

versionChooser = VersionChooser(aiodocker.Docker())


@version_router_v1.get(
    "/current", summary="Return the current running version of BlueOS", status_code=status.HTTP_200_OK
)
async def get_version() -> Any:
    return versionChooser.get_version()


@version_router_v1.post("/current", summary="Sets the current version of BlueOS to a new tag")
async def set_version(request: Request) -> Any:
    data = await request.json()
    tag = data["tag"]
    repository = data["repository"]
    return await versionChooser.set_version(repository, tag)


@version_router_v1.post("/pull", summary="Pulls a version from dockerhub")
async def pull_version(request: Request) -> Any:
    data = await request.json()
    repository = data["repository"]
    tag = data["tag"]
    return await versionChooser.pull_version(repository, tag)


@version_router_v1.delete("/delete", summary="Delete the selected version of BlueOS")
async def delete_version(request: Request) -> Any:
    data = await request.json()
    tag = data["tag"]
    repository = data["repository"]
    return await versionChooser.delete_version(repository, tag)


@version_router_v1.get("/available/local", summary="Returns available local versions")
async def get_available_local_versions() -> Any:
    return await versionChooser.get_available_local_versions()


@version_router_v1.get(
    "/available/{repository}/{image}", summary="Returns available versions, both locally and in dockerhub"
)
async def get_available_versions(repository: str, image: str) -> Any:
    return await versionChooser.get_available_versions(f"{repository}/{image}")


@version_router_v1.post("/load", summary="Load a docker tar file")
async def load(request: Request) -> Any:
    data = await request.read()
    return await versionChooser.load(data)


@version_router_v1.post("/restart", summary="Restart the currently running docker container")
async def restart() -> Any:
    return await versionChooser.restart()
