use super::pulse;

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
/// Gerenciado via API nativa (`libpulse-binding`), sem `pactl`.
///
/// Posse total dos nossos nomes reservados: `init` derruba o que rastreia
/// e retoma módulos por trás de `klipp-clips`/`klipp-mix`/`soundboard-mic`
/// (lixo de processo morto — instância única garante que não há dono vivo),
/// recria tudo do zero e `cleanup` descarrega tudo que criou. Sem isso o
/// lixo virava "pré-existente", era adotado-e-preservado e ficava imortal.
pub struct AudioGraph {
    loaded: Vec<(ModuleKind, u32)>,
    mic_loopback: Option<u32>,
    hear_loopback: Option<u32>,
    mic_source: String,
}

impl AudioGraph {
    pub fn new() -> Self {
        Self {
            loaded: vec![],
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
    ///
    /// Reentrante: derruba a geração anterior (caso tray reabra a janela)
    /// e retoma nossos nomes antes de recriar.
    pub fn init(
        &mut self,
        preferred: &str,
        mic_pass: bool,
        hear: bool,
    ) -> anyhow::Result<()> {
        self.shutdown_owned();
        for idx in pulse::owned_module_indices()? {
            pulse::unload_module(idx);
        }
        self.init_inner(preferred)?;
        self.set_mic_passthrough(mic_pass)?;
        self.set_hear_clips(hear)?;
        Ok(())
    }

    /// Cria os sinks, o mic virtual e o loopback clips→mix do zero.
    fn init_inner(&mut self, preferred: &str) -> anyhow::Result<()> {
        let idx = pulse::load_module(
            "module-null-sink",
            &format!(
                "sink_name={CLIPS_SINK} sink_properties=device.description=Klipp-Clips device.intended_roles=filter"
            ),
        )?;
        self.loaded.push((ModuleKind::ClipsSink, idx));

        let idx = pulse::load_module(
            "module-null-sink",
            &format!(
                "sink_name={MIX_SINK} sink_properties=device.description=Klipp-Mix device.intended_roles=filter"
            ),
        )?;
        self.loaded.push((ModuleKind::MixSink, idx));

        let idx = pulse::load_module(
            "module-remap-source",
            &format!(
                "source_name={VIRTUAL_MIC} master={MIX_SINK}.monitor channels=2 channel_map=front-left,front-right source_properties=device.description=Klipp-Mic"
            ),
        )?;
        self.loaded.push((ModuleKind::Remap, idx));

        // Clips -> mix (sempre): Discord ouve os clips.
        let idx = pulse::load_module(
            "module-loopback",
            &format!("source={CLIPS_SINK}.monitor sink={MIX_SINK}"),
        )?;
        self.loaded.push((ModuleKind::ClipsToMix, idx));

        // Passthrough do mic real para o mix.
        // Prefere a fonte salva nas settings; senão, o default source (se for
        // um mic de verdade — às vezes o sistema promove o próprio
        // soundboard-mic a default via stream-restore).
        let default_source = pulse::default_source()?.trim().to_string();
        let auto = if default_source != VIRTUAL_MIC && !default_source.is_empty() {
            default_source
        } else {
            pulse::list_source_names()?
                .into_iter()
                .find(|name| {
                    name != VIRTUAL_MIC && !name.ends_with(".monitor") && !name.is_empty()
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
            pulse::unload_module(idx);
            self.loaded
                .retain(|(k, i)| !(*k == ModuleKind::MicPassthrough && *i == idx));
        }
        if on && !self.mic_source.is_empty() {
            let mic = self.mic_source.clone();
            let idx =
                pulse::load_module("module-loopback", &format!("source={mic} sink={MIX_SINK}"))?;
            self.loaded.push((ModuleKind::MicPassthrough, idx));
            self.mic_loopback = Some(idx);
        }
        Ok(())
    }

    /// Liga/desliga o loopback clips → sink padrão (toggle do drawer).
    pub fn set_hear_clips(&mut self, on: bool) -> anyhow::Result<()> {
        if let Some(idx) = self.hear_loopback.take() {
            pulse::unload_module(idx);
            self.loaded
                .retain(|(k, i)| !(*k == ModuleKind::HearClips && *i == idx));
        }
        let default_sink = pulse::default_sink()?.trim().to_string();
        if on && !default_sink.is_empty() && default_sink != CLIPS_SINK {
            let idx = pulse::load_module(
                "module-loopback",
                &format!("source={CLIPS_SINK}.monitor sink={default_sink}"),
            )?;
            self.loaded.push((ModuleKind::HearClips, idx));
            self.hear_loopback = Some(idx);
        }
        Ok(())
    }

    /// Descarrega tudo que rastreamos e limpa o estado (melhor esforço).
    fn shutdown_owned(&mut self) {
        for (_, idx) in self.loaded.drain(..).rev() {
            pulse::unload_module(idx);
        }
        self.mic_loopback = None;
        self.hear_loopback = None;
    }

    /// Desfaz o grafo na saída do app: descarrega tudo que criamos.
    pub fn cleanup(&mut self) {
        self.shutdown_owned();
    }
}
