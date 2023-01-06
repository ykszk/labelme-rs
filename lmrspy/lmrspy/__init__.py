from .lmrspy import Validator as _Validator
from typing import List, Union
from pathlib import Path


class Validator(_Validator):
    def __new__(cls, rules: List[str], flags: List[str], ignores: List[str]):
        return super().__new__(cls, rules, flags, ignores)

    def validate_json(self, filename: Union[str, Path]):
        return super().validate_json(filename)