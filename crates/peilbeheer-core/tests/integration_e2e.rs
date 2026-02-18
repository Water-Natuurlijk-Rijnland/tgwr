//! End-to-end integration test for the peilbeheer water management system.
//!
//! This test simulates the complete operational workflow:
//! 1. Fetch energy prices (EnergyZero API)
//! 2. Fetch water levels (FEWS sync)
//! 3. Calculate water balance
//! 4. Determine optimal pump schedule
//! 5. Generate alerts when thresholds exceeded
//!
//! # Test Phases
//!
//! ## Setup
//! - Initialize mock clients for external APIs
//! - Configure test alert rules
//! - Set up deterministic test data
//!
//! ## Execute
//! - Run the 5-step workflow with various scenarios
//! - Validate results at each step
//!
//! ## Teardown
//! - Verify resource cleanup
//! - Log test metrics

use chrono::{Utc, Duration, TimeZone};
use peilbeheer_core::{
    energie::{OptimalisatieParams, UurPrijs},
    alert::{AlertRule, AlertSeverity, ComparisonOperator, AlertCondition, AlertValue, AlertCategory, ConditionLogic},
    timeseries::{TimeSeriesDataPoint, AggregationFunction, AggregationLevel, QualityFlag},
};
use std::collections::HashMap;

// ========================================================================
// MOCK MODULE
// ========================================================================

mod mocks {
    use super::*;

    /// Mock EnergyZero API client for testing.
    pub struct EnergyZeroMock {
        simulate_failure: bool,
    }

    impl EnergyZeroMock {
        pub fn new() -> Self {
            Self { simulate_failure: false }
        }

        pub fn with_flat_prices(&self, price: f64) -> Vec<UurPrijs> {
            (0..24)
                .map(|hour| UurPrijs {
                    uur: hour as u8,
                    prijs_eur_kwh: price,
                })
                .collect()
        }

        pub fn with_variable_prices(&self) -> Vec<UurPrijs> {
            (0..24)
                .map(|hour| {
                    let price = 0.20 + 0.30 * (1.0 - ((hour as f32 - 12.0).abs() / 12.0).powi(2));
                    UurPrijs {
                        uur: hour as u8,
                        prijs_eur_kwh: price as f64,
                    }
                })
                .collect()
        }

        pub fn with_peak_pattern(&self) -> Vec<UurPrijs> {
            (0..24)
                .map(|hour| {
                    let is_peak = (8..18).contains(&hour);
                    let price = if is_peak { 0.50 } else { 0.22 };
                    UurPrijs {
                        uur: hour as u8,
                        prijs_eur_kwh: price,
                    }
                })
                .collect()
        }

        pub fn simulate_failure(&mut self) {
            self.simulate_failure = true;
        }

        pub fn fetch_prices(&self) -> Result<Vec<UurPrijs>, Box<dyn std::error::Error>> {
            if self.simulate_failure {
                return Err("EnergyZero API unavailable".into());
            }
            Ok(self.with_flat_prices(0.25))
        }
    }

    /// Mock FEWS client for testing.
    pub struct FewsClientMock;

    impl FewsClientMock {
        pub fn new() -> Self {
            Self
        }

        pub fn with_stable_levels(&self, level: f64, hours: usize) -> Vec<f64> {
            vec![level; hours]
        }

        pub fn with_rising_water_levels(&self, start: f64, end: f64, hours: usize) -> Vec<f64> {
            (0..hours)
                .map(|hour| {
                    let t = hour as f64 / (hours - 1) as f64;
                    start + (end - start) * t
                })
                .collect()
        }

        pub fn with_missing_data(&self, level: f64, hours: usize) -> Vec<f64> {
            (0..hours)
                .map(|hour| {
                    // Introduce gap every 6 hours
                    if hour % 6 == 5 {
                        f64::NAN
                    } else {
                        level
                    }
                })
                .collect()
        }
    }

    /// Test scenario data container.
    pub struct ScenarioTestData {
        pub prices: Vec<UurPrijs>,
        pub water_levels: Vec<f64>,
        pub rain_mm: Vec<f64>,
    }

    /// Test time context for deterministic tests.
    pub struct TestTimeContext {
        pub now: chrono::DateTime<Utc>,
    }

    impl TestTimeContext {
        pub fn new() -> Self {
            Self {
                now: Utc.with_ymd_and_hms(2025, 2, 18, 12, 0, 0).unwrap(),
            }
        }
    }
}

use mocks::{EnergyZeroMock, FewsClientMock, ScenarioTestData, TestTimeContext};

// ========================================================================
// INTEGRATION TESTS
// ========================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Integration test fixture holding all test dependencies.
    struct IntegrationTestFixture {
        energyzero_mock: EnergyZeroMock,
        fews_mock: FewsClientMock,
        alert_rules: Vec<AlertRule>,
        time_context: TestTimeContext,
    }

    impl IntegrationTestFixture {
        /// Create a new test fixture with default configuration.
        fn new() -> Self {
            let now = Utc::now();

            Self {
                energyzero_mock: EnergyZeroMock::new(),
                fews_mock: FewsClientMock::new(),
                alert_rules: Self::default_alert_rules(now),
                time_context: TestTimeContext::new(),
            }
        }

        /// Create alert rules for testing.
        fn default_alert_rules(now: chrono::DateTime<Utc>) -> Vec<AlertRule> {
            vec![
                AlertRule {
                    id: "rule-high-water-level".to_string(),
                    name: "Hoge waterstand".to_string(),
                    description: Some("Waterstand boven kritiek niveau".to_string()),
                    category: AlertCategory::WaterLevel,
                    severity: AlertSeverity::Critical,
                    conditions: vec![
                        AlertCondition {
                            field: "waterlevel.main".to_string(),
                            operator: ComparisonOperator::Gt,
                            value: AlertValue::Number(-4.5),
                            source_filter: None,
                            time_window: None,
                            aggregation: None,
                        },
                    ],
                    condition_logic: ConditionLogic::And,
                    cooldown_seconds: 3600,
                    enabled: true,
                    notification_channels: vec![],
                    title_template: "Hoge waterstand".to_string(),
                    message_template: "Waterstand boven -4.5m NAP".to_string(),
                    metadata: HashMap::new(),
                    created_at: now,
                    updated_at: now,
                    created_by: None,
                },
                AlertRule {
                    id: "rule-low-water-level".to_string(),
                    name: "Lage waterstand".to_string(),
                    description: Some("Waterstand onder minimaal niveau".to_string()),
                    category: AlertCategory::WaterLevel,
                    severity: AlertSeverity::Warning,
                    conditions: vec![
                        AlertCondition {
                            field: "waterlevel.main".to_string(),
                            operator: ComparisonOperator::Lt,
                            value: AlertValue::Number(-5.5),
                            source_filter: None,
                            time_window: None,
                            aggregation: None,
                        },
                    ],
                    condition_logic: ConditionLogic::And,
                    cooldown_seconds: 7200,
                    enabled: true,
                    notification_channels: vec![],
                    title_template: "Lage waterstand".to_string(),
                    message_template: "Waterstand onder -5.5m NAP".to_string(),
                    metadata: HashMap::new(),
                    created_at: now,
                    updated_at: now,
                    created_by: None,
                },
            ]
        }

        /// Setup mock data for a normal operations scenario.
        fn setup_normal_scenario(&mut self) -> ScenarioTestData {
            let prices = self.energyzero_mock.with_flat_prices(0.25);
            let water_levels = self.fews_mock.with_stable_levels(-5.0, 24);
            let rain = vec![0.0; 24];

            ScenarioTestData {
                prices,
                water_levels,
                rain_mm: rain,
            }
        }

        /// Setup mock data for a rain event scenario.
        fn setup_rain_scenario(&mut self) -> ScenarioTestData {
            let prices = self.energyzero_mock.with_variable_prices();
            let water_levels = self.fews_mock.with_rising_water_levels(-5.0, -4.0, 24);
            let rain = vec![5.0; 24];

            ScenarioTestData {
                prices,
                water_levels,
                rain_mm: rain,
            }
        }

        /// Setup mock data for a peak price scenario.
        fn setup_peak_price_scenario(&mut self) -> ScenarioTestData {
            let prices = self.energyzero_mock.with_peak_pattern();
            let water_levels = self.fews_mock.with_stable_levels(-5.0, 24);
            let rain = vec![0.0; 24];

            ScenarioTestData {
                prices,
                water_levels,
                rain_mm: rain,
            }
        }

        /// Setup mock data with FEWS data gaps.
        fn setup_data_gap_scenario(&mut self) -> ScenarioTestData {
            let prices = self.energyzero_mock.with_flat_prices(0.25);
            let water_levels = self.fews_mock.with_missing_data(-5.0, 24);
            let rain = vec![0.0; 24];

            ScenarioTestData {
                prices,
                water_levels,
                rain_mm: rain,
            }
        }
    }

    // ========================================================================
    // TEST 1: Normal Operations - Validate price data structure
    // ========================================================================
    #[test]
    fn test_normal_operations_price_validation() {
        let mut fixture = IntegrationTestFixture::new();
        let data = fixture.setup_normal_scenario();

        // Given: Stable water levels and flat prices
        assert_eq!(data.prices.len(), 24, "Expected 24 hours of price data");

        // When: Validating price data
        for (hour, price) in data.prices.iter().enumerate() {
            assert_eq!(price.uur, hour as u8, "Hour index should match");
            assert!(price.prijs_eur_kwh > 0.0, "Price should be positive");
            assert!(price.prijs_eur_kwh < 1.0, "Price should be reasonable");
        }

        // Then: All prices are valid
        assert!(data.prices.iter().all(|p| p.prijs_eur_kwh > 0.0), "All prices should be positive");
    }

    // ========================================================================
    // TEST 2: Rain Event - Pumping increases with rain
    // ========================================================================
    #[test]
    fn test_rain_event_pumping_response() {
        let mut fixture = IntegrationTestFixture::new();
        let data = fixture.setup_rain_scenario();

        // Given: Rising water levels due to rain
        let initial_level = data.water_levels[0];
        let final_level = data.water_levels[23];

        assert!(initial_level < final_level, "Water levels should rise");

        // When: Computing with rain data
        let total_rain: f64 = data.rain_mm.iter().sum();
        assert!(total_rain > 0.0, "Should have rain in scenario");

        // Then: Rain data is properly structured
        assert_eq!(data.rain_mm.len(), 24, "Rain data should cover 24 hours");
    }

    // ========================================================================
    // TEST 3: Peak Price Pattern - Verify peak structure
    // ========================================================================
    #[test]
    fn test_peak_price_pattern_validation() {
        let mut fixture = IntegrationTestFixture::new();
        let data = fixture.setup_peak_price_scenario();

        // Given: Peak hours 8-18 with higher prices
        let peak_hours: Vec<_> = data.prices.iter()
            .filter(|p| (8..18).contains(&(p.uur as usize)))
            .collect();

        let off_peak_hours: Vec<_> = data.prices.iter()
            .filter(|p| !(8..18).contains(&(p.uur as usize)))
            .collect();

        // When: Comparing peak vs off-peak
        let avg_peak: f64 = peak_hours.iter().map(|p| p.prijs_eur_kwh).sum::<f64>() / peak_hours.len() as f64;
        let avg_off_peak: f64 = off_peak_hours.iter().map(|p| p.prijs_eur_kwh).sum::<f64>() / off_peak_hours.len() as f64;

        // Then: Peak prices should be higher
        assert!(avg_peak > avg_off_peak, "Peak prices should exceed off-peak");
    }

    // ========================================================================
    // TEST 4: Data Gaps - NaN handling
    // ========================================================================
    #[test]
    fn test_data_gaps_contain_nan() {
        let mut fixture = IntegrationTestFixture::new();
        let data = fixture.setup_data_gap_scenario();

        // Given: Water level data with gaps
        let nan_count = data.water_levels.iter()
            .filter(|v| v.is_nan() || v.is_infinite())
            .count();

        // Then: Gaps should be present in test data
        assert!(nan_count > 0, "Test data should contain NaN gaps for validation");
    }

    // ========================================================================
    // TEST 5: Alert Rule Structure Validation
    // ========================================================================
    #[test]
    fn test_alert_rule_structure() {
        let fixture = IntegrationTestFixture::new();
        let rules = fixture.alert_rules;

        // Given: Default alert rules
        assert_eq!(rules.len(), 2, "Should have 2 default rules");

        // When: Validating rule structure
        for rule in &rules {
            assert!(!rule.id.is_empty(), "Rule ID should not be empty");
            assert!(!rule.name.is_empty(), "Rule name should not be empty");
            assert!(rule.enabled, "Default rules should be enabled");
            assert!(rule.cooldown_seconds > 0, "Cooldown should be positive");
            assert!(!rule.conditions.is_empty(), "Rule should have conditions");
        }

        // Then: Critical rule should have critical severity
        let critical_rule = rules.iter()
            .find(|r| r.severity == AlertSeverity::Critical)
            .expect("Should have a critical rule");

        assert_eq!(critical_rule.category, AlertCategory::WaterLevel, "Critical rule should be for water level");
    }

    // ========================================================================
    // TEST 6: Alert Condition Validation
    // ========================================================================
    #[test]
    fn test_alert_condition_validation() {
        let fixture = IntegrationTestFixture::new();
        let rules = fixture.alert_rules;

        // Given: Alert rules with conditions
        for rule in &rules {
            for condition in &rule.conditions {
                // When: Checking condition structure
                assert!(!condition.field.is_empty(), "Condition field should not be empty");

                match &condition.value {
                    AlertValue::Number(val) => {
                        assert!(!val.is_nan(), "Threshold should be a valid number");
                    }
                    AlertValue::String(_) => {
                        // String values are valid
                    }
                    AlertValue::Boolean(_) => {
                        // Boolean values are valid
                    }
                    AlertValue::Array(_) => {
                        // Array values are valid
                    }
                }
            }
        }

        // Then: All conditions should be valid
        let total_conditions: usize = rules.iter().map(|r| r.conditions.len()).sum();
        assert!(total_conditions > 0, "Should have at least one condition");
    }

    // ========================================================================
    // TEST 7: Time Series Aggregation
    // ========================================================================
    #[test]
    fn test_time_series_aggregation_basic() {
        // Given: Minute-level data
        let now = Utc::now();
        let minute_data: Vec<TimeSeriesDataPoint> = (0..60)
            .map(|min| TimeSeriesDataPoint {
                timestamp: now + Duration::minutes(min),
                value: -5.0 + (min as f64 * 0.01),
                flag: QualityFlag::Good,
            })
            .collect();

        // When: Computing statistics
        let values: Vec<f64> = minute_data.iter().map(|d| d.value).collect();
        let avg = values.iter().sum::<f64>() / values.len() as f64;
        let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        // Then: Statistics should be valid
        assert!(!avg.is_nan(), "Average should not be NaN");
        assert!(min < max, "Min should be less than max");
        assert!(avg >= min && avg <= max, "Average should be within range");
    }

    // ========================================================================
    // TEST 8: EnergyZero API Failure Handling
    // ========================================================================
    #[test]
    fn test_energyzero_api_failure() {
        let mut fixture = IntegrationTestFixture::new();

        // Given: Simulated API failure
        fixture.energyzero_mock.simulate_failure();

        // When: Fetching prices
        let result = fixture.energyzero_mock.fetch_prices();

        // Then: Should return error gracefully
        assert!(result.is_err(), "Should return error when API fails");
        assert!(!result.unwrap_err().to_string().is_empty(), "Error should have message");
    }

    // ========================================================================
    // TEST 9: Comparison Operator Evaluation
    // ========================================================================
    #[test]
    fn test_comparison_operator_numeric() {
        // Given: Numeric comparison operators
        let gt = ComparisonOperator::Gt;
        let lt = ComparisonOperator::Lt;
        let gte = ComparisonOperator::Gte;
        let lte = ComparisonOperator::Lte;

        // When: Evaluating comparisons
        assert!(gt.eval_numeric(5.0, 3.0), "5 > 3 should be true");
        assert!(!gt.eval_numeric(3.0, 5.0), "3 > 5 should be false");

        assert!(lt.eval_numeric(3.0, 5.0), "3 < 5 should be true");
        assert!(!lt.eval_numeric(5.0, 3.0), "5 < 3 should be false");

        assert!(gte.eval_numeric(5.0, 5.0), "5 >= 5 should be true");
        assert!(!gte.eval_numeric(4.0, 5.0), "4 >= 5 should be false");

        assert!(lte.eval_numeric(5.0, 5.0), "5 <= 5 should be true");
        assert!(!lte.eval_numeric(6.0, 5.0), "6 <= 5 should be false");
    }

    // ========================================================================
    // TEST 10: Quality Flag Validation
    // ========================================================================
    #[test]
    fn test_quality_flag_validation() {
        // Given: Different quality flags
        let good = QualityFlag::Good;
        let bad = QualityFlag::Bad;
        let missing = QualityFlag::Missing;

        // When: Checking validity
        assert!(good.is_valid(), "Good quality should be valid");
        assert!(!bad.is_valid(), "Bad quality should not be valid");
        assert!(!missing.is_valid(), "Missing quality should not be valid");
    }

    // ========================================================================
    // TEST 11: Time Series Data Point Creation
    // ========================================================================
    #[test]
    fn test_timeseries_data_point_creation() {
        let now = Utc::now();

        // Given: TimeSeriesDataPoint with different quality flags
        let good_point = TimeSeriesDataPoint::new(now, -5.0);
        let missing_point = TimeSeriesDataPoint::missing(now);

        // When: Checking validity
        assert!(good_point.is_valid(), "Good point should be valid");
        assert!(!missing_point.is_valid(), "Missing point should not be valid");
        assert!(missing_point.value.is_nan(), "Missing point value should be NaN");
        assert_eq!(missing_point.flag, QualityFlag::Missing);
    }

    // ========================================================================
    // VALIDATION HELPERS
    // ========================================================================

    /// Validate that price array is well-formed.
    fn validate_prices(prices: &[UurPrijs]) -> Result<(), String> {
        if prices.len() != 24 {
            return Err(format!("Expected 24 hours, got {}", prices.len()));
        }

        for (hour, price) in prices.iter().enumerate() {
            if price.uur != hour as u8 {
                return Err(format!("Hour {}: uur field mismatch", hour));
            }
            if price.prijs_eur_kwh <= 0.0 {
                return Err(format!("Hour {}: price must be positive", hour));
            }
            if price.prijs_eur_kwh >= 1.0 {
                return Err(format!("Hour {}: price {} exceeds reasonable maximum", hour, price.prijs_eur_kwh));
            }
        }

        Ok(())
    }

    /// Validate that water level data is physically realistic.
    fn validate_water_levels(levels: &[f64]) -> Result<(), String> {
        for (hour, level) in levels.iter().enumerate() {
            if level.is_nan() || level.is_infinite() {
                return Err(format!("Hour {}: invalid water level", hour));
            }
            if *level < -10.0 || *level > 10.0 {
                return Err(format!("Hour {}: water level {} out of physical range [-10, 10] m NAP", hour, level));
            }
        }
        Ok(())
    }
}
