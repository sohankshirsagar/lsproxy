# coding: utf-8

"""
    lsproxy

    No description provided (generated by Openapi Generator https://github.com/openapitools/openapi-generator)

    The version of the OpenAPI document: 0.1.0
    Generated by OpenAPI Generator (https://openapi-generator.tech)

    Do not edit the class manually.
"""  # noqa: E501


from __future__ import annotations
import pprint
import re  # noqa: F401
import json

from pydantic import BaseModel, ConfigDict, Field
from typing import Any, ClassVar, Dict, List, Optional
from lsproxy_sdk.models.symbol import Symbol
from typing import Optional, Set
from typing_extensions import Self

class SymbolResponse(BaseModel):
    """
    SymbolResponse
    """ # noqa: E501
    raw_response: Optional[Any] = Field(default=None, description="The raw response from the langserver.  https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_symbol https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#document_symbol")
    symbols: List[Symbol]
    __properties: ClassVar[List[str]] = ["raw_response", "symbols"]

    model_config = ConfigDict(
        populate_by_name=True,
        validate_assignment=True,
        protected_namespaces=(),
    )


    def to_str(self) -> str:
        """Returns the string representation of the model using alias"""
        return pprint.pformat(self.model_dump(by_alias=True))

    def to_json(self) -> str:
        """Returns the JSON representation of the model using alias"""
        # TODO: pydantic v2: use .model_dump_json(by_alias=True, exclude_unset=True) instead
        return json.dumps(self.to_dict())

    @classmethod
    def from_json(cls, json_str: str) -> Optional[Self]:
        """Create an instance of SymbolResponse from a JSON string"""
        return cls.from_dict(json.loads(json_str))

    def to_dict(self) -> Dict[str, Any]:
        """Return the dictionary representation of the model using alias.

        This has the following differences from calling pydantic's
        `self.model_dump(by_alias=True)`:

        * `None` is only added to the output dict for nullable fields that
          were set at model initialization. Other fields with value `None`
          are ignored.
        """
        excluded_fields: Set[str] = set([
        ])

        _dict = self.model_dump(
            by_alias=True,
            exclude=excluded_fields,
            exclude_none=True,
        )
        # override the default output from pydantic by calling `to_dict()` of each item in symbols (list)
        _items = []
        if self.symbols:
            for _item_symbols in self.symbols:
                if _item_symbols:
                    _items.append(_item_symbols.to_dict())
            _dict['symbols'] = _items
        # set to None if raw_response (nullable) is None
        # and model_fields_set contains the field
        if self.raw_response is None and "raw_response" in self.model_fields_set:
            _dict['raw_response'] = None

        return _dict

    @classmethod
    def from_dict(cls, obj: Optional[Dict[str, Any]]) -> Optional[Self]:
        """Create an instance of SymbolResponse from a dict"""
        if obj is None:
            return None

        if not isinstance(obj, dict):
            return cls.model_validate(obj)

        _obj = cls.model_validate({
            "raw_response": obj.get("raw_response"),
            "symbols": [Symbol.from_dict(_item) for _item in obj["symbols"]] if obj.get("symbols") is not None else None
        })
        return _obj


