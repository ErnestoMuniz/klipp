use std::path::Path;

/// Áudio decodificado para f32 interleaved (symphonia).
pub struct Decoded {
    pub rate: u32,
    pub channels: usize,
    pub samples: Vec<f32>,
}

impl Decoded {
    pub fn duration_secs(&self) -> f32 {
        if self.rate == 0 || self.channels == 0 {
            return 0.0;
        }
        self.samples.len() as f32 / self.channels as f32 / self.rate as f32
    }

    /// Envelope estático para a forma de onda do player: pico (0–1) por
    /// balde, normalizado pelo pico global. Uma passada sobre as amostras,
    /// calculado uma vez por play — a UI só pinta, sem reamostrar nem
    /// mudar de forma com o áudio.
    pub fn peaks(&self, buckets: usize) -> Vec<f32> {
        if buckets == 0 {
            return vec![];
        }
        let channels = self.channels.max(1);
        let frames = self.samples.len() / channels;
        if frames == 0 {
            return vec![0.0; buckets];
        }
        let mut out = vec![0.0f32; buckets];
        for (i, frame) in self.samples.chunks_exact(channels).enumerate() {
            let mut v = 0.0f32;
            for s in frame {
                v = v.max(s.abs());
            }
            let b = (i * buckets / frames).min(buckets - 1);
            out[b] = out[b].max(v);
        }
        let max = out.iter().cloned().fold(0.0f32, f32::max);
        if max <= 0.0 {
            // Silêncio total: base mínima para não sumir.
            return vec![0.06; buckets];
        }
        for v in out.iter_mut() {
            *v = (*v / max).max(0.06);
        }
        out
    }
}

/// Decodifica o arquivo inteiro para f32 interleaved usando symphonia.
pub fn decode(path: &Path) -> anyhow::Result<Decoded> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| anyhow::anyhow!("sem trilha de áudio"))?;
    let params = track.codec_params.clone();
    let mut decoder = symphonia::default::get_codecs().make(&params, &DecoderOptions::default())?;
    let rate = params.sample_rate.unwrap_or(44100);

    let mut samples: Vec<f32> = Vec::new();
    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    let mut channels = 2usize;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(_))
            | Err(symphonia::core::errors::Error::ResetRequired)
            | Err(symphonia::core::errors::Error::DecodeError(_)) => break,
            Err(err) => return Err(err.into()),
        };
        let buffer = decoder.decode(&packet)?;
        channels = buffer.spec().channels.count().max(1);
        let buf = sample_buf.get_or_insert_with(|| {
            SampleBuffer::<f32>::new(buffer.frames() as u64, buffer.spec().clone())
        });
        buf.copy_interleaved_ref(buffer);
        samples.extend_from_slice(buf.samples());
    }

    if samples.is_empty() {
        return Err(anyhow::anyhow!("arquivo sem amostras"));
    }

    Ok(Decoded {
        rate,
        channels,
        samples,
    })
}

#[cfg(test)]
mod tests {
    use super::Decoded;

    fn decoded(samples: Vec<f32>, channels: usize) -> Decoded {
        Decoded {
            rate: 44100,
            channels,
            samples,
        }
    }

    #[test]
    fn picos_mapeiam_trechos_alto_e_baixo() {
        // Mono: metade silenciosa, metade no pico.
        let d = decoded(vec![0.0, 0.0, 1.0, 0.5], 1);
        assert_eq!(d.peaks(2), vec![0.06, 1.0]);
    }

    #[test]
    fn silencio_total_vira_base_minima() {
        let d = decoded(vec![0.0; 8], 2);
        assert_eq!(d.peaks(4), vec![0.06; 4]);
    }

    #[test]
    fn sem_balde_ou_sem_frame_nao_quebra() {
        let d = decoded(vec![0.5, 0.5], 1);
        assert!(d.peaks(0).is_empty());
        let empty = decoded(vec![], 2);
        assert_eq!(empty.peaks(3), vec![0.0; 3]);
    }
}
