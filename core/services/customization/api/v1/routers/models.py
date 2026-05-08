from pathlib import Path
from typing import List, Optional

from fastapi import APIRouter, File, HTTPException, Query, UploadFile, status
from fastapi_versioning import versioned_api_route
from loguru import logger
from storage import GLOBAL_MODEL_NAME, MODELS_DIR, ensure_dirs, safe_model_path
from typedefs import DeleteResponse, ModelEntry, ModelsResponse

models_router = APIRouter(
    prefix="/models",
    tags=["models"],
    route_class=versioned_api_route(1, 0),
)


def _entry_for(file: Path) -> ModelEntry:
    relative = file.relative_to(MODELS_DIR)
    parts = relative.parts
    if len(parts) == 1 and parts[0] == GLOBAL_MODEL_NAME:
        return ModelEntry(
            name=GLOBAL_MODEL_NAME,
            path=str(relative),
            url=f"/userdata/modeloverrides/{relative.as_posix()}",
            scope="global",
            size_bytes=file.stat().st_size,
        )
    vehicle = parts[0] if len(parts) > 1 else None
    frame = file.stem if vehicle is not None else None
    return ModelEntry(
        name=file.name,
        path=str(relative),
        url=f"/userdata/modeloverrides/{relative.as_posix()}",
        scope="vehicle",
        vehicle=vehicle,
        frame=frame,
        size_bytes=file.stat().st_size,
    )


@models_router.get("", response_model=ModelsResponse, summary="List custom 3D model overrides.")
async def list_models() -> ModelsResponse:
    ensure_dirs()
    entries: List[ModelEntry] = []
    for file in sorted(MODELS_DIR.rglob("*.glb")):
        if not file.is_file():
            continue
        try:
            entries.append(_entry_for(file))
        except Exception as exc:
            logger.warning(f"Skipping {file}: {exc}")
    return ModelsResponse(models=entries)


@models_router.post("", response_model=ModelEntry, summary="Upload a .glb model override.")
async def upload_model(
    file: UploadFile = File(..., description="GLB model file to upload."),
    scope: str = Query("global", regex="^(global|vehicle)$"),
    vehicle: Optional[str] = Query(None, description="Vehicle folder when scope=vehicle (e.g. 'sub', 'rover')."),
    frame: Optional[str] = Query(None, description="Frame name when scope=vehicle (e.g. 'BLUEROV2')."),
) -> ModelEntry:
    if scope == "global":
        relative = GLOBAL_MODEL_NAME
    else:
        if not vehicle or not frame:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="vehicle and frame query params are required when scope=vehicle.",
            )
        # Sanitize the components, no separators allowed.
        for value in (vehicle, frame):
            if "/" in value or ".." in value or "\\" in value:
                raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail=f"Invalid component: {value!r}")
        relative = f"{vehicle}/{frame}.glb"

    try:
        target = safe_model_path(relative)
    except ValueError as exc:
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail=str(exc)) from exc

    if not file.filename or not file.filename.lower().endswith(".glb"):
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="Only .glb files are accepted.")

    target.parent.mkdir(parents=True, exist_ok=True)
    contents = await file.read()
    target.write_bytes(contents)
    logger.info(f"Stored model override {target} ({len(contents)} bytes)")
    return _entry_for(target)


@models_router.delete("", response_model=DeleteResponse, summary="Delete a 3D model override by relative path.")
async def delete_model(
    path: str = Query(..., description="Path relative to /userdata/modeloverrides/.")
) -> DeleteResponse:
    try:
        target = safe_model_path(path)
    except ValueError as exc:
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail=str(exc)) from exc

    if not target.exists() or not target.is_file():
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Model not found.")

    target.unlink()
    logger.info(f"Deleted model override {target}")

    # Best-effort cleanup of empty vehicle subfolders.
    parent = target.parent
    if parent != MODELS_DIR and not any(parent.iterdir()):
        parent.rmdir()

    return DeleteResponse(deleted=path)
