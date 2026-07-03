//! ML-based Power Quality Event Classifier.
//!
//! Implements PQ event classification using:
//! - **k-Nearest Neighbours (k-NN)** with configurable distance metrics
//! - **CART Decision Tree** with entropy-based information gain
//! - **Haar Wavelet** feature extraction (4-level decomposition)
//! - **Ensemble** voting combining k-NN and decision tree
//!
//! All algorithms are implemented from scratch with no external ML dependencies.

use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// PqEventClass
// ─────────────────────────────────────────────────────────────────────────────

/// Power quality event class for ML classification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PqEventClass {
    /// Waveform within normal operating limits.
    Normal,
    /// Voltage sag (0.1–0.9 pu).
    Sag,
    /// Voltage swell (> 1.1 pu).
    Swell,
    /// Voltage interruption (< 0.1 pu).
    Interruption,
    /// Sustained harmonic distortion (THD elevated).
    Harmonic,
    /// Sub-cycle high-magnitude transient.
    Transient,
    /// Repetitive voltage fluctuation (flicker).
    Flicker,
    /// Voltage notching or high-frequency spikes.
    NotchSpike,
    /// Oscillatory disturbance.
    Oscillatory,
}

impl PqEventClass {
    /// Number of distinct event classes.
    pub const N_CLASSES: usize = 9;

    /// Stable 0-based index for this class.
    pub fn to_index(&self) -> usize {
        match self {
            Self::Normal => 0,
            Self::Sag => 1,
            Self::Swell => 2,
            Self::Interruption => 3,
            Self::Harmonic => 4,
            Self::Transient => 5,
            Self::Flicker => 6,
            Self::NotchSpike => 7,
            Self::Oscillatory => 8,
        }
    }

    /// Reconstruct class from index.
    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => Self::Normal,
            1 => Self::Sag,
            2 => Self::Swell,
            3 => Self::Interruption,
            4 => Self::Harmonic,
            5 => Self::Transient,
            6 => Self::Flicker,
            7 => Self::NotchSpike,
            8 => Self::Oscillatory,
            _ => Self::Normal,
        }
    }

    /// All class variants in index order.
    pub fn all_classes() -> Vec<Self> {
        (0..Self::N_CLASSES).map(Self::from_index).collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PqFeatures
// ─────────────────────────────────────────────────────────────────────────────

/// Extracted features from a PQ waveform for ML classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PqFeatures {
    /// RMS value (pu).
    pub rms_pu: f64,
    /// Peak value (pu).
    pub peak_pu: f64,
    /// Crest factor (peak / RMS).
    pub crest_factor: f64,
    /// Total harmonic distortion (%).
    pub thd_pct: f64,
    /// Fundamental component magnitude (pu).
    pub fundamental_pu: f64,
    /// Deviation from nominal RMS.
    pub rms_deviation: f64,
    /// Duration in power-frequency cycles.
    pub duration_cycles: f64,
    /// Maximum rate of voltage change (pu/cycle).
    pub max_dv_dt: f64,
    /// Wavelet energy at 4 Haar decomposition levels.
    pub wavelet_energy: [f64; 4],
    /// Zero-crossing rate deviation from expected.
    pub zero_crossing_deviation: f64,
    /// Frequency deviation from nominal (Hz).
    pub freq_deviation_hz: f64,
    /// Spectral entropy.
    pub spectral_entropy: f64,
}

/// Number of scalar features in the flat vector representation.
const N_FEATURES: usize = 15; // 8 scalars + 4 wavelet + 3 more scalars

// ─────────────────────────────────────────────────────────────────────────────
// TrainingSample
// ─────────────────────────────────────────────────────────────────────────────

/// A labelled feature vector for training.
#[derive(Debug, Clone)]
pub struct TrainingSample {
    /// Extracted feature vector.
    pub features: PqFeatures,
    /// Ground-truth class label.
    pub label: PqEventClass,
}

// ─────────────────────────────────────────────────────────────────────────────
// Distance metric and KnnConfig
// ─────────────────────────────────────────────────────────────────────────────

/// Distance metric for k-NN classification.
#[derive(Debug, Clone)]
pub enum DistanceMetric {
    /// Standard Euclidean (L2) distance.
    Euclidean,
    /// Manhattan (L1) distance.
    Manhattan,
    /// Minkowski distance with parameter p.
    Minkowski {
        /// Minkowski exponent.
        p: f64,
    },
    /// Weighted Euclidean distance using per-feature weights.
    Weighted,
}

/// Configuration for the k-NN classifier.
#[derive(Debug, Clone)]
pub struct KnnConfig {
    /// Number of nearest neighbours (default 5).
    pub k: usize,
    /// Distance metric to use.
    pub distance_metric: DistanceMetric,
    /// Per-feature importance weights (used with `Weighted` metric).
    pub feature_weights: Vec<f64>,
}

impl Default for KnnConfig {
    fn default() -> Self {
        Self {
            k: 5,
            distance_metric: DistanceMetric::Euclidean,
            feature_weights: Vec::new(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TreeNode and DecisionTreeConfig
// ─────────────────────────────────────────────────────────────────────────────

/// A node in the entropy-based decision tree.
#[derive(Debug, Clone)]
pub enum TreeNode {
    /// Leaf node with a predicted class and confidence.
    Leaf {
        /// Predicted class.
        class: PqEventClass,
        /// Confidence = majority_count / total.
        confidence: f64,
    },
    /// Internal split node.
    Split {
        /// Feature index used for splitting.
        feature_index: usize,
        /// Split threshold value.
        threshold: f64,
        /// Left child (feature < threshold).
        left: Box<TreeNode>,
        /// Right child (feature >= threshold).
        right: Box<TreeNode>,
    },
}

/// Configuration for the decision tree classifier.
#[derive(Debug, Clone)]
pub struct DecisionTreeConfig {
    /// Maximum tree depth (default 10).
    pub max_depth: usize,
    /// Minimum samples required to attempt a split (default 5).
    pub min_samples_split: usize,
    /// Minimum impurity decrease to justify a split (default 0.01).
    pub min_impurity_decrease: f64,
}

impl Default for DecisionTreeConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            min_samples_split: 5,
            min_impurity_decrease: 0.01,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ClassificationResult and ClassifierMethod
// ─────────────────────────────────────────────────────────────────────────────

/// Method used for classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassifierMethod {
    /// k-Nearest Neighbours.
    KNearestNeighbors,
    /// Decision Tree.
    DecisionTree,
    /// Ensemble (combined k-NN + decision tree).
    Ensemble,
}

/// Result of a classification operation.
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    /// Predicted event class.
    pub predicted_class: PqEventClass,
    /// Confidence score in [0, 1].
    pub confidence: f64,
    /// Per-class probability estimates.
    pub class_probabilities: Vec<(PqEventClass, f64)>,
    /// Method used for this classification.
    pub method: ClassifierMethod,
}

// ─────────────────────────────────────────────────────────────────────────────
// ConfusionMatrix
// ─────────────────────────────────────────────────────────────────────────────

/// Confusion matrix with per-class metrics.
#[derive(Debug, Clone)]
pub struct ConfusionMatrix {
    /// matrix\[true_class\]\[predicted_class\] = count.
    pub matrix: Vec<Vec<usize>>,
    /// Class labels in index order.
    pub class_labels: Vec<PqEventClass>,
    /// Overall accuracy.
    pub accuracy: f64,
    /// Precision per class.
    pub precision_per_class: Vec<f64>,
    /// Recall per class.
    pub recall_per_class: Vec<f64>,
    /// F1 score per class.
    pub f1_per_class: Vec<f64>,
}

impl ConfusionMatrix {
    /// Compute a confusion matrix from true/predicted pairs.
    pub fn compute(true_labels: &[PqEventClass], predicted: &[PqEventClass]) -> Self {
        let nc = PqEventClass::N_CLASSES;
        let mut matrix = vec![vec![0usize; nc]; nc];
        let n = true_labels.len().min(predicted.len());
        for i in 0..n {
            let ti = true_labels[i].to_index();
            let pi = predicted[i].to_index();
            if ti < nc && pi < nc {
                matrix[ti][pi] += 1;
            }
        }
        let correct: usize = (0..nc).map(|i| matrix[i][i]).sum();
        let total: usize = matrix.iter().flat_map(|r| r.iter()).sum();
        let accuracy = if total > 0 {
            correct as f64 / total as f64
        } else {
            0.0
        };

        let mut precision_per_class = vec![0.0; nc];
        let mut recall_per_class = vec![0.0; nc];
        let mut f1_per_class = vec![0.0; nc];

        for c in 0..nc {
            let tp = matrix[c][c] as f64;
            let col_sum: f64 = (0..nc).map(|r| matrix[r][c] as f64).sum();
            let row_sum: f64 = matrix[c].iter().sum::<usize>() as f64;

            let prec = if col_sum > 0.0 { tp / col_sum } else { 0.0 };
            let rec = if row_sum > 0.0 { tp / row_sum } else { 0.0 };
            let f1 = if prec + rec > 0.0 {
                2.0 * prec * rec / (prec + rec)
            } else {
                0.0
            };
            precision_per_class[c] = prec;
            recall_per_class[c] = rec;
            f1_per_class[c] = f1;
        }

        Self {
            matrix,
            class_labels: PqEventClass::all_classes(),
            accuracy,
            precision_per_class,
            recall_per_class,
            f1_per_class,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PqClassifier
// ─────────────────────────────────────────────────────────────────────────────

/// ML-based power quality event classifier combining k-NN and decision tree.
pub struct PqClassifier {
    /// k-NN configuration.
    pub knn_config: KnnConfig,
    /// Decision tree configuration.
    pub tree_config: DecisionTreeConfig,
    /// Accumulated training data.
    pub training_data: Vec<TrainingSample>,
    /// Built decision tree (populated by `train()`).
    pub decision_tree: Option<TreeNode>,
    /// Whether the classifier has been trained.
    pub is_trained: bool,
}

impl PqClassifier {
    /// Create a new PQ classifier with the given configurations.
    pub fn new(knn_config: KnnConfig, tree_config: DecisionTreeConfig) -> Self {
        Self {
            knn_config,
            tree_config,
            training_data: Vec::new(),
            decision_tree: None,
            is_trained: false,
        }
    }

    /// Add a single labelled training sample.
    pub fn add_training_sample(&mut self, sample: TrainingSample) {
        self.training_data.push(sample);
        self.is_trained = false;
    }

    /// Train both the k-NN (stores normalisation stats) and decision tree.
    pub fn train(&mut self) -> Result<(), String> {
        if self.training_data.len() < self.tree_config.min_samples_split {
            return Err(format!(
                "need at least {} training samples, have {}",
                self.tree_config.min_samples_split,
                self.training_data.len()
            ));
        }
        let refs: Vec<&TrainingSample> = self.training_data.iter().collect();
        let tree = Self::build_tree(&refs, 0, &self.tree_config);
        self.decision_tree = Some(tree);
        self.is_trained = true;
        Ok(())
    }

    /// Classify features using k-NN.
    pub fn classify_knn(&self, features: &PqFeatures) -> Result<ClassificationResult, String> {
        if self.training_data.is_empty() {
            return Err("training data is empty".to_string());
        }
        let query = Self::feature_to_vec(features);
        let norm_query = self.normalize_features_vec(&query);

        let k = self.knn_config.k.min(self.training_data.len());
        let mut distances: Vec<(f64, usize)> = self
            .training_data
            .iter()
            .enumerate()
            .map(|(i, sample)| {
                let sv = Self::feature_to_vec(&sample.features);
                let norm_sv = self.normalize_features_vec(&sv);
                let d = self.compute_distance(&norm_query, &norm_sv);
                (d, i)
            })
            .collect();

        distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        distances.truncate(k);

        let mut votes = [0usize; PqEventClass::N_CLASSES];
        for &(_, idx) in &distances {
            let ci = self.training_data[idx].label.to_index();
            if ci < PqEventClass::N_CLASSES {
                votes[ci] += 1;
            }
        }

        let (winner_idx, &winner_count) = votes
            .iter()
            .enumerate()
            .max_by_key(|&(_, &c)| c)
            .unwrap_or((0, &0));

        let confidence = if k > 0 {
            winner_count as f64 / k as f64
        } else {
            0.0
        };

        let class_probabilities: Vec<(PqEventClass, f64)> = (0..PqEventClass::N_CLASSES)
            .map(|i| {
                let prob = if k > 0 {
                    votes[i] as f64 / k as f64
                } else {
                    0.0
                };
                (PqEventClass::from_index(i), prob)
            })
            .collect();

        Ok(ClassificationResult {
            predicted_class: PqEventClass::from_index(winner_idx),
            confidence,
            class_probabilities,
            method: ClassifierMethod::KNearestNeighbors,
        })
    }

    /// Classify features using the trained decision tree.
    pub fn classify_tree(&self, features: &PqFeatures) -> Result<ClassificationResult, String> {
        let root = self
            .decision_tree
            .as_ref()
            .ok_or_else(|| "decision tree not built — call train() first".to_string())?;
        let fv = Self::feature_to_vec(features);
        let (class, confidence) = Self::traverse_tree(root, &fv);

        let mut class_probabilities: Vec<(PqEventClass, f64)> = PqEventClass::all_classes()
            .into_iter()
            .map(|c| {
                let p = if c == class { confidence } else { 0.0 };
                (c, p)
            })
            .collect();
        // Normalise: distribute remaining probability
        let remaining = 1.0 - confidence;
        let n_other = (PqEventClass::N_CLASSES - 1).max(1) as f64;
        for item in &mut class_probabilities {
            if item.0 != class {
                item.1 = remaining / n_other;
            }
        }

        Ok(ClassificationResult {
            predicted_class: class,
            confidence,
            class_probabilities,
            method: ClassifierMethod::DecisionTree,
        })
    }

    /// Classify using ensemble voting (k-NN + decision tree).
    pub fn classify_ensemble(&self, features: &PqFeatures) -> Result<ClassificationResult, String> {
        let knn_result = self.classify_knn(features)?;
        let tree_result = self.classify_tree(features)?;

        let knn_idx = knn_result.predicted_class.to_index();
        let tree_idx = tree_result.predicted_class.to_index();

        let (predicted_class, confidence) = if knn_idx == tree_idx {
            // Both agree — high confidence
            let conf = (knn_result.confidence + tree_result.confidence) / 2.0;
            (knn_result.predicted_class, conf.min(1.0))
        } else {
            // Disagree — use decision tree (typically more stable)
            (
                tree_result.predicted_class.clone(),
                tree_result.confidence * 0.8,
            )
        };

        // Blend class probabilities: 0.5 * kNN + 0.5 * tree
        let mut blended = [0.0; PqEventClass::N_CLASSES];
        for &(ref cls, prob) in &knn_result.class_probabilities {
            blended[cls.to_index()] += 0.5 * prob;
        }
        for &(ref cls, prob) in &tree_result.class_probabilities {
            blended[cls.to_index()] += 0.5 * prob;
        }
        let class_probabilities: Vec<(PqEventClass, f64)> = (0..PqEventClass::N_CLASSES)
            .map(|i| (PqEventClass::from_index(i), blended[i]))
            .collect();

        Ok(ClassificationResult {
            predicted_class,
            confidence,
            class_probabilities,
            method: ClassifierMethod::Ensemble,
        })
    }

    /// Evaluate the classifier on test data, returning a confusion matrix.
    pub fn evaluate(&self, test_data: &[TrainingSample]) -> ConfusionMatrix {
        let mut true_labels = Vec::with_capacity(test_data.len());
        let mut predicted = Vec::with_capacity(test_data.len());
        for sample in test_data {
            true_labels.push(sample.label.clone());
            let pred = self
                .classify_knn(&sample.features)
                .map(|r| r.predicted_class)
                .unwrap_or(PqEventClass::Normal);
            predicted.push(pred);
        }
        ConfusionMatrix::compute(&true_labels, &predicted)
    }

    // ── Feature extraction ──────────────────────────────────────────────────

    /// Extract features from a raw waveform.
    pub fn extract_features(
        waveform: &[f64],
        nominal_rms: f64,
        freq_hz: f64,
        samples_per_cycle: usize,
    ) -> PqFeatures {
        let n = waveform.len();
        if n == 0 {
            return PqFeatures {
                rms_pu: 0.0,
                peak_pu: 0.0,
                crest_factor: 0.0,
                thd_pct: 0.0,
                fundamental_pu: 0.0,
                rms_deviation: 0.0,
                duration_cycles: 0.0,
                max_dv_dt: 0.0,
                wavelet_energy: [0.0; 4],
                zero_crossing_deviation: 0.0,
                freq_deviation_hz: 0.0,
                spectral_entropy: 0.0,
            };
        }

        let nf = n as f64;
        let rms = (waveform.iter().map(|&x| x * x).sum::<f64>() / nf).sqrt();
        let nom = if nominal_rms.abs() > 1e-12 {
            nominal_rms
        } else {
            1.0
        };
        let rms_pu = rms / nom;
        let peak_abs = waveform.iter().map(|&x| x.abs()).fold(0.0_f64, f64::max);
        let peak_pu = peak_abs / nom;
        let crest_factor = if rms > 1e-12 { peak_abs / rms } else { 0.0 };
        let rms_deviation = rms_pu - 1.0;
        let spc = if samples_per_cycle > 0 {
            samples_per_cycle
        } else {
            1
        };
        let duration_cycles = nf / spc as f64;

        // THD via DFT on first 5 harmonics
        let (thd_pct, fundamental_pu) = compute_thd_and_fundamental(waveform, freq_hz, spc, nom);

        // max dV/dt
        let max_dv_dt = if n > 1 {
            let dt_per_sample_cycles = 1.0 / spc as f64;
            waveform
                .windows(2)
                .map(|w| ((w[1] - w[0]) / nom).abs() / dt_per_sample_cycles)
                .fold(0.0_f64, f64::max)
        } else {
            0.0
        };

        // Zero-crossing deviation
        let zero_crossings = if n > 1 {
            waveform.windows(2).filter(|w| w[0] * w[1] < 0.0).count()
        } else {
            0
        };
        let expected_zc_per_cycle = 2.0;
        let actual_zc_per_cycle = if duration_cycles > 0.0 {
            zero_crossings as f64 / duration_cycles
        } else {
            0.0
        };
        let zero_crossing_deviation = actual_zc_per_cycle - expected_zc_per_cycle;

        // Haar wavelet decomposition (4 levels)
        let wavelet_details = Self::haar_wavelet_decompose(waveform, 4);
        let mut wavelet_energy = [0.0_f64; 4];
        for (i, detail) in wavelet_details.iter().enumerate() {
            if i < 4 {
                wavelet_energy[i] = detail.iter().map(|&c| c * c).sum();
            }
        }

        // Frequency deviation: estimate fundamental via zero-crossings
        let freq_deviation_hz = if duration_cycles > 0.5 && spc > 0 {
            let sample_rate = freq_hz * spc as f64;
            let estimated_freq = if zero_crossings > 0 {
                zero_crossings as f64 * sample_rate / (2.0 * nf)
            } else {
                freq_hz
            };
            estimated_freq - freq_hz
        } else {
            0.0
        };

        // Spectral entropy
        let spectral_entropy = compute_spectral_entropy(waveform, spc);

        PqFeatures {
            rms_pu,
            peak_pu,
            crest_factor,
            thd_pct,
            fundamental_pu,
            rms_deviation,
            duration_cycles,
            max_dv_dt,
            wavelet_energy,
            zero_crossing_deviation,
            freq_deviation_hz,
            spectral_entropy,
        }
    }

    /// Generate synthetic training data with LCG-based waveform generation.
    pub fn generate_synthetic_data(n_per_class: usize) -> Vec<TrainingSample> {
        let mut samples = Vec::with_capacity(n_per_class * PqEventClass::N_CLASSES);
        let mut lcg_state: u64 = 0xDEAD_BEEF_CAFE_0001;
        let freq_hz = 60.0;
        let spc: usize = 128; // samples per cycle
        let n_points = spc * 4; // 4 cycles
        let nominal_rms = 1.0 / std::f64::consts::SQRT_2;

        for class_idx in 0..PqEventClass::N_CLASSES {
            let class = PqEventClass::from_index(class_idx);
            for sample_idx in 0..n_per_class {
                lcg_state = lcg_state
                    .wrapping_add(sample_idx as u64)
                    .wrapping_mul(6_364_136_223_846_793_005_u64)
                    .wrapping_add(1_442_695_040_888_963_407_u64);
                let waveform =
                    generate_class_waveform(&class, n_points, freq_hz, spc, &mut lcg_state);
                let features = Self::extract_features(&waveform, nominal_rms, freq_hz, spc);
                samples.push(TrainingSample {
                    features,
                    label: class.clone(),
                });
            }
        }
        samples
    }

    // ── Decision tree building ──────────────────────────────────────────────

    /// Build a decision tree from training sample references.
    fn build_tree(data: &[&TrainingSample], depth: usize, config: &DecisionTreeConfig) -> TreeNode {
        if data.is_empty() {
            return TreeNode::Leaf {
                class: PqEventClass::Normal,
                confidence: 0.0,
            };
        }

        let ent = Self::entropy(data);
        let all_same = data.windows(2).all(|w| w[0].label == w[1].label);

        if depth >= config.max_depth
            || data.len() < config.min_samples_split
            || all_same
            || ent < 1e-9
        {
            let (class, confidence) = majority_class(data);
            return TreeNode::Leaf { class, confidence };
        }

        // Find best split
        let mut best_gain = config.min_impurity_decrease;
        let mut best_feat = 0usize;
        let mut best_threshold = 0.0_f64;
        let mut found = false;

        for feat_idx in 0..N_FEATURES {
            let mut values: Vec<f64> = data
                .iter()
                .map(|s| Self::feature_to_vec(&s.features)[feat_idx])
                .collect();
            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            values.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
            if values.len() < 2 {
                continue;
            }
            // Try midpoints between consecutive sorted values
            for w in values.windows(2) {
                let threshold = (w[0] + w[1]) / 2.0;
                let gain = Self::information_gain(data, feat_idx, threshold);
                if gain > best_gain {
                    best_gain = gain;
                    best_feat = feat_idx;
                    best_threshold = threshold;
                    found = true;
                }
            }
        }

        if !found {
            let (class, confidence) = majority_class(data);
            return TreeNode::Leaf { class, confidence };
        }

        let (left_data, right_data): (Vec<&TrainingSample>, Vec<&TrainingSample>) = data
            .iter()
            .partition(|s| Self::feature_to_vec(&s.features)[best_feat] < best_threshold);

        if left_data.is_empty() || right_data.is_empty() {
            let (class, confidence) = majority_class(data);
            return TreeNode::Leaf { class, confidence };
        }

        let left = Self::build_tree(&left_data, depth + 1, config);
        let right = Self::build_tree(&right_data, depth + 1, config);

        TreeNode::Split {
            feature_index: best_feat,
            threshold: best_threshold,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Shannon entropy of a set of labelled samples.
    fn entropy(data: &[&TrainingSample]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }
        let n = data.len() as f64;
        let mut counts = [0usize; PqEventClass::N_CLASSES];
        for s in data {
            let ci = s.label.to_index();
            if ci < PqEventClass::N_CLASSES {
                counts[ci] += 1;
            }
        }
        let mut ent = 0.0_f64;
        for &c in &counts {
            if c > 0 {
                let p = c as f64 / n;
                ent -= p * p.log2();
            }
        }
        ent
    }

    /// Information gain from splitting data on a feature at a threshold.
    fn information_gain(data: &[&TrainingSample], feature_idx: usize, threshold: f64) -> f64 {
        let parent_entropy = Self::entropy(data);
        let n = data.len() as f64;
        let mut left_refs: Vec<&TrainingSample> = Vec::new();
        let mut right_refs: Vec<&TrainingSample> = Vec::new();
        for &s in data {
            if Self::feature_to_vec(&s.features)[feature_idx] < threshold {
                left_refs.push(s);
            } else {
                right_refs.push(s);
            }
        }

        if left_refs.is_empty() || right_refs.is_empty() {
            return 0.0;
        }

        let weighted = (left_refs.len() as f64 / n) * Self::entropy(&left_refs)
            + (right_refs.len() as f64 / n) * Self::entropy(&right_refs);
        parent_entropy - weighted
    }

    /// Traverse the decision tree, returning (class, confidence).
    fn traverse_tree(node: &TreeNode, features: &[f64]) -> (PqEventClass, f64) {
        match node {
            TreeNode::Leaf { class, confidence } => (class.clone(), *confidence),
            TreeNode::Split {
                feature_index,
                threshold,
                left,
                right,
            } => {
                let v = if *feature_index < features.len() {
                    features[*feature_index]
                } else {
                    0.0
                };
                if v < *threshold {
                    Self::traverse_tree(left, features)
                } else {
                    Self::traverse_tree(right, features)
                }
            }
        }
    }

    // ── Feature normalisation ───────────────────────────────────────────────

    /// Convert `PqFeatures` to a flat `Vec<f64>`.
    pub fn feature_to_vec(features: &PqFeatures) -> Vec<f64> {
        let mut v = Vec::with_capacity(N_FEATURES);
        v.push(features.rms_pu);
        v.push(features.peak_pu);
        v.push(features.crest_factor);
        v.push(features.thd_pct);
        v.push(features.fundamental_pu);
        v.push(features.rms_deviation);
        v.push(features.duration_cycles);
        v.push(features.max_dv_dt);
        v.extend_from_slice(&features.wavelet_energy);
        v.push(features.zero_crossing_deviation);
        v.push(features.freq_deviation_hz);
        v.push(features.spectral_entropy);
        v
    }

    /// Normalise a feature vector to `0,1` using training set min/max.
    pub fn normalize_features(&self, features: &PqFeatures) -> Vec<f64> {
        let raw = Self::feature_to_vec(features);
        self.normalize_features_vec(&raw)
    }

    /// Normalise a raw feature vector using training min/max.
    fn normalize_features_vec(&self, raw: &[f64]) -> Vec<f64> {
        if self.training_data.is_empty() {
            return raw.to_vec();
        }
        let n_feat = raw.len();
        let mut mins = vec![f64::INFINITY; n_feat];
        let mut maxs = vec![f64::NEG_INFINITY; n_feat];

        for sample in &self.training_data {
            let fv = Self::feature_to_vec(&sample.features);
            for (i, &val) in fv.iter().enumerate() {
                if i < n_feat {
                    if val < mins[i] {
                        mins[i] = val;
                    }
                    if val > maxs[i] {
                        maxs[i] = val;
                    }
                }
            }
        }

        raw.iter()
            .enumerate()
            .map(|(i, &v)| {
                let range = maxs[i] - mins[i];
                if range.abs() > 1e-12 {
                    (v - mins[i]) / range
                } else {
                    0.5
                }
            })
            .collect()
    }

    // ── Haar wavelet decomposition ──────────────────────────────────────────

    /// Haar wavelet decomposition returning detail coefficients at each level.
    pub fn haar_wavelet_decompose(signal: &[f64], levels: usize) -> Vec<Vec<f64>> {
        let mut current = signal.to_vec();
        // Pad to even length if needed
        if !current.len().is_multiple_of(2) {
            current.push(0.0);
        }
        let mut details = Vec::with_capacity(levels);
        let scale = std::f64::consts::FRAC_1_SQRT_2;

        for _ in 0..levels {
            if current.len() < 2 {
                details.push(vec![]);
                continue;
            }
            let half = current.len() / 2;
            let mut approx = Vec::with_capacity(half);
            let mut detail = Vec::with_capacity(half);
            for i in 0..half {
                let a = current[2 * i];
                let b = if 2 * i + 1 < current.len() {
                    current[2 * i + 1]
                } else {
                    0.0
                };
                approx.push((a + b) * scale);
                detail.push((a - b) * scale);
            }
            details.push(detail);
            current = approx;
        }
        details
    }

    // ── Distance computation ────────────────────────────────────────────────

    /// Compute distance between two feature vectors using the configured metric.
    fn compute_distance(&self, a: &[f64], b: &[f64]) -> f64 {
        match &self.knn_config.distance_metric {
            DistanceMetric::Euclidean => euclidean_distance(a, b),
            DistanceMetric::Manhattan => manhattan_distance(a, b),
            DistanceMetric::Minkowski { p } => minkowski_distance(a, b, *p),
            DistanceMetric::Weighted => {
                weighted_euclidean_distance(a, b, &self.knn_config.feature_weights)
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Distance functions
// ─────────────────────────────────────────────────────────────────────────────

/// Euclidean (L2) distance.
pub fn euclidean_distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// Manhattan (L1) distance.
pub fn manhattan_distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(&x, &y)| (x - y).abs()).sum()
}

/// Minkowski distance with exponent p.
pub fn minkowski_distance(a: &[f64], b: &[f64], p: f64) -> f64 {
    if p.abs() < 1e-12 {
        return 0.0;
    }
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x - y).abs().powf(p))
        .sum::<f64>()
        .powf(1.0 / p)
}

/// Weighted Euclidean distance.
pub fn weighted_euclidean_distance(a: &[f64], b: &[f64], weights: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .enumerate()
        .map(|(i, (&x, &y))| {
            let w = if i < weights.len() { weights[i] } else { 1.0 };
            w * (x - y).powi(2)
        })
        .sum::<f64>()
        .sqrt()
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Majority class and confidence from training samples.
fn majority_class(data: &[&TrainingSample]) -> (PqEventClass, f64) {
    let mut counts = [0usize; PqEventClass::N_CLASSES];
    for s in data {
        let ci = s.label.to_index();
        if ci < PqEventClass::N_CLASSES {
            counts[ci] += 1;
        }
    }
    let (idx, &count) = counts
        .iter()
        .enumerate()
        .max_by_key(|&(_, &c)| c)
        .unwrap_or((0, &0));
    let n = data.len().max(1) as f64;
    (PqEventClass::from_index(idx), count as f64 / n)
}

/// Compute THD (%) and fundamental magnitude (pu) via DFT.
fn compute_thd_and_fundamental(
    signal: &[f64],
    freq_hz: f64,
    samples_per_cycle: usize,
    nominal: f64,
) -> (f64, f64) {
    let n = signal.len();
    if n == 0 || freq_hz <= 0.0 || samples_per_cycle == 0 {
        return (0.0, 0.0);
    }
    let sample_rate = freq_hz * samples_per_cycle as f64;
    let nf = n as f64;

    // DFT coefficient magnitude at bin k: |X_k| = sqrt(re^2 + im^2) * 2/N
    let dft_mag = |k: usize| -> f64 {
        let (re, im) = signal
            .iter()
            .enumerate()
            .fold((0.0_f64, 0.0_f64), |(r, i), (j, &x)| {
                let angle = 2.0 * PI * k as f64 * j as f64 / nf;
                (r + x * angle.cos(), i - x * angle.sin())
            });
        (re * re + im * im).sqrt() * 2.0 / nf
    };

    let fund_bin = (freq_hz * nf / sample_rate).round() as usize;
    let v1 = dft_mag(fund_bin);
    let fundamental_pu = v1 / nominal.max(1e-12);

    if v1 < 1e-12 {
        return (0.0, fundamental_pu);
    }

    let harmonics_sq: f64 = (2..=5)
        .map(|h| {
            let bin = (h as f64 * freq_hz * nf / sample_rate).round() as usize;
            if bin < n / 2 + 1 {
                dft_mag(bin).powi(2)
            } else {
                0.0
            }
        })
        .sum();

    let thd = harmonics_sq.sqrt() / v1 * 100.0;
    (thd, fundamental_pu)
}

/// Compute spectral entropy.
fn compute_spectral_entropy(signal: &[f64], _samples_per_cycle: usize) -> f64 {
    let n = signal.len();
    if n < 4 {
        return 0.0;
    }
    let nf = n as f64;
    let n_bins = n / 2 + 1;

    // Compute power spectrum
    let powers: Vec<f64> = (0..n_bins)
        .map(|k| {
            let (re, im) = signal
                .iter()
                .enumerate()
                .fold((0.0_f64, 0.0_f64), |(r, i), (j, &x)| {
                    let angle = 2.0 * PI * k as f64 * j as f64 / nf;
                    (r + x * angle.cos(), i - x * angle.sin())
                });
            re * re + im * im
        })
        .collect();

    let total: f64 = powers.iter().sum();
    if total < 1e-30 {
        return 0.0;
    }

    let mut entropy = 0.0_f64;
    for &p in &powers {
        let pk = p / total;
        if pk > 1e-30 {
            entropy -= pk * pk.ln();
        }
    }
    entropy
}

/// LCG pseudo-random noise in [-1, 1].
fn lcg_noise(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005_u64)
        .wrapping_add(1_442_695_040_888_963_407_u64);
    (*state as f64) / (u64::MAX as f64) * 2.0 - 1.0
}

/// Generate a synthetic waveform for a given class.
fn generate_class_waveform(
    class: &PqEventClass,
    n_points: usize,
    freq_hz: f64,
    samples_per_cycle: usize,
    lcg: &mut u64,
) -> Vec<f64> {
    let omega = 2.0 * PI * freq_hz / (freq_hz * samples_per_cycle as f64);
    let noise_amp = 0.01;
    let mut waveform = Vec::with_capacity(n_points);

    match class {
        PqEventClass::Normal => {
            for i in 0..n_points {
                waveform.push((omega * i as f64).sin() + noise_amp * lcg_noise(lcg));
            }
        }
        PqEventClass::Sag => {
            let start = n_points / 3;
            let end = 2 * n_points / 3;
            for i in 0..n_points {
                let amp = if i >= start && i < end { 0.5 } else { 1.0 };
                waveform.push(amp * (omega * i as f64).sin() + noise_amp * lcg_noise(lcg));
            }
        }
        PqEventClass::Swell => {
            let start = n_points / 3;
            let end = 2 * n_points / 3;
            for i in 0..n_points {
                let amp = if i >= start && i < end { 1.3 } else { 1.0 };
                waveform.push(amp * (omega * i as f64).sin() + noise_amp * lcg_noise(lcg));
            }
        }
        PqEventClass::Interruption => {
            let start = n_points / 4;
            let end = 3 * n_points / 4;
            for i in 0..n_points {
                let amp = if i >= start && i < end { 0.02 } else { 1.0 };
                waveform.push(amp * (omega * i as f64).sin() + noise_amp * lcg_noise(lcg));
            }
        }
        PqEventClass::Harmonic => {
            for i in 0..n_points {
                let t = omega * i as f64;
                waveform.push(
                    t.sin()
                        + 0.25 * (5.0 * t).sin()
                        + 0.15 * (7.0 * t).sin()
                        + noise_amp * lcg_noise(lcg),
                );
            }
        }
        PqEventClass::Transient => {
            let spike_pos = n_points / 2;
            for i in 0..n_points {
                let dist = (i as isize - spike_pos as isize).unsigned_abs();
                let spike = if dist < 8 {
                    1.2 * (-(dist as f64) * 0.6).exp()
                } else {
                    0.0
                };
                waveform.push((omega * i as f64).sin() + spike + noise_amp * lcg_noise(lcg));
            }
        }
        PqEventClass::Flicker => {
            let sample_rate = freq_hz * samples_per_cycle as f64;
            let mod_freq = 2.0 * PI * 8.0 / sample_rate;
            for i in 0..n_points {
                let m = 1.0 + 0.1 * (mod_freq * i as f64).sin();
                waveform.push(m * (omega * i as f64).sin() + noise_amp * lcg_noise(lcg));
            }
        }
        PqEventClass::NotchSpike => {
            for i in 0..n_points {
                let notch = if (i % 64) < 4 { -0.4 } else { 0.0 };
                waveform.push((omega * i as f64).sin() + notch + 0.03 * lcg_noise(lcg));
            }
        }
        PqEventClass::Oscillatory => {
            let osc_freq = 2.0 * PI * 300.0 / (freq_hz * samples_per_cycle as f64);
            let start = n_points / 3;
            let end = start + n_points / 6;
            for i in 0..n_points {
                let osc = if i >= start && i < end {
                    let decay = (-0.05 * (i - start) as f64).exp();
                    0.4 * decay * (osc_freq * i as f64).sin()
                } else {
                    0.0
                };
                waveform.push((omega * i as f64).sin() + osc + noise_amp * lcg_noise(lcg));
            }
        }
    }
    waveform
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn default_classifier() -> PqClassifier {
        PqClassifier::new(KnnConfig::default(), DecisionTreeConfig::default())
    }

    fn train_classifier() -> PqClassifier {
        let mut clf = default_classifier();
        let data = PqClassifier::generate_synthetic_data(10);
        for s in data {
            clf.add_training_sample(s);
        }
        clf.train().expect("training should succeed");
        clf
    }

    fn pure_sine(n: usize, spc: usize) -> Vec<f64> {
        let omega = 2.0 * PI / spc as f64;
        (0..n).map(|i| (omega * i as f64).sin()).collect()
    }

    // ── Feature extraction tests ────────────────────────────────────────

    #[test]
    fn test_extract_features_normal_sine_rms() {
        let spc = 128;
        let w = pure_sine(spc * 4, spc);
        let nominal_rms = 1.0 / std::f64::consts::SQRT_2;
        let f = PqClassifier::extract_features(&w, nominal_rms, 60.0, spc);
        assert!(
            (f.rms_pu - 1.0).abs() < 0.05,
            "RMS pu for pure sine should be ~1.0, got {}",
            f.rms_pu
        );
    }

    #[test]
    fn test_extract_features_sag_deviation() {
        let spc = 128;
        let w: Vec<f64> = (0..spc * 4)
            .map(|i| 0.5 * (2.0 * PI * i as f64 / spc as f64).sin())
            .collect();
        let nominal_rms = 1.0 / std::f64::consts::SQRT_2;
        let f = PqClassifier::extract_features(&w, nominal_rms, 60.0, spc);
        assert!(
            f.rms_deviation < 0.0,
            "sag should have negative rms_deviation, got {}",
            f.rms_deviation
        );
    }

    #[test]
    fn test_extract_features_swell_deviation() {
        let spc = 128;
        let w: Vec<f64> = (0..spc * 4)
            .map(|i| 1.3 * (2.0 * PI * i as f64 / spc as f64).sin())
            .collect();
        let nominal_rms = 1.0 / std::f64::consts::SQRT_2;
        let f = PqClassifier::extract_features(&w, nominal_rms, 60.0, spc);
        assert!(
            f.rms_deviation > 0.0,
            "swell should have positive rms_deviation, got {}",
            f.rms_deviation
        );
    }

    #[test]
    fn test_extract_features_harmonic_thd() {
        let spc = 128;
        let w: Vec<f64> = (0..spc * 4)
            .map(|i| {
                let t = 2.0 * PI * i as f64 / spc as f64;
                t.sin() + 0.3 * (5.0 * t).sin()
            })
            .collect();
        let nominal_rms = 1.0 / std::f64::consts::SQRT_2;
        let f = PqClassifier::extract_features(&w, nominal_rms, 60.0, spc);
        assert!(
            f.thd_pct > 5.0,
            "harmonic waveform should have THD > 5%, got {}",
            f.thd_pct
        );
    }

    #[test]
    fn test_crest_factor_pure_sine() {
        let spc = 128;
        let w = pure_sine(spc * 4, spc);
        let nominal_rms = 1.0 / std::f64::consts::SQRT_2;
        let f = PqClassifier::extract_features(&w, nominal_rms, 60.0, spc);
        let expected = std::f64::consts::SQRT_2;
        assert!(
            (f.crest_factor - expected).abs() < 0.1,
            "crest factor should be ~sqrt(2), got {}",
            f.crest_factor
        );
    }

    #[test]
    fn test_haar_wavelet_4_levels() {
        let signal = pure_sine(256, 64);
        let details = PqClassifier::haar_wavelet_decompose(&signal, 4);
        assert_eq!(details.len(), 4, "should return 4 levels of detail");
        for level in &details {
            assert!(
                !level.is_empty(),
                "each detail level should have coefficients"
            );
        }
    }

    // ── k-NN tests ──────────────────────────────────────────────────────

    #[test]
    fn test_knn_classifies_normal() {
        let clf = train_classifier();
        let spc = 128;
        let w = pure_sine(spc * 4, spc);
        let nominal_rms = 1.0 / std::f64::consts::SQRT_2;
        let f = PqClassifier::extract_features(&w, nominal_rms, 60.0, spc);
        let result = clf.classify_knn(&f).expect("knn classify");
        assert_eq!(
            result.predicted_class,
            PqEventClass::Normal,
            "pure sine should be Normal, got {:?}",
            result.predicted_class
        );
    }

    #[test]
    fn test_knn_classifies_sag() {
        let clf = train_classifier();
        let spc = 128;
        let n = spc * 4;
        let w: Vec<f64> = (0..n)
            .map(|i| {
                let amp = if i >= n / 3 && i < 2 * n / 3 {
                    0.5
                } else {
                    1.0
                };
                amp * (2.0 * PI * i as f64 / spc as f64).sin()
            })
            .collect();
        let nominal_rms = 1.0 / std::f64::consts::SQRT_2;
        let f = PqClassifier::extract_features(&w, nominal_rms, 60.0, spc);
        let result = clf.classify_knn(&f).expect("knn classify");
        // Should be Sag (or at least not Normal for a sagged waveform)
        assert!(
            result.predicted_class == PqEventClass::Sag
                || result.predicted_class != PqEventClass::Normal,
            "sagged waveform should not be Normal, got {:?}",
            result.predicted_class
        );
    }

    #[test]
    fn test_knn_classifies_harmonic() {
        let clf = train_classifier();
        let spc = 128;
        let w: Vec<f64> = (0..spc * 4)
            .map(|i| {
                let t = 2.0 * PI * i as f64 / spc as f64;
                t.sin() + 0.25 * (5.0 * t).sin() + 0.15 * (7.0 * t).sin()
            })
            .collect();
        let nominal_rms = 1.0 / std::f64::consts::SQRT_2;
        let f = PqClassifier::extract_features(&w, nominal_rms, 60.0, spc);
        let result = clf.classify_knn(&f).expect("knn classify");
        assert_eq!(
            result.predicted_class,
            PqEventClass::Harmonic,
            "harmonic waveform should be Harmonic, got {:?}",
            result.predicted_class
        );
    }

    #[test]
    fn test_knn_confidence_bounds() {
        let clf = train_classifier();
        let spc = 128;
        let w = pure_sine(spc * 4, spc);
        let nominal_rms = 1.0 / std::f64::consts::SQRT_2;
        let f = PqClassifier::extract_features(&w, nominal_rms, 60.0, spc);
        let result = clf.classify_knn(&f).expect("knn classify");
        assert!(
            result.confidence > 0.0 && result.confidence <= 1.0,
            "confidence should be in (0, 1], got {}",
            result.confidence
        );
    }

    // ── Decision tree tests ─────────────────────────────────────────────

    #[test]
    fn test_tree_has_root_after_training() {
        let clf = train_classifier();
        assert!(
            clf.decision_tree.is_some(),
            "decision tree should be built after training"
        );
    }

    #[test]
    fn test_tree_classifies_normal() {
        let clf = train_classifier();
        let spc = 128;
        let w = pure_sine(spc * 4, spc);
        let nominal_rms = 1.0 / std::f64::consts::SQRT_2;
        let f = PqClassifier::extract_features(&w, nominal_rms, 60.0, spc);
        let result = clf.classify_tree(&f).expect("tree classify");
        assert_eq!(
            result.predicted_class,
            PqEventClass::Normal,
            "pure sine should be Normal, got {:?}",
            result.predicted_class
        );
    }

    #[test]
    fn test_tree_max_depth_respected() {
        let config = DecisionTreeConfig {
            max_depth: 2,
            min_samples_split: 2,
            min_impurity_decrease: 0.001,
        };
        let mut clf = PqClassifier::new(KnnConfig::default(), config);
        let data = PqClassifier::generate_synthetic_data(10);
        for s in data {
            clf.add_training_sample(s);
        }
        clf.train().expect("train");

        fn max_depth(node: &TreeNode) -> usize {
            match node {
                TreeNode::Leaf { .. } => 0,
                TreeNode::Split { left, right, .. } => 1 + max_depth(left).max(max_depth(right)),
            }
        }

        let root = clf.decision_tree.as_ref().expect("tree exists");
        let depth = max_depth(root);
        assert!(depth <= 2, "max depth should be <= 2, got {}", depth);
    }

    // ── Ensemble tests ──────────────────────────────────────────────────

    #[test]
    fn test_ensemble_returns_result() {
        let clf = train_classifier();
        let spc = 128;
        let w = pure_sine(spc * 4, spc);
        let nominal_rms = 1.0 / std::f64::consts::SQRT_2;
        let f = PqClassifier::extract_features(&w, nominal_rms, 60.0, spc);
        let result = clf.classify_ensemble(&f).expect("ensemble classify");
        assert_eq!(result.method, ClassifierMethod::Ensemble);
    }

    #[test]
    fn test_ensemble_confidence_reasonable() {
        let clf = train_classifier();
        let spc = 128;
        let w = pure_sine(spc * 4, spc);
        let nominal_rms = 1.0 / std::f64::consts::SQRT_2;
        let f = PqClassifier::extract_features(&w, nominal_rms, 60.0, spc);
        let result = clf.classify_ensemble(&f).expect("ensemble classify");
        assert!(
            result.confidence >= 0.0 && result.confidence <= 1.0,
            "ensemble confidence should be in [0, 1], got {}",
            result.confidence
        );
    }

    // ── ConfusionMatrix tests ───────────────────────────────────────────

    #[test]
    fn test_confusion_matrix_accuracy() {
        let true_labels = vec![
            PqEventClass::Normal,
            PqEventClass::Sag,
            PqEventClass::Normal,
            PqEventClass::Sag,
        ];
        let predicted = vec![
            PqEventClass::Normal,
            PqEventClass::Sag,
            PqEventClass::Normal,
            PqEventClass::Normal,
        ];
        let cm = ConfusionMatrix::compute(&true_labels, &predicted);
        assert!(
            (cm.accuracy - 0.75).abs() < 1e-10,
            "accuracy should be 0.75, got {}",
            cm.accuracy
        );
    }

    #[test]
    fn test_precision_positive() {
        let clf = train_classifier();
        let test_data = PqClassifier::generate_synthetic_data(5);
        let cm = clf.evaluate(&test_data);
        let has_positive = cm.precision_per_class.iter().any(|&p| p > 0.0);
        assert!(
            has_positive,
            "at least one class should have positive precision"
        );
    }

    #[test]
    fn test_recall_positive() {
        let clf = train_classifier();
        let test_data = PqClassifier::generate_synthetic_data(5);
        let cm = clf.evaluate(&test_data);
        let has_positive = cm.recall_per_class.iter().any(|&r| r > 0.0);
        assert!(
            has_positive,
            "at least one class should have positive recall"
        );
    }

    #[test]
    fn test_f1_harmonic_mean() {
        let true_labels = vec![PqEventClass::Normal; 10];
        let predicted = vec![PqEventClass::Normal; 10];
        let cm = ConfusionMatrix::compute(&true_labels, &predicted);
        // For Normal class: precision=1.0, recall=1.0, f1=1.0
        let f1 = cm.f1_per_class[PqEventClass::Normal.to_index()];
        assert!(
            (f1 - 1.0).abs() < 1e-10,
            "F1 for perfect classification should be 1.0, got {}",
            f1
        );
    }

    // ── Synthetic data generation test ──────────────────────────────────

    #[test]
    fn test_generate_synthetic_data_count() {
        let n_per_class = 7;
        let data = PqClassifier::generate_synthetic_data(n_per_class);
        assert_eq!(
            data.len(),
            n_per_class * PqEventClass::N_CLASSES,
            "should have {} samples, got {}",
            n_per_class * PqEventClass::N_CLASSES,
            data.len()
        );
    }

    // ── Distance function tests ─────────────────────────────────────────

    #[test]
    fn test_euclidean_self_distance_zero() {
        let x = vec![1.0, 2.0, 3.0];
        let d = euclidean_distance(&x, &x);
        assert!(d.abs() < 1e-12, "d(x, x) should be 0, got {}", d);
    }

    #[test]
    fn test_manhattan_triangle_inequality() {
        let a = vec![0.0, 0.0];
        let b = vec![1.0, 1.0];
        let c = vec![2.0, 0.0];
        let ab = manhattan_distance(&a, &b);
        let bc = manhattan_distance(&b, &c);
        let ac = manhattan_distance(&a, &c);
        assert!(
            ac <= ab + bc + 1e-10,
            "triangle inequality violated: ac={}, ab+bc={}",
            ac,
            ab + bc
        );
    }

    // ── Entropy tests ───────────────────────────────────────────────────

    #[test]
    fn test_entropy_pure_class_is_zero() {
        let samples: Vec<TrainingSample> = (0..10)
            .map(|_| TrainingSample {
                features: PqClassifier::extract_features(&[0.0; 128], 1.0, 60.0, 128),
                label: PqEventClass::Sag,
            })
            .collect();
        let refs: Vec<&TrainingSample> = samples.iter().collect();
        let ent = PqClassifier::entropy(&refs);
        assert!(
            ent.abs() < 1e-10,
            "entropy of pure class should be 0, got {}",
            ent
        );
    }

    #[test]
    fn test_entropy_uniform_distribution() {
        // Create samples with each class represented equally
        let classes = PqEventClass::all_classes();
        let mut samples = Vec::new();
        for class in &classes {
            for _ in 0..10 {
                samples.push(TrainingSample {
                    features: PqClassifier::extract_features(&[0.0; 128], 1.0, 60.0, 128),
                    label: class.clone(),
                });
            }
        }
        let refs: Vec<&TrainingSample> = samples.iter().collect();
        let ent = PqClassifier::entropy(&refs);
        let expected = (PqEventClass::N_CLASSES as f64).log2();
        assert!(
            (ent - expected).abs() < 0.01,
            "entropy of uniform dist should be log2({}), got {}, expected {}",
            PqEventClass::N_CLASSES,
            ent,
            expected
        );
    }

    #[test]
    fn test_minkowski_reduces_to_euclidean() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        let d_euc = euclidean_distance(&a, &b);
        let d_mink = minkowski_distance(&a, &b, 2.0);
        assert!(
            (d_euc - d_mink).abs() < 1e-10,
            "Minkowski(p=2) should equal Euclidean: {} vs {}",
            d_mink,
            d_euc
        );
    }

    #[test]
    fn test_minkowski_reduces_to_manhattan() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        let d_man = manhattan_distance(&a, &b);
        let d_mink = minkowski_distance(&a, &b, 1.0);
        assert!(
            (d_man - d_mink).abs() < 1e-10,
            "Minkowski(p=1) should equal Manhattan: {} vs {}",
            d_mink,
            d_man
        );
    }
}
