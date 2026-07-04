use neuralbudget::similarity_traits::{ScorerRegistry, SimilarityScorer};
use neuralbudget::DummyScorer;
use std::sync::Arc;

#[test]
fn test_register_get_list_remove_set_default() -> Result<(), Box<dyn std::error::Error>> {
    let registry = ScorerRegistry::new("dummy_default");
    let scorer = Arc::new(DummyScorer::new());
    registry.register("dummy".to_string(), scorer.clone())?;

    // duplicate registration errors
    let res = registry.register("dummy".to_string(), scorer.clone());
    assert!(res.is_err());

    // get scorer
    let got = registry.get_scorer("dummy")?;
    assert_eq!(got.name(), "dummy");

    // list
    let list = registry.list_scorers();
    assert!(list.contains(&"dummy".to_string()));

    // set default
    registry.set_default("dummy")?;
    assert_eq!(registry.default_name(), "dummy".to_string());

    // remove
    registry.remove_scorer("dummy")?;
    let res = registry.get_scorer("dummy");
    assert!(res.is_err());

    Ok(())
}

#[test]
fn test_set_invalid_default() -> Result<(), Box<dyn std::error::Error>> {
    let registry = ScorerRegistry::new("x");
    let res = registry.set_default("nope");
    assert!(res.is_err());
    Ok(())
}
