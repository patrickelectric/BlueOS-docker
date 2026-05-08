#! /usr/bin/env python3

import asyncio
import logging

from api.v1.routers.models import models_router
from api.v1.routers.theme import theme_router
from commonwealth.utils.apis import GenericErrorHandlingRoute, PrettyJSONResponse
from commonwealth.utils.logs import InterceptHandler, init_logger
from commonwealth.utils.sentry_config import init_sentry_async
from fastapi import FastAPI
from fastapi_versioning import VersionedFastAPI
from loguru import logger
from storage import ensure_dirs
from uvicorn import Config, Server

SERVICE_NAME = "customization"
PORT = 9152

logging.basicConfig(handlers=[InterceptHandler()], level=logging.DEBUG)
init_logger(SERVICE_NAME)
logger.info("Starting Customization service")

ensure_dirs()

fast_api_app = FastAPI(
    title="Customization API",
    description="Manage BlueOS appearance overrides (theme color, custom 3D models).",
    default_response_class=PrettyJSONResponse,
)
fast_api_app.router.route_class = GenericErrorHandlingRoute
fast_api_app.include_router(theme_router)
fast_api_app.include_router(models_router)

app = VersionedFastAPI(
    fast_api_app,
    version="1.0.0",
    prefix_format="/v{major}.{minor}",
    enable_latest=True,
)


@app.get("/")
async def root() -> dict[str, str]:
    return {"service": SERVICE_NAME}


async def main() -> None:
    try:
        await init_sentry_async(SERVICE_NAME)
        config = Config(app=app, host="0.0.0.0", port=PORT, log_config=None)
        server = Server(config)
        await server.serve()
    finally:
        logger.info("Customization service stopped")


if __name__ == "__main__":
    asyncio.run(main())
