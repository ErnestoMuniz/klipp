use std::collections::HashSet;

use super::pulse::run_pactl;

pub const CLIPS_SINK: &str = "klipp-clips";
pub const MIX_SINK: &str = "klipp-mix";
pub const VIRTUAL_MIC: &str = "soundboard-mic";

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum ModuleKind {
    ClipsSink,
    MixSink,
    Remap,
    ClipsToMix,
    MicPassthrough,
    HearClips,
}

/// Grafo PipeWire/Pulse: klipp-clips, klipp-mix, mic virtual e loopbacks.
/// Mesma estratégia do klipp-old via pactl.
pub struct AudioGraph {
    loaded: Vec<(ModuleKind, String)>,
    existed: HashSet<ModuleKind>,
    mic_loopback: Option<String>,
    hear_loopback: Option<String>,
    mic_source: String,
}

impl AudioGraph {
    pub fn new() -> Self {
        Self {
            loaded: vec![],
            existed: HashSet::new(),
            mic_loopback: None,
            hear_loopback: None,
            mic_source: String::new(),
        }
    }

    /// Fonte do microfone real resolvida no `init` (para exibir no drawer).
    pub fn mic_source(&self) -> &str {
        &self.mic_source
    }

    /// Cria o grafo. `preferred` é a fonte salva nas settings (pode ter
    /// desplugado — nesse caso cai no auto-detect). `mic_pass`/`hear`
    /// vêm dos toggles do drawer.
    pub fn init(
        &mut self,
        preferred: &str,
        mic_pass: bool,
        hear: bool,
    ) -> anyhow::Result<()> {
        self.init_inner(preferred)?;
        self.set_mic_passthrough(mic_pass)?;
        self.set_hear_clips(hear)?;
        Ok(())
    }

    /// Cria o grafo. Idempotente: primeiro remove loopbacks antigos com as
    /// nossas assinaturas (lixo de crashes ou execuções anteriores).
    fn init_inner(&mut self, preferred: &str) -> anyhow::Result<()> {
        self.remove_stale_loopbacks()?;

        let sinks = run_pactl(&["list", "short", "sinks"])?;
        let sources = run_pactl(&["list", "short", "sources"])?;

        let exists = |out: &str, name: &str| {
            out.lines().any(|line| line.split('\t').nth(1) == Some(name))
        };

        if !exists(&sinks, CLIPS_SINK) {
            let idx = run_pactl(&[
                "load-module",
                "module-null-sink",
                &format!("sink_name={CLIPS_SINK}"),
                "sink_properties=device.description=Klipp-Clips device.intended_roles=filter",
            ])?;
            self.loaded.push((ModuleKind::ClipsSink, idx.trim().into()));
        } else {
            self.existed.insert(ModuleKind::ClipsSink);
        }

        if !exists(&sinks, MIX_SINK) {
            let idx = run_pactl(&[
                "load-module",
                "module-null-sink",
                &format!("sink_name={MIX_SINK}"),
                "sink_properties=device.description=Klipp-Mix device.intended_roles=filter",
            ])?;
            self.loaded.push((ModuleKind::MixSink, idx.trim().into()));
        } else {
            self.existed.insert(ModuleKind::MixSink);
        }

        if !exists(&sources, VIRTUAL_MIC) {
            let idx = run_pactl(&[
                "load-module",
                "module-remap-source",
                &format!("source_name={VIRTUAL_MIC}"),
                &format!("master={MIX_SINK}.monitor"),
                "channels=2",
                "channel_map=front-left,front-right",
                "source_properties=device.description=Klipp-Mic",
            ])?;
            self.loaded.push((ModuleKind::Remap, idx.trim().into()));
        } else {
            self.existed.insert(ModuleKind::Remap);
        }

        // Clips -> mix (sempre): Discord ouve os clips.
        let idx = run_pactl(&[
            "load-module",
            "module-loopback",
            &format!("source={CLIPS_SINK}.monitor"),
            &format!("sink={MIX_SINK}"),
        ])?;
        self.loaded.push((ModuleKind::ClipsToMix, idx.trim().into()));

        // Passthrough do mic real para o mix.
        // Prefere a fonte salva nas settings; senão, o default source (se for
        // um mic de verdade — às vezes o sistema promove o próprio
        // soundboard-mic a default via stream-restore).
        let default_source = run_pactl(&["get-default-source"])?.trim().to_string();
        let auto = if default_source != VIRTUAL_MIC && !default_source.is_empty() {
            default_source
        } else {
            run_pactl(&["list", "short", "sources"])?
                .lines()
                .map(|line| line.split('\t').nth(1).unwrap_or("").to_string())
                .find(|name| {
                    name != VIRTUAL_MIC
                        && !name.ends_with(".monitor")
                        && !name.is_empty()
                })
                .unwrap_or_default()
        };
        self.mic_source = if !preferred.is_empty() {
            preferred.to_string()
        } else {
            auto
        };

        Ok(())
    }

    /// Troca a fonte do mic (seletor do drawer): recarrega o loopback se ligado.
    pub fn set_mic_source(&mut self, name: &str, pass_on: bool) -> anyhow::Result<()> {
        self.mic_source = name.to_string();
        self.set_mic_passthrough(pass_on)
    }

    /// Liga/desliga o loopback do mic real → mix (toggle do drawer).
    pub fn set_mic_passthrough(&mut self, on: bool) -> anyhow::Result<()> {
        if let Some(idx) = self.mic_loopback.take() {
            let _ = run_pactl(&["unload-module", &idx]);
            self.loaded
                .retain(|(k, i)| !(*k == ModuleKind::MicPassthrough && i == &idx));
        }
        if on && !self.mic_source.is_empty() {
            let mic = self.mic_source.clone();
            let idx = run_pactl(&[
                "load-module",
                "module-loopback",
                &format!("source={mic}"),
                &format!("sink={MIX_SINK}"),
            ])?;
            let idx = idx.trim().to_string();
            self.loaded.push((ModuleKind::MicPassthrough, idx.clone()));
            self.mic_loopback = Some(idx);
        }
        Ok(())
    }

    /// Liga/desliga o loopback clips → sink padrão (toggle do drawer).
    pub fn set_hear_clips(&mut self, on: bool) -> anyhow::Result<()> {
        if let Some(idx) = self.hear_loopback.take() {
            let _ = run_pactl(&["unload-module", &idx]);
            self.loaded
                .retain(|(k, i)| !(*k == ModuleKind::HearClips && i == &idx));
        }
        let default_sink = run_pactl(&["get-default-sink"])?.trim().to_string();
        if on && !default_sink.is_empty() && default_sink != CLIPS_SINK {
            let idx = run_pactl(&[
                "load-module",
                "module-loopback",
                &format!("source={CLIPS_SINK}.monitor"),
                &format!("sink={default_sink}"),
            ])?;
            let idx = idx.trim().to_string();
            self.loaded.push((ModuleKind::HearClips, idx.clone()));
            self.hear_loopback = Some(idx);
        }
        Ok(())
    }

    /// Descarrega loopbacks antigos cujos argumentos contenham os nossos nomes
    /// (klipp-clips/klipp-mix) — inclusive de processos mortos sem cleanup.
    fn remove_stale_loopbacks(&mut self) -> anyhow::Result<()> {
        let modules = run_pactl(&["list", "short", "modules"])?;
        for line in modules.lines() {
            let mut parts = line.split('\t');
            let (Some(idx), Some(name)) = (parts.next(), parts.next()) else {
                continue;
            };
            if name != "module-loopback" {
                continue;
            }
            if line.contains("klipp-clips") || line.contains("klipp-mix") {
                let _ = run_pactl(&["unload-module", idx]);
            }
        }
        Ok(())
    }

    /// Desfaz apenas os módulos que criamos (não os pré-existentes).
    pub fn cleanup(&mut self) {
        for (kind, idx) in self.loaded.drain(..).rev() {
            if self.existed.contains(&kind) {
                continue;
            }
            let _ = run_pactl(&["unload-module", &idx]);
        }
        self.existed.clear();
    }
}
