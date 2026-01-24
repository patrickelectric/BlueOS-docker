#! /usr/bin/env python3

import asyncio
import contextlib
import json
import logging
import shutil
import tempfile
import uuid
from functools import wraps
from io import BytesIO
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional
from urllib.parse import quote

from aiocache import cached
from commonwealth.utils.apis import GenericErrorHandlingRoute, PrettyJSONResponse
from commonwealth.utils.general import file_is_open_async
from commonwealth.utils.logs import InterceptHandler, init_logger
from commonwealth.utils.sentry_config import init_sentry_async
from fastapi import APIRouter, FastAPI, HTTPException, status
from fastapi.responses import FileResponse, HTMLResponse, StreamingResponse
from fastapi_versioning import VersionedFastAPI, versioned_api_route
from loguru import logger
from pydantic import BaseModel
from uvicorn import Config, Server

SERVICE_NAME = "recorder-extractor"
RECORDER_DIR = Path("/usr/blueos/userdata/recorder")
ARDUPILOT_LOG_DIRS = [Path("/shortcuts/ardupilot_logs/firmware/logs"), Path("/shortcuts/ardupilot_logs/logs")]
MISSIONS_FILE = Path("/usr/blueos/userdata/recorder/missions.json")
PORT = 9150

TIMESTAMP_TOLERANCE_SECONDS = 60

# Prevent thumbnails from being generated while MCAP extraction is running
thumbnail_lock = asyncio.Lock()

# Track MCAP files currently being processed
processing_mcap_files: set[str] = set()

logging.basicConfig(handlers=[InterceptHandler()], level=logging.DEBUG)
init_logger(SERVICE_NAME)
logger.info("Starting Recorder Extractor service")


class MissionFile(BaseModel):
    name: str
    path: str
    size_bytes: int
    modified: float
    type: str
    download_url: str
    thumbnail_url: Optional[str] = None
    stream_url: Optional[str] = None


class Mission(BaseModel):
    id: str
    name: str
    date: float
    duration_seconds: Optional[float] = None
    files: List[MissionFile]
    thumbnails: List[str]
    is_complete: bool
    is_processing: bool


class MissionsResponse(BaseModel):
    missions: List[Mission]
    orphaned_files: List[MissionFile]


class LinkFilesRequest(BaseModel):
    mission_id: str
    file_paths: List[str]


class CreateMissionRequest(BaseModel):
    name: Optional[str] = None
    file_paths: List[str]


class RecordingFile(BaseModel):
    name: str
    path: str
    size_bytes: int
    modified: float
    download_url: str
    stream_url: str
    thumbnail_url: str


class ProcessingFile(BaseModel):
    name: str
    path: str


class ProcessingStatus(BaseModel):
    processing: List[ProcessingFile]


class MissionsStore:
    def __init__(self, path: Path):
        self.path = path
        self.missions: Dict[str, Dict[str, Any]] = {}
        self.load()

    def load(self) -> None:
        if self.path.exists():
            try:
                with open(self.path, "r", encoding="utf-8") as f:
                    data = json.load(f)
                    self.missions = data.get("missions", {})
            except Exception as e:
                logger.error(f"Failed to load missions store: {e}")
                self.missions = {}
        else:
            self.missions = {}

    def save(self) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        try:
            with open(self.path, "w", encoding="utf-8") as f:
                json.dump({"missions": self.missions}, f, indent=2)
        except Exception as e:
            logger.error(f"Failed to save missions store: {e}")

    def get_mission(self, mission_id: str) -> Optional[Dict[str, Any]]:
        return self.missions.get(mission_id)

    def create_mission(self, name: str, file_paths: List[str], date: float) -> str:
        mission_id = str(uuid.uuid4())[:8]
        self.missions[mission_id] = {
            "id": mission_id,
            "name": name,
            "date": date,
            "file_paths": file_paths,
        }
        self.save()
        return mission_id

    def add_files_to_mission(self, mission_id: str, file_paths: List[str]) -> bool:
        if mission_id not in self.missions:
            return False
        existing = set(self.missions[mission_id].get("file_paths", []))
        existing.update(file_paths)
        self.missions[mission_id]["file_paths"] = list(existing)
        self.save()
        return True

    def remove_files_from_mission(self, mission_id: str, file_paths: List[str]) -> bool:
        if mission_id not in self.missions:
            return False
        existing = set(self.missions[mission_id].get("file_paths", []))
        existing -= set(file_paths)
        self.missions[mission_id]["file_paths"] = list(existing)
        self.save()
        return True

    def delete_mission(self, mission_id: str) -> bool:
        if mission_id in self.missions:
            del self.missions[mission_id]
            self.save()
            return True
        return False

    def rename_mission(self, mission_id: str, name: str) -> bool:
        if mission_id not in self.missions:
            return False
        self.missions[mission_id]["name"] = name
        self.save()
        return True

    def get_all_assigned_paths(self) -> set[str]:
        paths: set[str] = set()
        for mission in self.missions.values():
            paths.update(mission.get("file_paths", []))
        return paths


missions_store = MissionsStore(MISSIONS_FILE)


def ensure_recorder_dir() -> Path:
    RECORDER_DIR.mkdir(parents=True, exist_ok=True)
    return RECORDER_DIR.resolve()


def resolve_recording(filename: str) -> Path:
    base = ensure_recorder_dir()
    candidate = (base / filename).resolve()
    try:
        candidate.relative_to(base)
    except ValueError as exc:
        logger.warning(f"Path resolve attempt: base={base} candidate={candidate} raw={filename}")
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="Invalid recording path.") from exc

    if candidate.is_dir():
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="Invalid recording path.")
    if candidate.suffix.lower() != ".mp4":
        logger.warning(f"Rejected non-mp4 path: {candidate}")
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="Only .mp4 recordings are supported.")
    if not candidate.exists() or not candidate.is_file():
        logger.warning(f"Recording not found: {candidate}")
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Recording not found.")
    return candidate


def parse_duration_ns(discover_output: str) -> int:
    duration_ns = 0
    for line in discover_output.splitlines():
        # Example: Duration: 0:00:12.345678000
        if "Duration:" not in line:
            continue
        try:
            parts = line.split("Duration:", maxsplit=1)[1].strip().split(".")
            hms = parts[0]
            nanos = parts[1] if len(parts) > 1 else "0"
            hours, minutes, seconds = [int(x) for x in hms.split(":")]
            duration_ns = ((hours * 3600) + (minutes * 60) + seconds) * 1_000_000_000 + int(nanos)
            break
        except Exception as exception:
            logger.error(f"Failed to parse duration: {exception}")
            break
    return duration_ns


async def check_and_recover_mcap(mcap_path: Path) -> None:
    # Check if mcap binary exists
    mcap_binary = shutil.which("mcap")
    if not mcap_binary:
        logger.warning("mcap binary not found, skipping doctor/recover check")
        return

    # Ensure path exists and is a file
    if not mcap_path.exists() or not mcap_path.is_file():
        logger.debug(f"MCAP file not found or not a file: {mcap_path}")
        return

    logger.info(f"Running mcap doctor on {mcap_path}")
    # Run mcap doctor
    doctor_cmd = [mcap_binary, "doctor", str(mcap_path)]
    doctor_proc = await asyncio.create_subprocess_exec(
        *doctor_cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        text=False,
    )
    stdout_bytes, stderr_bytes = await doctor_proc.communicate()
    stdout = stdout_bytes.decode("utf-8", "ignore")
    stderr = stderr_bytes.decode("utf-8", "ignore")

    if doctor_proc.returncode == 0:
        logger.info(f"mcap doctor passed for {mcap_path}: {stdout.strip()}")
        return

    logger.warning(f"mcap doctor failed for {mcap_path} (code={doctor_proc.returncode}): {stderr.strip()}")
    logger.info(f"Attempting to recover {mcap_path}")

    # Create a temporary file path in the same directory as the mcap file
    # This ensures atomic replacement on the same filesystem
    tmp_path = None
    try:
        with tempfile.NamedTemporaryFile(delete=False, dir=mcap_path.parent, suffix=".recover") as tmpfile:
            tmp_path = Path(tmpfile.name)

        recover_cmd = [mcap_binary, "recover", str(mcap_path), "-o", str(tmp_path)]
        recover_proc = await asyncio.create_subprocess_exec(
            *recover_cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            text=False,
        )
        _, recover_stderr_bytes = await recover_proc.communicate()
        recover_stderr = recover_stderr_bytes.decode("utf-8", "ignore")

        # Check if recovery succeeded
        if recover_proc.returncode != 0:
            logger.error(
                f"mcap recover command failed for {mcap_path} (code={recover_proc.returncode}): {recover_stderr.strip()}",
            )
            return

        if not tmp_path.exists():
            logger.error(f"mcap recover did not create output file: {tmp_path}")
            return

        if tmp_path.stat().st_size == 0:
            logger.error(f"mcap recover produced empty file: {tmp_path}")
            return

        # Atomically replace the original file with the recovered one
        # Using replace ensures atomic operation
        tmp_path.replace(mcap_path)
        logger.info(f"Successfully recovered {mcap_path} (recovered size: {mcap_path.stat().st_size} bytes)")
        tmp_path = None  # Mark as successfully moved to prevent cleanup
    except OSError as exception:
        logger.error(f"Failed to replace original file after mcap recover: {exception}")
    except Exception as exception:
        logger.exception(f"Unexpected error during mcap recover: {exception}")
    finally:
        # Clean up temporary file if it still exists
        if tmp_path is not None and tmp_path.exists():
            try:
                tmp_path.unlink()
            except OSError as exception:
                logger.error(f"Failed to clean up temporary file {tmp_path}: {exception}")


@cached()
async def build_thumbnail_bytes(path: Path) -> bytes:
    # 1) Discover duration (nanoseconds) using gst-discoverer
    discover_cmd = ["gst-discoverer-1.0", f"file://{path}"]
    discover_proc = await asyncio.create_subprocess_exec(
        *discover_cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        text=False,
    )
    stdout_bytes, stderr_bytes = await discover_proc.communicate()
    stdout = stdout_bytes.decode("utf-8", "ignore")
    stderr = stderr_bytes.decode("utf-8", "ignore")
    if discover_proc.returncode != 0:
        logger.error(f"gst-discoverer-1.0 failed for {path}: {stderr.strip()}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to inspect recording.",
        )

    duration_ns = parse_duration_ns(stdout)
    target_ns = duration_ns // 2 if duration_ns > 0 else 0
    target_sec = target_ns / 1_000_000_000

    # 2) Grab a frame at the target time using gst-play-1.0 + raw pipeline sink
    pipeline = (
        "videoconvert ! videoscale ! "
        "video/x-raw,width=320,height=180 ! "
        "jpegenc snapshot=true quality=85 ! "
        "fdsink fd=1 sync=false"
    )

    play_cmd = [
        "gst-play-1.0",
        f"--start-position={target_sec:.3f}",
        f"--videosink={pipeline}",
        "--audiosink=fakesink",
        "--no-interactive",
        "-q",
        f"file://{path}",
    ]
    logger.info(
        f"Thumbnail target: duration_ns={duration_ns} target_ns={target_ns} target_sec={target_sec:.3f} file={path}"
    )
    logger.info(f"Thumbnail command: {' '.join(play_cmd)}")
    play_proc = await asyncio.create_subprocess_exec(
        *play_cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        text=False,
    )
    stdout_bytes, stderr_bytes = await play_proc.communicate()
    stderr = stderr_bytes.decode("utf-8", "ignore")
    if play_proc.returncode != 0 or not stdout:
        logger.error(f"gst-play-1.0 failed for {path} (code={play_proc.returncode}): {stderr}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to generate thumbnail.",
        )

    return stdout_bytes


async def extract_mcap_recordings() -> None:
    while True:
        await asyncio.sleep(10)
        try:
            base = ensure_recorder_dir()
            for mcap_path in base.rglob("*.mcap"):
                # If the folder already exists, it's already extracted or deleted by user
                output_dir = mcap_path.with_suffix("")
                if output_dir.exists():
                    continue

                logger.info(f"Checking if file is in use: {mcap_path}")
                if await file_is_open_async(mcap_path):
                    logger.info(f"Skipping MCAP extract, file in use: {mcap_path}")
                    continue

                # Check and recover MCAP file if mcap binary is available
                await check_and_recover_mcap(mcap_path)

                command = [
                    "mcap-foxglove-video-extract",
                    str(mcap_path),
                    "all",
                    "--output",
                    str(output_dir),
                ]
                logger.info(f"Extracting MCAP video to {output_dir} with command: {' '.join(command)}")
                mcap_relative = str(mcap_path.relative_to(base))
                processing_mcap_files.add(mcap_relative)
                try:
                    async with thumbnail_lock:
                        process = await asyncio.create_subprocess_exec(
                            *command,
                            stdout=asyncio.subprocess.PIPE,
                            stderr=asyncio.subprocess.PIPE,
                            text=False,
                        )
                        stdout_bytes, stderr_bytes = await process.communicate()
                        stdout = stdout_bytes.decode("utf-8", "ignore")
                        stderr = stderr_bytes.decode("utf-8", "ignore")
                finally:
                    processing_mcap_files.discard(mcap_relative)
                if process.returncode != 0:
                    logger.error(
                        f"MCAP extract failed for {mcap_path} (code={process.returncode}): {stderr}",
                    )
                else:
                    logger.info(f"MCAP extract completed for {mcap_path}: {stdout.strip()}")
        except Exception as exception:
            logger.exception(f"MCAP extraction loop failed: {exception}")


def to_http_exception(endpoint: Callable[..., Any]) -> Callable[..., Any]:
    is_async = asyncio.iscoroutinefunction(endpoint)

    @wraps(endpoint)
    async def wrapper(*args: Any, **kwargs: Any) -> Any:
        try:
            if is_async:
                return await endpoint(*args, **kwargs)
            return endpoint(*args, **kwargs)
        except HTTPException as exception:
            raise exception
        except Exception as exception:
            logger.exception("Recorder endpoint failed")
            raise HTTPException(status_code=status.HTTP_500_INTERNAL_SERVER_ERROR, detail=str(exception)) from exception

    return wrapper


def scan_all_files() -> List[MissionFile]:
    files: List[MissionFile] = []
    recorder_base = ensure_recorder_dir()
    recorder_base_url = "/recorder-extractor/v1.0/recorder/files"
    filebrowser_base_url = "/file-browser/api/raw"

    for mcap_path in recorder_base.rglob("*.mcap"):
        stat = mcap_path.stat()
        relative_path = str(mcap_path.relative_to(recorder_base))
        files.append(
            MissionFile(
                name=mcap_path.name,
                path=str(mcap_path),
                size_bytes=stat.st_size,
                modified=stat.st_mtime,
                type="mcap",
                download_url=f"{filebrowser_base_url}{mcap_path}",
            )
        )

    for mp4_path in recorder_base.rglob("*.mp4"):
        stat = mp4_path.stat()
        relative_path = str(mp4_path.relative_to(recorder_base))
        encoded_path = quote(relative_path, safe="")
        files.append(
            MissionFile(
                name=mp4_path.name,
                path=str(mp4_path),
                size_bytes=stat.st_size,
                modified=stat.st_mtime,
                type="mp4",
                download_url=f"{recorder_base_url}/{encoded_path}",
                stream_url=f"{recorder_base_url}/{encoded_path}",
                thumbnail_url=f"{recorder_base_url}/{encoded_path}/thumbnail",
            )
        )

    for log_dir in ARDUPILOT_LOG_DIRS:
        if not log_dir.exists():
            continue
        for log_path in log_dir.rglob("*"):
            if not log_path.is_file():
                continue
            ext = log_path.suffix.lower()
            if ext not in [".bin", ".tlog"]:
                continue
            stat = log_path.stat()
            if stat.st_size < 100:
                continue
            file_type = "bin" if ext == ".bin" else "tlog"
            files.append(
                MissionFile(
                    name=log_path.name,
                    path=str(log_path),
                    size_bytes=stat.st_size,
                    modified=stat.st_mtime,
                    type=file_type,
                    download_url=f"{filebrowser_base_url}{log_path}",
                )
            )

    return files


def _is_file_processing(file: MissionFile) -> bool:
    return file.type == "mcap" and file.path in [str(RECORDER_DIR / p) for p in processing_mcap_files]


def _build_mission_from_files(
    mission_id: str, name: str, mission_files: List[MissionFile], stored_date: Optional[float] = None
) -> Mission:
    thumbnails = [f.thumbnail_url for f in mission_files if f.thumbnail_url]
    is_processing = any(_is_file_processing(f) for f in mission_files)
    has_video = any(f.type == "mp4" for f in mission_files)
    has_log = any(f.type in ["bin", "tlog"] for f in mission_files)

    min_date = min(f.modified for f in mission_files)
    max_date = max(f.modified for f in mission_files)
    duration = max_date - min_date if len(mission_files) > 1 and max_date > min_date else None

    return Mission(
        id=mission_id,
        name=name,
        date=stored_date if stored_date else min_date,
        duration_seconds=duration,
        files=sorted(mission_files, key=lambda x: x.modified),
        thumbnails=thumbnails[:4],
        is_complete=has_video and has_log,
        is_processing=is_processing,
    )


def _group_unassigned_files(unassigned: List[MissionFile]) -> List[List[MissionFile]]:
    sorted_files = sorted(unassigned, key=lambda x: x.modified)
    groups: List[List[MissionFile]] = []

    for file in sorted_files:
        placed = False
        for group in groups:
            if any(abs(file.modified - existing.modified) <= TIMESTAMP_TOLERANCE_SECONDS for existing in group):
                group.append(file)
                placed = True
                break
        if not placed:
            groups.append([file])

    return groups


def group_files_into_missions(files: List[MissionFile]) -> MissionsResponse:
    files_by_path = {f.path: f for f in files}
    missions: List[Mission] = []
    used_paths: set[str] = set()

    for mission_id, mission_data in missions_store.missions.items():
        mission_files = [files_by_path[p] for p in mission_data.get("file_paths", []) if p in files_by_path]
        if not mission_files:
            continue
        for f in mission_files:
            used_paths.add(f.path)
        name = mission_data.get("name", f"Mission {mission_id}")
        missions.append(_build_mission_from_files(mission_id, name, mission_files, mission_data.get("date")))

    unassigned = [f for f in files if f.path not in used_paths]
    auto_groups = _group_unassigned_files(unassigned)

    orphaned: List[MissionFile] = []
    for group in auto_groups:
        if len(group) == 1:
            orphaned.append(group[0])
            continue
        min_date = min(f.modified for f in group)
        auto_id = f"auto-{int(min_date)}"
        missions.append(_build_mission_from_files(auto_id, f"Mission {len(missions) + 1}", group))

    missions.sort(key=lambda x: x.date, reverse=True)
    return MissionsResponse(missions=missions, orphaned_files=orphaned)


recorder_router = APIRouter(
    prefix="/recorder",
    tags=["recorder_v1"],
    route_class=versioned_api_route(1, 0),
    responses={status.HTTP_404_NOT_FOUND: {"description": "Not found"}},
)


missions_router = APIRouter(
    prefix="/missions",
    tags=["missions_v1"],
    route_class=versioned_api_route(1, 0),
    responses={status.HTTP_404_NOT_FOUND: {"description": "Not found"}},
)


@missions_router.get(
    "",
    response_model=MissionsResponse,
    summary="List all missions with their associated files.",
)
@to_http_exception
async def list_missions() -> MissionsResponse:
    files = scan_all_files()
    return group_files_into_missions(files)


@missions_router.post(
    "",
    response_model=Mission,
    summary="Create a new mission from orphaned files.",
)
@to_http_exception
async def create_mission(request: CreateMissionRequest) -> Mission:
    if not request.file_paths:
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="No files provided.")

    files = scan_all_files()
    files_by_path = {f.path: f for f in files}

    mission_files = [files_by_path[p] for p in request.file_paths if p in files_by_path]
    if not mission_files:
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="No valid files found.")

    min_date = min(f.modified for f in mission_files)
    name = request.name or f"Mission {len(missions_store.missions) + 1}"
    mission_id = missions_store.create_mission(name, request.file_paths, min_date)

    thumbnails = [f.thumbnail_url for f in mission_files if f.thumbnail_url]
    has_video = any(f.type == "mp4" for f in mission_files)
    has_log = any(f.type in ["bin", "tlog"] for f in mission_files)

    return Mission(
        id=mission_id,
        name=name,
        date=min_date,
        duration_seconds=None,
        files=mission_files,
        thumbnails=thumbnails[:4],
        is_complete=has_video and has_log,
        is_processing=False,
    )


@missions_router.post(
    "/{mission_id}/files",
    summary="Link files to an existing mission.",
)
@to_http_exception
async def link_files_to_mission(mission_id: str, request: LinkFilesRequest) -> Dict[str, Any]:
    if mission_id.startswith("auto-"):
        files = scan_all_files()
        files_by_path = {f.path: f for f in files}
        existing_files = [files_by_path[p] for p in request.file_paths if p in files_by_path]
        if not existing_files:
            raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="No valid files found.")
        min_date = min(f.modified for f in existing_files)
        new_id = missions_store.create_mission("Mission", request.file_paths, min_date)
        return {"success": True, "mission_id": new_id}

    if not missions_store.add_files_to_mission(mission_id, request.file_paths):
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Mission not found.")
    return {"success": True}


@missions_router.delete(
    "/{mission_id}/files",
    summary="Unlink files from a mission.",
)
@to_http_exception
async def unlink_files_from_mission(mission_id: str, file_paths: List[str]) -> Dict[str, bool]:
    if not missions_store.remove_files_from_mission(mission_id, file_paths):
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Mission not found.")
    return {"success": True}


@missions_router.patch(
    "/{mission_id}",
    summary="Rename a mission.",
)
@to_http_exception
async def rename_mission(mission_id: str, name: str) -> Dict[str, bool]:
    if mission_id.startswith("auto-"):
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Cannot rename auto-grouped mission. Save it first.",
        )
    if not missions_store.rename_mission(mission_id, name):
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Mission not found.")
    return {"success": True}


@missions_router.delete(
    "/{mission_id}",
    summary="Delete a mission (metadata only, files are preserved).",
)
@to_http_exception
async def delete_mission_metadata(mission_id: str) -> Dict[str, bool]:
    if mission_id.startswith("auto-"):
        return {"success": True}
    if not missions_store.delete_mission(mission_id):
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Mission not found.")
    return {"success": True}


@missions_router.delete(
    "/{mission_id}/all",
    summary="Delete a mission and all its files.",
)
@to_http_exception
async def delete_mission_with_files(mission_id: str) -> Dict[str, bool]:
    files = scan_all_files()

    if mission_id.startswith("auto-"):
        response = group_files_into_missions(files)
        mission = next((m for m in response.missions if m.id == mission_id), None)
        if not mission:
            raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Mission not found.")
        for f in mission.files:
            try:
                Path(f.path).unlink()
            except Exception as e:
                logger.error(f"Failed to delete file {f.path}: {e}")
        return {"success": True}

    mission_data = missions_store.get_mission(mission_id)
    if not mission_data:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Mission not found.")

    for path in mission_data.get("file_paths", []):
        try:
            Path(path).unlink()
        except Exception as e:
            logger.error(f"Failed to delete file {path}: {e}")

    missions_store.delete_mission(mission_id)
    return {"success": True}


@recorder_router.get(
    "/files",
    response_model=List[RecordingFile],
    summary="List available MP4 recordings under /usr/blueos/userdata/recorder.",
)
@to_http_exception
async def list_recordings() -> List[RecordingFile]:
    base_url = "/recorder-extractor/v1.0/recorder/files"
    files: List[RecordingFile] = []
    base_path = ensure_recorder_dir()
    mp4_files = sorted(base_path.rglob("*.mp4"), key=lambda item: item.stat().st_mtime, reverse=True)
    for path in mp4_files:
        stat = path.stat()
        relative_path = path.relative_to(base_path)
        safe_path = str(relative_path)
        encoded_path = quote(safe_path, safe="")
        files.append(
            RecordingFile(
                name=path.name,
                path=safe_path,
                size_bytes=stat.st_size,
                modified=stat.st_mtime,
                download_url=f"{base_url}/{encoded_path}",
                stream_url=f"{base_url}/{encoded_path}",
                thumbnail_url=f"{base_url}/{encoded_path}/thumbnail",
            )
        )
    return files


@recorder_router.get(
    "/status",
    response_model=ProcessingStatus,
    summary="Get MCAP extraction processing status.",
)
@to_http_exception
async def get_processing_status() -> ProcessingStatus:
    # Snapshot the set with list to avoid RuntimeError from concurrent mutation
    processing = [ProcessingFile(name=Path(path).name, path=path) for path in list(processing_mcap_files)]
    return ProcessingStatus(processing=processing)


@recorder_router.get(
    "/files/{filename:path}/thumbnail",
    summary="Grab a thumbnail from a recording.",
)
@to_http_exception
async def get_recording_thumbnail(filename: str) -> StreamingResponse:
    path = resolve_recording(filename)
    async with thumbnail_lock:
        thumbnail_bytes = await build_thumbnail_bytes(path)
    return StreamingResponse(BytesIO(thumbnail_bytes), media_type="image/jpeg")


@recorder_router.delete(
    "/files/{filename:path}",
    summary="Delete a recording.",
    status_code=status.HTTP_204_NO_CONTENT,
)
@to_http_exception
async def delete_recording(filename: str) -> None:
    path = resolve_recording(filename)
    try:
        path.unlink()
    except Exception as exception:
        logger.exception(f"Failed to delete recording {filename}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Failed to delete recording.",
        ) from exception


@recorder_router.get(
    "/files/{filename:path}",
    summary="Download or stream a recording.",
)
@to_http_exception
async def get_recording(filename: str) -> FileResponse:
    path = resolve_recording(filename)
    return FileResponse(path, media_type="video/mp4", filename=path.name)


fast_api_app = FastAPI(
    title="Recorder Extractor API",
    description="Serve recorded MP4 files and manage missions.",
    default_response_class=PrettyJSONResponse,
)
fast_api_app.router.route_class = GenericErrorHandlingRoute
fast_api_app.include_router(recorder_router)
fast_api_app.include_router(missions_router)

app = VersionedFastAPI(
    fast_api_app,
    version="1.0.0",
    prefix_format="/v{major}.{minor}",
    enable_latest=True,
)


@app.get("/")
async def root() -> HTMLResponse:
    html_content = """
    <html>
        <head>
            <title>Recorder Extractor</title>
        </head>
    </html>
    """
    return HTMLResponse(content=html_content, status_code=200)


async def main() -> None:
    extractor_task = asyncio.create_task(extract_mcap_recordings())
    try:
        await init_sentry_async(SERVICE_NAME)

        config = Config(app=app, host="0.0.0.0", port=PORT, log_config=None)
        server = Server(config)

        await server.serve()
    finally:
        extractor_task.cancel()
        with contextlib.suppress(asyncio.CancelledError):
            await extractor_task


if __name__ == "__main__":
    asyncio.run(main())
