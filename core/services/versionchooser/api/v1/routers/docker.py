from typing import Any

from fastapi import APIRouter, Request, status
from fastapi_versioning import versioned_api_route

from docker_login import (
    DockerLoginInfo,
    get_docker_accounts,
    make_docker_login,
    make_docker_logout,
)

docker_router_v1 = APIRouter(
    prefix="/docker",
    tags=["docker_v1"],
    route_class=versioned_api_route(1, 0),
    responses={status.HTTP_404_NOT_FOUND: {"description": "Not found"}},
)


@docker_router_v1.post("/login", summary="Login Docker daemon to a registry")
async def docker_login(request: Request) -> None:
    data = await request.json()
    info = DockerLoginInfo.from_json(data)

    return make_docker_login(info)


@docker_router_v1.post("/logout", summary="Logout Docker daemon from a registry")
async def docker_logout(request: Request) -> Any:
    data = await request.json()
    info = DockerLoginInfo.from_json(data)

    return make_docker_logout(info)


@docker_router_v1.post("/accounts", summary="Get the list of accounts logged in")
def docker_accounts() -> Any:
    return get_docker_accounts()
