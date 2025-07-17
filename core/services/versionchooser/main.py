#! /usr/bin/env python3
import asyncio
import logging

from commonwealth.utils.logs import InterceptHandler, init_logger
from commonwealth.utils.sentry_config import init_sentry_async
from loguru import logger
from uvicorn import Config, Server

from api import application
from args import CommandLineArgs

SERVICE_NAME = "version-chooser"

logging.basicConfig(handlers=[InterceptHandler()], level=0)
init_logger(SERVICE_NAME)

logger.info("Starting Version Chooser")


async def main() -> None:
    await init_sentry_async(SERVICE_NAME)

    args = CommandLineArgs.from_args()
    if args.debug:
        logging.getLogger(SERVICE_NAME).setLevel(logging.DEBUG)

    logger.info("Starting Version Chooser service.")

    config = Config(app=application, host="0.0.0.0", port=8081)
    server = Server(config)

    await server.serve()


if __name__ == "__main__":
    asyncio.run(main())
