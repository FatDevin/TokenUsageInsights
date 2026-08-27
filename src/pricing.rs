use serde::Serialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PricingRule {
    pub model_name: String,
    pub input_price: f64,
    pub cache_input_price: f64,
    pub output_price: f64,
}

#[derive(Serialize)]
pub struct PricingEntry {
    pub model_name: String,
    pub deployment_type: String,
    pub unit: String,
    pub input_price: f64,
    pub cache_input_price: f64,
    pub output_price: f64,
    pub batch_api_price: String,
}

pub fn load_pricing_rules() -> Vec<PricingRule> {
    let mut rules = Vec::new();
    let file_path =
        crate::paths::find_resource("pricing.csv").unwrap_or_else(|| PathBuf::from("pricing.csv"));
    if let Ok(file) = File::open(&file_path) {
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        if let Some(Ok(_header)) = lines.next() {
            for line in lines.map_while(Result::ok) {
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() >= 6 {
                    let input_price = parts[3].trim().parse::<f64>().unwrap_or(0.0);
                    let cache_input_price = parts[4].trim().parse::<f64>().unwrap_or(0.0);
                    let output_price = parts[5].trim().parse::<f64>().unwrap_or(0.0);
                    rules.push(PricingRule {
                        model_name: parts[0].trim().to_string(),
                        input_price,
                        cache_input_price,
                        output_price,
                    });
                }
            }
        }
    }
    if rules.is_empty() {
        rules = vec![
            PricingRule {
                model_name: "Gemini 3.5 Flash".to_string(),
                input_price: 1.50,
                cache_input_price: 0.375,
                output_price: 9.00,
            },
            PricingRule {
                model_name: "Gemini 1.5 Flash".to_string(),
                input_price: 0.075,
                cache_input_price: 0.01875,
                output_price: 0.30,
            },
            PricingRule {
                model_name: "Gemini 1.5 Pro".to_string(),
                input_price: 1.25,
                cache_input_price: 0.3125,
                output_price: 5.00,
            },
            PricingRule {
                model_name: "Gemini 2.0 Flash".to_string(),
                input_price: 0.10,
                cache_input_price: 0.025,
                output_price: 0.40,
            },
        ];
    }
    rules
}

/// Parsed long-context threshold marker from a pricing rule label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ThresholdRule {
    is_greater: bool,
    threshold_tokens: u64,
}

/// Parse a model/rule label into a normalized base name and optional threshold.
///
/// Threshold labels have appeared as `(>272k)`, `(>272k context length)`, and
/// `(>200k)`. Parse the number instead of coupling matching to one boundary.
fn parse_threshold_rule(name: &str) -> (String, Option<ThresholdRule>) {
    let lower = name.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let context_length: Vec<char> = "context length".chars().collect();
    let mut threshold = None;
    let mut cleaned = String::with_capacity(lower.len());
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c == '>' || c == '<' {
            let is_greater = c == '>';
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_whitespace() {
                j += 1;
            }

            let digits_start = j;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }

            if j > digits_start && j < chars.len() && chars[j] == 'k' {
                let digits: String = chars[digits_start..j].iter().collect();
                if let Ok(value) = digits.parse::<u64>() {
                    if threshold.is_none() {
                        threshold = Some(ThresholdRule {
                            is_greater,
                            threshold_tokens: value.saturating_mul(1_000),
                        });
                    }

                    j += 1;
                    while j < chars.len() && chars[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    if chars
                        .get(j..j + context_length.len())
                        .is_some_and(|suffix| suffix == context_length.as_slice())
                    {
                        j += context_length.len();
                    }
                    while j < chars.len() && chars[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    if j < chars.len() && chars[j] == ')' {
                        j += 1;
                    }

                    i = j;
                    continue;
                }
            }
        }

        cleaned.push(c);
        i += 1;
    }

    let normalized = cleaned.chars().filter(|c| c.is_alphanumeric()).collect();
    (normalized, threshold)
}

fn threshold_matches(rule: ThresholdRule, prompt_tokens: u64) -> bool {
    if rule.is_greater {
        prompt_tokens > rule.threshold_tokens
    } else {
        prompt_tokens <= rule.threshold_tokens
    }
}

fn rule_applies_to_context(
    rule_base: &str,
    rule_threshold: Option<ThresholdRule>,
    model_base: &str,
    prompt_tokens: u64,
    contains_match: bool,
) -> bool {
    if rule_base.is_empty() {
        return false;
    }

    let base_matches = if contains_match {
        model_base.contains(rule_base) || rule_base.contains(model_base)
    } else {
        rule_base == model_base
    };
    if !base_matches {
        return false;
    }

    rule_threshold
        .map(|threshold| threshold_matches(threshold, prompt_tokens))
        .unwrap_or(true)
}

/// Prefer the most specific applicable model base, then a threshold row over
/// an unthresholded default row, regardless of their order in the CSV.
fn find_pricing_rule<'a>(
    rules: &'a [PricingRule],
    model_base: &str,
    prompt_tokens: u64,
    contains_match: bool,
) -> Option<&'a PricingRule> {
    let mut best_rule = None;
    let mut best_base_len = 0;
    let mut best_has_threshold = false;

    for rule in rules {
        let (rule_base, rule_threshold) = parse_threshold_rule(&rule.model_name);
        if !rule_applies_to_context(
            &rule_base,
            rule_threshold,
            model_base,
            prompt_tokens,
            contains_match,
        ) {
            continue;
        }

        let base_len = rule_base.len();
        let has_threshold = rule_threshold.is_some();
        let is_more_specific = base_len > best_base_len;
        let is_same_base_with_threshold =
            base_len == best_base_len && has_threshold && !best_has_threshold;
        if best_rule.is_none() || is_more_specific || is_same_base_with_threshold {
            best_rule = Some(rule);
            best_base_len = base_len;
            best_has_threshold = has_threshold;
        }
    }

    best_rule
}

#[allow(dead_code)]
pub fn normalize_model_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

pub fn calculate_cost(
    rules: &[PricingRule],
    model_name: &str,
    input: u64,
    output: u64,
    cache_read: u64,
    cache_write_5m: u64,
    cache_write_1h: u64,
) -> Result<f64, String> {
    let (m_base, _) = parse_threshold_rule(model_name);
    if m_base.is_empty() {
        return Err(format!(
            "模型名稱為空，無法估算成本。來源模型：{}",
            model_name
        ));
    }

    let is_claude_model = rules.iter().any(|rule| {
        let (rule_base, _) = parse_threshold_rule(&rule.model_name);
        !rule_base.is_empty()
            && (rule_base == m_base || m_base.contains(&rule_base) || rule_base.contains(&m_base))
            && rule.model_name.to_ascii_lowercase().contains("claude")
    });
    let (priced_cache_write_5m, priced_cache_write_1h) = if is_claude_model {
        (cache_write_5m, cache_write_1h)
    } else {
        (0, 0)
    };
    // Provider long-context tiers apply to prompt tokens. Output tokens do not
    // increase the prompt size; cached reads and Claude cache writes do.
    let prompt_tokens = input
        .saturating_add(cache_read)
        .saturating_add(priced_cache_write_5m)
        .saturating_add(priced_cache_write_1h);

    // 1. Exact base name match (threshold-aware)
    let mut rule = find_pricing_rule(rules, &m_base, prompt_tokens, false);

    // 2. Fallback: contains base name match
    if rule.is_none() {
        rule = find_pricing_rule(rules, &m_base, prompt_tokens, true);
    }

    if let Some(r) = rule {
        let input_cost = (input as f64 / 1_000_000.0) * r.input_price;
        let cache_cost = (cache_read as f64 / 1_000_000.0) * r.cache_input_price;
        let cache_write_5m_cost =
            (priced_cache_write_5m as f64 / 1_000_000.0) * r.input_price * 1.25;
        let cache_write_1h_cost =
            (priced_cache_write_1h as f64 / 1_000_000.0) * r.input_price * 2.0;
        let output_cost = (output as f64 / 1_000_000.0) * r.output_price;
        Ok(input_cost + cache_cost + cache_write_5m_cost + cache_write_1h_cost + output_cost)
    } else {
        Err(format!("找不到可用的模型價格規則：{}", model_name))
    }
}

pub fn calculate_usage_cost(
    rules: &[PricingRule],
    model_name: Option<&str>,
    input: u64,
    output: u64,
    cache_read: u64,
    cache_write_5m: u64,
    cache_write_1h: u64,
) -> Result<f64, String> {
    if input == 0 && output == 0 && cache_read == 0 && cache_write_5m == 0 && cache_write_1h == 0 {
        return Ok(0.0);
    }

    let model_name = model_name
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "缺少模型名稱，無法估算成本".to_string())?;
    calculate_cost(
        rules,
        model_name,
        input,
        output,
        cache_read,
        cache_write_5m,
        cache_write_1h,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_token_usage_without_model_costs_zero() {
        let cost = calculate_usage_cost(&[], None, 0, 0, 0, 0, 0).unwrap();
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn token_usage_without_model_reports_missing_metadata() {
        let error = calculate_usage_cost(&[], None, 10, 2, 3, 0, 0).unwrap_err();
        assert_eq!(error, "缺少模型名稱，無法估算成本");
    }

    #[test]
    fn copilot_cli_cost_uses_non_cached_input() {
        let rules = [PricingRule {
            model_name: "MAI-Code-1-Flash".to_string(),
            input_price: 0.75,
            cache_input_price: 0.075,
            output_price: 4.50,
        }];

        let cost = calculate_usage_cost(
            &rules,
            Some("mai-code-1-flash-picker · medium"),
            42_530,
            1_370,
            401_024,
            0,
            0,
        )
        .unwrap();

        assert!((cost - 0.068_139_3).abs() < f64::EPSILON);
    }

    #[test]
    fn claude_opus_5_variants_use_packaged_pricing() {
        let rules = load_pricing_rules();

        for model_name in [
            "claude-opus-5",
            "Claude Opus 5",
            "claude-opus-5-1m · high",
            "opus-5",
        ] {
            let cost = calculate_usage_cost(
                &rules,
                Some(model_name),
                1_000_000,
                1_000_000,
                1_000_000,
                0,
                0,
            )
            .unwrap();

            assert!(
                (cost - 30.5).abs() < 1e-9,
                "unexpected Opus 5 cost for {model_name}: {cost}"
            );
        }
    }

    #[test]
    fn gemini_3_7_flash_thinking_levels_use_packaged_pricing() {
        let rules = load_pricing_rules();

        for model_name in ["Gemini 3.7 Flash (High)", "Gemini 3.7 Flash (Low)"] {
            let cost = calculate_usage_cost(
                &rules,
                Some(model_name),
                1_000_000,
                1_000_000,
                1_000_000,
                0,
                0,
            )
            .unwrap();

            assert!(
                (cost - 9.15).abs() < 1e-9,
                "unexpected Gemini 3.7 Flash cost for {model_name}: {cost}"
            );
        }
    }

    #[test]
    fn gemini_3_6_flash_thinking_levels_use_packaged_pricing() {
        let rules = load_pricing_rules();

        for model_name in ["Gemini 3.6 Flash (High)", "Gemini 3.6 Flash (Low)"] {
            let cost = calculate_usage_cost(
                &rules,
                Some(model_name),
                1_000_000,
                1_000_000,
                1_000_000,
                0,
                0,
            )
            .unwrap();

            assert!(
                (cost - 9.15).abs() < 1e-9,
                "unexpected Gemini 3.6 Flash cost for {model_name}: {cost}"
            );
        }
    }

    #[test]
    fn cache_writes_use_official_ttl_multipliers() {
        let rules = [PricingRule {
            model_name: "Claude Fable 5".to_string(),
            input_price: 10.0,
            cache_input_price: 1.0,
            output_price: 50.0,
        }];

        let cost = calculate_usage_cost(
            &rules,
            Some("claude-fable-5"),
            1_000_000,
            1_000_000,
            1_000_000,
            1_000_000,
            1_000_000,
        )
        .unwrap();

        assert!((cost - 93.5).abs() < 1e-9);
    }

    #[test]
    fn cache_write_ttl_fields_do_not_change_non_claude_cost() {
        let rules = [PricingRule {
            model_name: "GPT-5".to_string(),
            input_price: 2.0,
            cache_input_price: 0.2,
            output_price: 8.0,
        }];

        let cost = calculate_usage_cost(
            &rules,
            Some("gpt-5"),
            1_000_000,
            1_000_000,
            1_000_000,
            1_000_000,
            1_000_000,
        )
        .unwrap();

        assert!((cost - 10.2).abs() < 1e-9);
    }

    fn gemini_pro_rules() -> Vec<PricingRule> {
        vec![
            PricingRule {
                model_name: "Gemini 3.1 Pro (Low) (<200k)".to_string(),
                input_price: 2.00,
                cache_input_price: 0.20,
                output_price: 12.00,
            },
            PricingRule {
                model_name: "Gemini 3.1 Pro (Low) (>200k)".to_string(),
                input_price: 4.00,
                cache_input_price: 0.40,
                output_price: 18.00,
            },
            PricingRule {
                model_name: "Gemini 3.1 Pro (Low)".to_string(),
                input_price: 2.00,
                cache_input_price: 0.20,
                output_price: 12.00,
            },
        ]
    }

    #[test]
    fn parse_threshold_rule_supports_variable_boundaries_and_legacy_suffix() {
        let (short_base, short) = parse_threshold_rule("Gemini 3.1 Pro (Low) (< 200K)");
        assert_eq!(short_base, "gemini31prolow");
        assert_eq!(
            short,
            Some(ThresholdRule {
                is_greater: false,
                threshold_tokens: 200_000,
            })
        );

        let (long_base, long) = parse_threshold_rule("GPT-5.5 (>272k context length)");
        assert_eq!(long_base, "gpt55");
        assert_eq!(
            long,
            Some(ThresholdRule {
                is_greater: true,
                threshold_tokens: 272_000,
            })
        );
    }

    #[test]
    fn threshold_uses_prompt_tokens_without_counting_output() {
        let rules = gemini_pro_rules();

        // A 190k prompt remains in the short tier even with a 20k response.
        let cost =
            calculate_cost(&rules, "Gemini 3.1 Pro (Low)", 190_000, 20_000, 0, 0, 0).unwrap();
        let expected = (190_000.0 / 1_000_000.0) * 2.00 + (20_000.0 / 1_000_000.0) * 12.00;
        assert!((cost - expected).abs() < 1e-12);
    }

    #[test]
    fn cache_read_tokens_count_toward_prompt_threshold() {
        let rules = gemini_pro_rules();

        // 190k input + 11k cached prompt = 201k, so the long tier applies.
        let cost = calculate_cost(
            &rules,
            "Gemini 3.1 Pro (Low)",
            190_000,
            20_000,
            11_000,
            0,
            0,
        )
        .unwrap();
        let expected = (190_000.0 / 1_000_000.0) * 4.00
            + (11_000.0 / 1_000_000.0) * 0.40
            + (20_000.0 / 1_000_000.0) * 18.00;
        assert!((cost - expected).abs() < 1e-12);
    }

    #[test]
    fn threshold_boundary_uses_short_tier() {
        let rules = gemini_pro_rules();

        // Exactly 200k prompt tokens remains in the <=200k tier.
        let cost =
            calculate_cost(&rules, "Gemini 3.1 Pro (Low)", 200_000, 20_000, 0, 0, 0).unwrap();
        let expected = (200_000.0 / 1_000_000.0) * 2.00 + (20_000.0 / 1_000_000.0) * 12.00;
        assert!((cost - expected).abs() < 1e-12);
    }

    #[test]
    fn threshold_rule_wins_when_default_is_listed_first() {
        let rules = [
            PricingRule {
                model_name: "GPT-5.5".to_string(),
                input_price: 5.00,
                cache_input_price: 0.50,
                output_price: 30.00,
            },
            PricingRule {
                model_name: "GPT-5.5 (>272k context length)".to_string(),
                input_price: 10.00,
                cache_input_price: 1.00,
                output_price: 45.00,
            },
            PricingRule {
                model_name: "GPT-5.5 (<272k context length)".to_string(),
                input_price: 5.00,
                cache_input_price: 0.50,
                output_price: 30.00,
            },
        ];

        let short = calculate_cost(&rules, "GPT-5.5", 100_000, 20_000, 0, 0, 0).unwrap();
        let short_expected = (100_000.0 / 1_000_000.0) * 5.00 + (20_000.0 / 1_000_000.0) * 30.00;
        assert!((short - short_expected).abs() < 1e-12);

        let long = calculate_cost(&rules, "GPT-5.5", 300_000, 20_000, 0, 0, 0).unwrap();
        let long_expected = (300_000.0 / 1_000_000.0) * 10.00 + (20_000.0 / 1_000_000.0) * 45.00;
        assert!((long - long_expected).abs() < 1e-12);
    }

    #[test]
    fn contains_fallback_prefers_the_most_specific_model_base() {
        let rules = [
            PricingRule {
                model_name: "GPT-5.4 (<272k)".to_string(),
                input_price: 2.50,
                cache_input_price: 0.25,
                output_price: 15.00,
            },
            PricingRule {
                model_name: "GPT-5.4-mini".to_string(),
                input_price: 0.75,
                cache_input_price: 0.08,
                output_price: 4.50,
            },
        ];

        let cost = calculate_cost(&rules, "GPT-5.4-mini-picker", 100_000, 0, 0, 0, 0).unwrap();

        assert!((cost - 0.075).abs() < 1e-12);
    }

    #[test]
    fn packaged_gemini_pricing_uses_standard_cache_rates() {
        let rules = load_pricing_rules();

        let short_context =
            calculate_cost(&rules, "Gemini 3.1 Pro (Low)", 100_000, 0, 100_000, 0, 0).unwrap();
        assert!((short_context - 0.22).abs() < 1e-12);

        let long_context =
            calculate_cost(&rules, "Gemini 3.1 Pro (Low)", 201_000, 0, 0, 0, 0).unwrap();
        assert!((long_context - 0.804).abs() < 1e-12);
    }

    #[test]
    fn packaged_grok_build_01_pricing_is_distinct_from_grok_45() {
        let rules = load_pricing_rules();

        let short_context =
            calculate_usage_cost(&rules, Some("grok-build-0.1"), 100_000, 0, 100_000, 0, 0)
                .unwrap();
        let long_context =
            calculate_usage_cost(&rules, Some("grok-build-0.1"), 201_000, 0, 0, 0, 0).unwrap();
        let grok_45 =
            calculate_usage_cost(&rules, Some("grok-4.5"), 100_000, 0, 100_000, 0, 0).unwrap();

        assert!((short_context - 0.12).abs() < 1e-12);
        assert!((long_context - 0.402).abs() < 1e-12);
        assert!((grok_45 - 0.23).abs() < 1e-12);
    }

    #[test]
    fn kimi_k3_resolves_pricing_from_csv() {
        let rules = load_pricing_rules();

        let cost = calculate_usage_cost(&rules, Some("kimi-k3"), 1_000_000, 1_000_000, 0, 0, 0)
            .expect("kimi-k3 should have a pricing rule");
        // input 3.00 + output 15.00 = 18.00
        assert!((cost - 18.0).abs() < 1e-12);
    }

    #[test]
    fn composer_2_5_speed_tiers_resolve_distinct_pricing_from_csv() {
        let rules = load_pricing_rules();

        let standard = calculate_usage_cost(
            &rules,
            Some("composer-2.5"),
            1_000_000,
            1_000_000,
            1_000_000,
            0,
            0,
        )
        .expect("composer-2.5 should have a pricing rule");
        // input 0.50 + cache read 0.20 + output 2.50 = 3.20
        assert!((standard - 3.2).abs() < 1e-12);

        let fast = calculate_usage_cost(
            &rules,
            Some("composer-2.5-fast"),
            1_000_000,
            1_000_000,
            1_000_000,
            0,
            0,
        )
        .expect("composer-2.5-fast should have a pricing rule");
        // input 3.00 + cache read 0.50 + output 15.00 = 18.50
        assert!((fast - 18.5).abs() < 1e-12);
    }
}
