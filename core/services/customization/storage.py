from pathlib import Path

USERDATA_ROOT = Path("/usr/blueos/userdata")
STYLES_DIR = USERDATA_ROOT / "styles"
THEME_CSS = STYLES_DIR / "theme_style.css"
THEME_META = STYLES_DIR / "theme_style.meta.json"
MODELS_DIR = USERDATA_ROOT / "modeloverrides"

GLOBAL_MODEL_NAME = "ALL.glb"


def ensure_dirs() -> None:
    STYLES_DIR.mkdir(parents=True, exist_ok=True)
    MODELS_DIR.mkdir(parents=True, exist_ok=True)


def safe_model_path(relative: str) -> Path:
    """Resolve a path relative to MODELS_DIR while rejecting traversal and unsupported extensions."""
    candidate = (MODELS_DIR / relative).resolve()
    if MODELS_DIR.resolve() not in candidate.parents and candidate != MODELS_DIR.resolve():
        raise ValueError(f"Path escapes models directory: {relative!r}")
    if candidate.suffix.lower() != ".glb":
        raise ValueError("Only .glb files are supported.")
    return candidate
