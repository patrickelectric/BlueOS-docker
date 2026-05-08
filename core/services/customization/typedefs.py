from typing import List, Optional

from pydantic import BaseModel, Field


class ThemePalette(BaseModel):
    light: str = Field(..., description="Lightest gradient anchor (replaces br_blue), as #RRGGBB.")
    mid: str = Field(..., description="Mid gradient anchor (replaces mariner_blue / primary), as #RRGGBB.")
    dark: str = Field(..., description="Darkest gradient anchor (replaces blue_whale), as #RRGGBB.")


class ThemeStatus(BaseModel):
    active: bool = Field(..., description="Whether a custom theme override is currently installed.")
    primary_color: Optional[str] = Field(None, description="User-selected primary color, as #RRGGBB.")
    palette: Optional[ThemePalette] = Field(None, description="Derived 3-anchor gradient palette.")


class ThemeRequest(BaseModel):
    primary_color: str = Field(..., description="Primary color (mid gradient anchor), as #RRGGBB or #RGB.")


class ModelEntry(BaseModel):
    name: str = Field(..., description="Display name for the model file.")
    path: str = Field(..., description="Path inside /userdata/modeloverrides/, e.g. 'sub/BLUEROV2.glb'.")
    url: str = Field(..., description="Public URL to fetch the model.")
    scope: str = Field(..., description="'global' for ALL.glb, 'vehicle' for per-vehicle/frame.")
    vehicle: Optional[str] = Field(None, description="Vehicle folder (sub, rover, ...).")
    frame: Optional[str] = Field(None, description="Frame name (without .glb).")
    size_bytes: int = Field(..., description="File size in bytes.")


class ModelsResponse(BaseModel):
    models: List[ModelEntry]


class DeleteResponse(BaseModel):
    deleted: str
