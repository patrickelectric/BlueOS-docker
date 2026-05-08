import json

from colors import derive_palette, normalize_hex, render_theme_css
from fastapi import APIRouter, HTTPException, status
from fastapi.responses import PlainTextResponse
from fastapi_versioning import versioned_api_route
from loguru import logger
from storage import THEME_CSS, THEME_META, ensure_dirs
from typedefs import ThemePalette, ThemeRequest, ThemeStatus

theme_router = APIRouter(
    prefix="/theme",
    tags=["theme"],
    route_class=versioned_api_route(1, 0),
)


def _read_meta() -> ThemeStatus:
    if not THEME_CSS.exists():
        return ThemeStatus(active=False)
    try:
        meta = json.loads(THEME_META.read_text())
        primary = normalize_hex(meta["primary_color"])
        return ThemeStatus(active=True, primary_color=primary, palette=derive_palette(primary))
    except Exception as exc:
        # File exists but we can't introspect it; still report active.
        logger.warning(f"theme_style.css present but meta is unreadable: {exc}")
        return ThemeStatus(active=True)


@theme_router.get("", response_model=ThemeStatus, summary="Current theme override status.")
async def get_theme() -> ThemeStatus:
    return _read_meta()


@theme_router.post("", response_model=ThemeStatus, summary="Apply a custom theme color.")
async def set_theme(request: ThemeRequest) -> ThemeStatus:
    try:
        primary = normalize_hex(request.primary_color)
    except ValueError as exc:
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail=str(exc)) from exc

    derived = derive_palette(primary)
    ensure_dirs()
    THEME_CSS.write_text(render_theme_css(derived))
    THEME_META.write_text(json.dumps({"primary_color": primary}))
    logger.info(f"Theme override written for primary={primary}")
    return ThemeStatus(active=True, primary_color=primary, palette=derived)


@theme_router.delete("", response_model=ThemeStatus, summary="Remove the custom theme override.")
async def reset_theme() -> ThemeStatus:
    THEME_CSS.unlink(missing_ok=True)
    THEME_META.unlink(missing_ok=True)
    logger.info("Theme override removed")
    return ThemeStatus(active=False)


@theme_router.get(
    "/preview",
    response_class=PlainTextResponse,
    summary="Render the override CSS for a primary color without writing it to disk.",
)
async def preview_theme(primary_color: str) -> str:
    try:
        primary = normalize_hex(primary_color)
    except ValueError as exc:
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail=str(exc)) from exc
    css: str = render_theme_css(derive_palette(primary))
    return css


@theme_router.get(
    "/palette",
    response_model=ThemePalette,
    summary="Compute the derived 3-anchor palette for a primary color.",
)
async def palette(primary_color: str) -> ThemePalette:
    try:
        primary = normalize_hex(primary_color)
    except ValueError as exc:
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail=str(exc)) from exc
    return derive_palette(primary)
