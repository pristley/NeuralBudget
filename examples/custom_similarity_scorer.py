"""Example demonstrating registration and use of a similarity scorer."""
from neuralbudget.similarity import SimilarityRegistry
import neuralbudget


def main():
    reg = SimilarityRegistry(default_name="dummy_default")

    # Use the built-in DummyScorer (provided by the Rust lib) for examples
    dummy = neuralbudget.DummyScorer()
    reg.register("dummy", dummy)

    scorer = reg.get_scorer("dummy")
    score = scorer.score("The quick brown fox", "The quick brown fox")
    print("Exact match score:", score)

    print("Available scorers:", reg.list_scorers())


if __name__ == "__main__":
    main()
