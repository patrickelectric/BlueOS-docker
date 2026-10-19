#!/usr/bin/env python3
"""Generate golden settings JSON fixtures using commonwealth Pydantic settings."""

from __future__ import annotations

import pathlib
from typing import Any, Dict

from commonwealth.settings.bases.pydantic_base import PydanticSettings
from pydantic import BaseModel

FIXTURES_DIR = pathlib.Path(__file__).resolve().parent


class GoldenAnimal(BaseModel):
    name: str = "bilica"
    animal_type: str = "dog"


class GoldenSettingsV1(PydanticSettings):
    first_variable: int = 100
    animal: GoldenAnimal = GoldenAnimal(name="pingu", animal_type="penguin")

    def migrate(self, data: Dict[str, Any]) -> None:
        if data["VERSION"] == GoldenSettingsV1.STATIC_VERSION:
            return
        if data["VERSION"] < GoldenSettingsV1.STATIC_VERSION:
            super().migrate(data)
        data["VERSION"] = GoldenSettingsV1.STATIC_VERSION
        data["first_variable"] = self.first_variable
        data["animal"] = self.animal.model_dump(mode="json")


class GoldenSettingsV2(PydanticSettings):
    first_variable: int = 66
    new_animal: GoldenAnimal = GoldenAnimal()

    def migrate(self, data: Dict[str, Any]) -> None:
        if data["VERSION"] == GoldenSettingsV2.STATIC_VERSION:
            return
        if data["VERSION"] < GoldenSettingsV2.STATIC_VERSION:
            GoldenSettingsV1().migrate(data)
        data["VERSION"] = GoldenSettingsV2.STATIC_VERSION
        data["first_variable"] = data.get("first_variable", self.first_variable)
        if "animal" in data:
            data["new_animal"] = data["animal"]
            data.pop("animal")


def main() -> None:
    v1_path = FIXTURES_DIR / "golden_settings_v1.json"
    GoldenSettingsV1().save(v1_path)

    migrated = GoldenSettingsV2()
    migrated.load(v1_path)
    migrated.save(FIXTURES_DIR / "golden_settings_v2_migrated_save.json")

    GoldenSettingsV2().save(FIXTURES_DIR / "golden_settings_v2_direct_save.json")

    future_path = FIXTURES_DIR / "golden_settings_future_v9.json"
    future_path.write_text(
        '{"VERSION": 9, "first_variable": 1, "new_animal": {"name": "x", "animal_type": "y"}}',
        encoding="utf-8",
    )

    print("golden fixtures generated")


if __name__ == "__main__":
    main()
