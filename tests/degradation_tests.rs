use neuralbudget::degradation::*;
use proptest::prelude::*;
use std::time::Instant;

#[test]
fn test_add_metric_simple() -> Result<(), Box<dyn std::error::Error>> {
    let mut d = DegradationDetector::new(50, 0.15, 2.5);
    for i in 0..10 {
        d.add_metric(i as f64)?;
    }
    let ev = d.detect_change()?;
    assert!(ev.is_some());
    assert_eq!(ev.unwrap().degradation_type, DegradationType::Stable);
    Ok(())
}

#[test]
fn test_gradual_drift_detection() -> Result<(), Box<dyn std::error::Error>> {
    let mut d = DegradationDetector::new(100, 0.15, 2.5);
    for i in 0..100 {
        d.add_metric((i as f64) * 0.01)?;
    }
    let ev = d.detect_change()?.expect("expected event");
    assert_eq!(ev.degradation_type, DegradationType::Gradual);
    assert!(ev.magnitude > 0.10);
    Ok(())
}

#[test]
fn test_sudden_spike_detection() -> Result<(), Box<dyn std::error::Error>> {
    let mut d = DegradationDetector::new(100, 0.15, 2.0);
    for _ in 0..50 {
        d.add_metric(50.0)?;
    }
    for _ in 0..3 {
        d.add_metric(150.0)?;
    }
    let ev = d.detect_change()?.expect("expected spike");
    assert_eq!(ev.degradation_type, DegradationType::Sudden);
    assert!(ev.magnitude > 0.5);
    Ok(())
}

#[test]
fn test_oscillation_detection() -> Result<(), Box<dyn std::error::Error>> {
    let mut d = DegradationDetector::new(120, 0.15, 2.5);
    for i in 0..120 {
        if i % 2 == 0 {
            d.add_metric(50.0)?;
        } else {
            d.add_metric(150.0)?;
        }
    }
    let ev = d.detect_change()?.expect("expected oscillation");
    assert_eq!(ev.degradation_type, DegradationType::Oscillating);
    Ok(())
}

#[test]
fn test_no_false_positives() -> Result<(), Box<dyn std::error::Error>> {
    use rand_distr::{Distribution, Normal};
    let mut d = DegradationDetector::new(200, 0.15, 2.5);
    let normal = Normal::new(50.0, 5.0).unwrap();
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        d.add_metric(normal.sample(&mut rng))?;
    }
    let ev = d.detect_change()?.expect("expected stable");
    assert_eq!(ev.degradation_type, DegradationType::Stable);
    assert!(ev.confidence > 0.9);
    Ok(())
}

#[test]
fn test_reset_clears_history() -> Result<(), Box<dyn std::error::Error>> {
    let mut d = DegradationDetector::new(100, 0.15, 2.5);
    for i in 0..50 {
        d.add_metric(i as f64)?;
    }
    d.reset();
    assert_eq!(d.history_len(), 0);
    let ev = d.detect_change()?;
    assert!(ev.is_none());
    Ok(())
}

#[test]
fn test_window_size_respected() -> Result<(), Box<dyn std::error::Error>> {
    let mut d = DegradationDetector::new(20, 0.15, 2.5);
    for i in 0..100 {
        d.add_metric(i as f64)?;
    }
    assert!(d.history_len() <= 20);
    Ok(())
}

#[test]
fn test_degradation_type_enum_variants() {
    assert_eq!(DegradationType::Gradual, DegradationType::Gradual);
    assert_eq!(DegradationType::Sudden, DegradationType::Sudden);
    assert_eq!(DegradationType::Oscillating, DegradationType::Oscillating);
    assert_eq!(DegradationType::Stable, DegradationType::Stable);
}

#[test]
fn test_event_clone_and_copy() -> Result<(), Box<dyn std::error::Error>> {
    let ev = DegradationEvent {
        timestamp: 1,
        degradation_type: DegradationType::Stable,
        magnitude: 0.0,
        confidence: 1.0,
        duration_samples: 0,
        affected_metric: "value".to_string(),
    };
    let ev2 = ev.clone();
    assert_eq!(ev.timestamp, ev2.timestamp);
    Ok(())
}

#[test]
fn test_event_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let ev = DegradationEvent {
        timestamp: 1,
        degradation_type: DegradationType::Gradual,
        magnitude: 0.42,
        confidence: 0.8,
        duration_samples: 10,
        affected_metric: "latency".to_string(),
    };
    let s = serde_json::to_string(&ev)?;
    let ev2: DegradationEvent = serde_json::from_str(&s)?;
    assert_eq!(ev2.degradation_type, DegradationType::Gradual);
    Ok(())
}

#[test]
fn test_nan_handling() {
    let mut d = DegradationDetector::new(10, 0.15, 2.5);
    let res = d.add_metric(f64::NAN);
    assert!(res.is_err());
}

#[test]
fn test_empty_history() -> Result<(), Box<dyn std::error::Error>> {
    let d = DegradationDetector::new(10, 0.15, 2.5);
    let ev = d.detect_change()?;
    assert!(ev.is_none());
    Ok(())
}

#[test]
fn test_single_metric() -> Result<(), Box<dyn std::error::Error>> {
    let mut d = DegradationDetector::new(10, 0.15, 2.5);
    d.add_metric(42.0)?;
    let ev = d.detect_change()?.expect("expected stable");
    assert_eq!(ev.degradation_type, DegradationType::Stable);
    Ok(())
}

#[test]
fn test_threshold_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let mut d = DegradationDetector::new(50, 0.15, 2.5);
    // values that hover around threshold magnitude
    for _ in 0..25 {
        d.add_metric(0.0)?;
    }
    for _ in 0..25 {
        d.add_metric(0.15)?;
    }
    // ensure no panic and returns either stable or gradual
    let ev = d.detect_change()?;
    assert!(ev.is_some());
    Ok(())
}

#[test]
fn test_detection_latency() -> Result<(), Box<dyn std::error::Error>> {
    let mut d = DegradationDetector::new(1000, 0.15, 2.5);
    for i in 0..1000 {
        d.add_metric((i % 100) as f64)?;
    }
    let start = Instant::now();
    let _ = d.detect_change()?;
    let dur = start.elapsed();
    // requested threshold: < 1ms
    assert!(dur.as_micros() < 1000, "detection too slow: {:?}", dur);
    Ok(())
}

#[test]
fn test_add_metric_latency() -> Result<(), Box<dyn std::error::Error>> {
    let mut d = DegradationDetector::new(1000, 0.15, 2.5);
    let start = Instant::now();
    for i in 0..1000 {
        d.add_metric(i as f64)?;
    }
    let dur = start.elapsed();
    let avg = dur.as_micros() as f64 / 1000.0;
    // requested threshold: < 10 microseconds per call
    assert!(avg < 10.0, "add_metric too slow: {}us avg", avg);
    Ok(())
}

// Integration test placeholder: only run if the genai_slo module is available.
#[test]
#[ignore]
fn test_degradation_in_genai_slo() {
    // Integration tests should be implemented in a separate task.
}

// Proptest to ensure no panics on arbitrary inputs
proptest! {
    #[test]
    fn proptest_no_panic(values in prop::collection::vec(0.0f64..1.0f64, 0..200), thr in 0.0001f64..0.5f64) {
        let mut d = DegradationDetector::new(100, thr, 2.5);
        for v in values.iter() {
            let _ = d.add_metric(*v);
        }
        let _ = d.detect_change();
    }
}
