"""
Convenience helpers for the similarity scorer registry and scorers.
This module wraps the low-level PyO3 objects to present a compact Python API.
"""
from typing import List
import neuralbudget


class SimilarityRegistry:
    """Thin wrapper around the Rust-backed `SimilarityRegistry`.

    Example:
        reg = SimilarityRegistry("dummy_default")
        reg.register("dummy", neuralbudget.DummyScorer())
        s = reg.get_scorer("dummy")
        print(s.score("a","b"))
    """

    def __init__(self, default_name: str = "default"):
        self._inner = neuralbudget.SimilarityRegistry(default_name)

    def register(self, name: str, scorer) -> None:
        return self._inner.register(name, scorer)

    def get_scorer(self, name: str):
        return self._inner.get_scorer(name)

    def list_scorers(self) -> List[str]:
        return list(self._inner.list_scorers())

    def set_default(self, name: str) -> None:
        return self._inner.set_default(name)

    def default_name(self) -> str:
        return str(self._inner.default_name())

    def remove_scorer(self, name: str) -> None:
        return self._inner.remove_scorer(name)
