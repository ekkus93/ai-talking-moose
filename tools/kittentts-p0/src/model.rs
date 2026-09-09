//! Minimal P0-only Kitten Mini ONNX adapter.
//!
//! The tensor shapes and voice-style selection follow the Apache-2.0
//! `kittentts-rs` 0.4.1 port, but this adapter owns ONNX Runtime configuration
//! directly so the probe can enforce exact Kitten framing and bounded CPU
//! thread counts without inheriting unrelated downloader/runtime dependencies.

use crate::{npz::load_npz, tokenize::ipa_to_ids};
use anyhow::{anyhow, Context, Result};
use ort::{session::Session, value::Tensor};
use std::{collections::HashMap, path::Path, sync::Mutex};

struct Voice {
    rows: usize,
    columns: usize,
    data: Vec<f32>,
}

impl Voice {
    fn style_row(&self, index: usize) -> &[f32] {
        let row = index.min(self.rows.saturating_sub(1));
        &self.data[row * self.columns..(row + 1) * self.columns]
    }
}

pub struct KittenModel {
    session: Mutex<Session>,
    voices: HashMap<String, Voice>,
}

impl KittenModel {
    pub fn load(model_path: &Path, voices_path: &Path, threads: usize) -> Result<Self> {
        let session = Session::builder()
            .context("failed to create ORT session builder")?
            .with_intra_threads(threads)
            .map_err(|error| anyhow!("failed to configure ORT intra-op threads: {error}"))?
            .with_inter_threads(1)
            .map_err(|error| anyhow!("failed to configure ORT inter-op threads: {error}"))?
            .commit_from_file(model_path)
            .with_context(|| format!("failed to load ONNX model {}", model_path.display()))?;

        let voices = load_npz(voices_path)?
            .into_iter()
            .map(|(name, array)| {
                let rows = array.nrows();
                let columns = array.ncols();
                (
                    name,
                    Voice {
                        rows,
                        columns,
                        data: array.data,
                    },
                )
            })
            .collect();

        Ok(Self {
            session: Mutex::new(session),
            voices,
        })
    }

    pub fn generate_from_ipa(
        &self,
        ipa: &str,
        voice: &str,
        speed: f32,
        style_index: usize,
    ) -> Result<Vec<f32>> {
        let ids = ipa_to_ids(ipa);
        let voice = self
            .voices
            .get(voice)
            .with_context(|| format!("voice {voice:?} is not present in voices.npz"))?;
        let style = voice.style_row(style_index);

        let input_ids = Tensor::<i64>::from_array(([1usize, ids.len()], ids))
            .context("failed to build input_ids tensor")?;
        let style_tensor = Tensor::<f32>::from_array(([1usize, style.len()], style.to_vec()))
            .context("failed to build style tensor")?;
        let speed_tensor = Tensor::<f32>::from_array(([1usize], vec![speed]))
            .context("failed to build speed tensor")?;

        let mut session = self.session.lock().expect("ORT session mutex poisoned");
        let outputs = session
            .run(ort::inputs![input_ids, style_tensor, speed_tensor])
            .context("Kitten ONNX inference failed")?;
        let (_shape, samples) = outputs[0]
            .try_extract_tensor::<f32>()
            .context("failed to extract Kitten audio tensor")?;
        Ok(samples.to_vec())
    }
}
