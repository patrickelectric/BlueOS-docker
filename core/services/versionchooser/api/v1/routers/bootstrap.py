from typing import Any

import aiodocker
from fastapi import APIRouter, Request, status
from fastapi_versioning import versioned_api_route

from utils.chooser import VersionChooser

bootstrap_router_v1 = APIRouter(
    prefix="/bootstrap",
    tags=["bootstrap_v1"],
    route_class=versioned_api_route(1, 0),
    responses={status.HTTP_404_NOT_FOUND: {"description": "Not found"}},
)

versionChooser = VersionChooser(aiodocker.Docker())


@bootstrap_router_v1.get("/current", summary="Return the current running version of BlueOS-bootstrap")
async def get_bootstrap_version() -> Any:
    return await versionChooser.get_bootstrap_version()


@bootstrap_router_v1.post("/current", summary="Sets the current version of BlueOS-bootstrap to a new tag")
async def set_bootstrap_version(request: Request) -> Any:
    data = await request.json()
    tag = data["tag"]
    return await versionChooser.set_bootstrap_version(tag)
