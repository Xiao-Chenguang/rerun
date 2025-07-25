# DO NOT EDIT! This file was auto-generated by crates/build/re_types_builder/src/codegen/python/mod.rs
# Based on "crates/store/re_types/definitions/rerun/blueprint/components/link_axis.fbs".

# You can extend this class by creating a "LinkAxisExt" class in "link_axis_ext.py".

from __future__ import annotations

from collections.abc import Sequence
from typing import Literal, Union

import pyarrow as pa

from ..._baseclasses import (
    BaseBatch,
    ComponentBatchMixin,
)

__all__ = ["LinkAxis", "LinkAxisArrayLike", "LinkAxisBatch", "LinkAxisLike"]


from enum import Enum


class LinkAxis(Enum):
    """**Component**: How should the horizontal/X/time axis be linked across multiple plots."""

    Independent = 1
    """The axis is independent from all other plots."""

    LinkToGlobal = 2
    """Link to all other plots that also have this options set."""

    @classmethod
    def auto(cls, val: str | int | LinkAxis) -> LinkAxis:
        """Best-effort converter, including a case-insensitive string matcher."""
        if isinstance(val, LinkAxis):
            return val
        if isinstance(val, int):
            return cls(val)
        try:
            return cls[val]
        except KeyError:
            val_lower = val.lower()
            for variant in cls:
                if variant.name.lower() == val_lower:
                    return variant
        raise ValueError(f"Cannot convert {val} to {cls.__name__}")

    def __str__(self) -> str:
        """Returns the variant name."""
        return self.name


LinkAxisLike = Union[LinkAxis, Literal["Independent", "LinkToGlobal", "independent", "linktoglobal"], int]
LinkAxisArrayLike = Union[LinkAxisLike, Sequence[LinkAxisLike]]


class LinkAxisBatch(BaseBatch[LinkAxisArrayLike], ComponentBatchMixin):
    _ARROW_DATATYPE = pa.uint8()
    _COMPONENT_TYPE: str = "rerun.blueprint.components.LinkAxis"

    @staticmethod
    def _native_to_pa_array(data: LinkAxisArrayLike, data_type: pa.DataType) -> pa.Array:
        if isinstance(data, (LinkAxis, int, str)):
            data = [data]

        pa_data = [LinkAxis.auto(v).value if v is not None else None for v in data]  # type: ignore[redundant-expr]

        return pa.array(pa_data, type=data_type)
