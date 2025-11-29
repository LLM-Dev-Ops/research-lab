use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use statrs::distribution::{ContinuousCDF, StudentsT};
use statrs::statistics::Statistics;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalResult {
    pub statistic: f64,
    pub p_value: Option<f64>,
    pub confidence_interval: Option<(f64, f64)>,
    pub effect_size: Option<f64>,
}

pub struct StatisticalAnalyzer;

impl StatisticalAnalyzer {
    /// Calculate confidence interval for mean at given confidence level
    pub fn confidence_interval(values: &[f64], confidence: f64) -> (Decimal, Decimal) {
        if values.len() < 2 {
            return (Decimal::ZERO, Decimal::ZERO);
        }

        let mean = values.mean();
        let std_dev = values.std_dev();
        let n = values.len() as f64;
        let df = n - 1.0;

        let t_dist = StudentsT::new(0.0, 1.0, df).unwrap();
        let t_value = t_dist.inverse_cdf((1.0 + confidence) / 2.0);

        let margin = t_value * (std_dev / n.sqrt());
        let lower = mean - margin;
        let upper = mean + margin;

        (
            Decimal::try_from(lower).unwrap_or_default(),
            Decimal::try_from(upper).unwrap_or_default(),
        )
    }

    /// Perform t-test to compare two samples
    pub fn t_test(sample1: &[f64], sample2: &[f64]) -> StatisticalResult {
        if sample1.len() < 2 || sample2.len() < 2 {
            return StatisticalResult {
                statistic: 0.0,
                p_value: None,
                confidence_interval: None,
                effect_size: None,
            };
        }

        let mean1 = sample1.mean();
        let mean2 = sample2.mean();
        let var1 = sample1.variance();
        let var2 = sample2.variance();
        let n1 = sample1.len() as f64;
        let n2 = sample2.len() as f64;

        let pooled_var = ((n1 - 1.0) * var1 + (n2 - 1.0) * var2) / (n1 + n2 - 2.0);
        let t_stat = (mean1 - mean2) / (pooled_var * (1.0 / n1 + 1.0 / n2)).sqrt();

        // Calculate degrees of freedom and p-value
        let df = n1 + n2 - 2.0;
        let t_dist = StudentsT::new(0.0, 1.0, df).unwrap_or_else(|_| StudentsT::new(0.0, 1.0, 1.0).unwrap());
        let p_value = 2.0 * (1.0 - t_dist.cdf(t_stat.abs()));

        StatisticalResult {
            statistic: t_stat,
            p_value: Some(p_value),
            confidence_interval: None,
            effect_size: Some(Self::cohens_d(sample1, sample2)),
        }
    }

    /// Mann-Whitney U test (non-parametric alternative to t-test)
    pub fn mann_whitney_u(sample1: &[f64], sample2: &[f64]) -> StatisticalResult {
        if sample1.is_empty() || sample2.is_empty() {
            return StatisticalResult {
                statistic: 0.0,
                p_value: None,
                confidence_interval: None,
                effect_size: None,
            };
        }

        let n1 = sample1.len();
        let n2 = sample2.len();

        // Combine and rank all values
        let mut combined: Vec<(f64, usize)> = sample1
            .iter()
            .map(|&x| (x, 1))
            .chain(sample2.iter().map(|&x| (x, 2)))
            .collect();

        combined.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // Assign ranks (handling ties)
        let mut rank_sum1 = 0.0;
        let mut i = 0;
        while i < combined.len() {
            let mut j = i;
            while j < combined.len() && combined[j].0 == combined[i].0 {
                j += 1;
            }
            let rank = (i + j + 1) as f64 / 2.0;
            for k in i..j {
                if combined[k].1 == 1 {
                    rank_sum1 += rank;
                }
            }
            i = j;
        }

        // Calculate U statistic
        let u1 = rank_sum1 - (n1 * (n1 + 1)) as f64 / 2.0;
        let u2 = (n1 * n2) as f64 - u1;
        let u = u1.min(u2);

        // Calculate z-score and p-value for large samples
        let mean_u = (n1 * n2) as f64 / 2.0;
        let std_u = ((n1 * n2 * (n1 + n2 + 1)) as f64 / 12.0).sqrt();
        let z = (u - mean_u) / std_u;

        // Approximate p-value using normal distribution
        let p_value = 2.0 * (1.0 - Self::normal_cdf(z.abs()));

        StatisticalResult {
            statistic: u,
            p_value: Some(p_value),
            confidence_interval: None,
            effect_size: None,
        }
    }

    /// Bootstrap comparison for confidence intervals
    pub fn bootstrap_comparison(
        sample1: &[f64],
        sample2: &[f64],
        n_iterations: usize,
        confidence: f64,
    ) -> StatisticalResult {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        if sample1.is_empty() || sample2.is_empty() {
            return StatisticalResult {
                statistic: 0.0,
                p_value: None,
                confidence_interval: None,
                effect_size: None,
            };
        }

        let mut rng = thread_rng();
        let mut differences = Vec::with_capacity(n_iterations);

        for _ in 0..n_iterations {
            // Bootstrap resample
            let boot1: Vec<f64> = (0..sample1.len())
                .map(|_| *sample1.choose(&mut rng).unwrap())
                .collect();
            let boot2: Vec<f64> = (0..sample2.len())
                .map(|_| *sample2.choose(&mut rng).unwrap())
                .collect();

            let mean1 = boot1.iter().sum::<f64>() / boot1.len() as f64;
            let mean2 = boot2.iter().sum::<f64>() / boot2.len() as f64;
            differences.push(mean1 - mean2);
        }

        differences.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mean_diff = sample1.mean() - sample2.mean();
        let alpha = (1.0 - confidence) / 2.0;
        let lower_idx = (n_iterations as f64 * alpha) as usize;
        let upper_idx = (n_iterations as f64 * (1.0 - alpha)) as usize;

        StatisticalResult {
            statistic: mean_diff,
            p_value: None,
            confidence_interval: Some((differences[lower_idx], differences[upper_idx])),
            effect_size: Some(Self::cohens_d(sample1, sample2)),
        }
    }

    /// Calculate effect size (Cohen's d)
    pub fn cohens_d(sample1: &[f64], sample2: &[f64]) -> f64 {
        if sample1.len() < 2 || sample2.len() < 2 {
            return 0.0;
        }

        let mean1 = sample1.mean();
        let mean2 = sample2.mean();
        let var1 = sample1.variance();
        let var2 = sample2.variance();
        let n1 = sample1.len() as f64;
        let n2 = sample2.len() as f64;

        let pooled_std = (((n1 - 1.0) * var1 + (n2 - 1.0) * var2) / (n1 + n2 - 2.0)).sqrt();

        if pooled_std == 0.0 {
            return 0.0;
        }

        (mean1 - mean2) / pooled_std
    }

    /// Normal CDF approximation
    fn normal_cdf(x: f64) -> f64 {
        0.5 * (1.0 + Self::erf(x / 2_f64.sqrt()))
    }

    /// Error function approximation
    fn erf(x: f64) -> f64 {
        let sign = x.signum();
        let x = x.abs();

        let a1 = 0.254829592;
        let a2 = -0.284496736;
        let a3 = 1.421413741;
        let a4 = -1.453152027;
        let a5 = 1.061405429;
        let p = 0.3275911;

        let t = 1.0 / (1.0 + p * x);
        let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

        sign * y
    }
}
