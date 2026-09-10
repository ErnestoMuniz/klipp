//! Cliente nativo PulseAudio / PipeWire-Pulse via `libpulse-binding`.
//!
//! Substitui o antigo `pactl`: cada função abre uma conexão curta ao
//! servidor, executa uma operação e fecha. Sem binário externo no PATH.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::{Context, FlagSet, State as CtxState};
use libpulse_binding::mainloop::threaded::Mainloop;
use libpulse_binding::operation::{Operation, State as OpState};

use super::audio_graph::{CLIPS_SINK, MIX_SINK, VIRTUAL_MIC};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const OP_TIMEOUT: Duration = Duration::from_secs(5);

struct Connection {
    mainloop: Mainloop,
    context: Context,
}

impl Connection {
    fn connect() -> anyhow::Result<Self> {
        let mut mainloop =
            Mainloop::new().ok_or_else(|| anyhow::anyhow!("falha ao criar mainloop de áudio"))?;
        let mut context = Context::new(&mainloop, "klipp")
            .ok_or_else(|| anyhow::anyhow!("falha ao criar contexto de áudio"))?;
        context
            .connect(None, FlagSet::NOFLAGS, None)
            .map_err(|e| anyhow::anyhow!("falha ao conectar ao servidor de áudio: {e}"))?;
        mainloop
            .start()
            .map_err(|e| anyhow::anyhow!("falha ao iniciar áudio: {e}"))?;

        let deadline = Instant::now() + CONNECT_TIMEOUT;
        loop {
            match context.get_state() {
                CtxState::Ready => break,
                CtxState::Failed | CtxState::Terminated => {
                    mainloop.stop();
                    return Err(anyhow::anyhow!(
                        "servidor de áudio indisponível (PipeWire-Pulse/PulseAudio não está rodando?)"
                    ));
                }
                _ => {
                    if Instant::now() > deadline {
                        context.disconnect();
                        mainloop.stop();
                        return Err(anyhow::anyhow!(
                            "tempo esgotado ao conectar ao servidor de áudio"
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
        Ok(Self { mainloop, context })
    }

    fn wait_done<T: ?Sized>(op: &Operation<T>) -> anyhow::Result<()> {
        let deadline = Instant::now() + OP_TIMEOUT;
        loop {
            match op.get_state() {
                OpState::Done => return Ok(()),
                OpState::Cancelled => {
                    return Err(anyhow::anyhow!("operação de áudio cancelada"));
                }
                OpState::Running => {
                    if Instant::now() > deadline {
                        return Err(anyhow::anyhow!(
                            "tempo esgotado aguardando o servidor de áudio"
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.context.disconnect();
        self.mainloop.stop();
    }
}

/// `(nome, descrição)` de todas as sources (inclui monitores e mic virtual).
fn list_all_sources() -> anyhow::Result<Vec<(String, String)>> {
    let conn = Connection::connect()?;
    let out: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(vec![]));
    let op = {
        let intro = conn.context.introspect();
        let out = out.clone();
        intro.get_source_info_list(move |r| {
            if let ListResult::Item(info) = r {
                let name = info.name.as_ref().map(|s| s.to_string()).unwrap_or_default();
                if name.is_empty() {
                    return;
                }
                let desc = info
                    .description
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| name.clone());
                out.lock().unwrap().push((name, desc));
            }
        })
    };
    Connection::wait_done(&op)?;
    Ok(out.lock().unwrap().clone())
}

/// Nomes curtos de todas as sources.
pub fn list_source_names() -> anyhow::Result<Vec<String>> {
    Ok(list_all_sources()?
        .into_iter()
        .map(|(n, _)| n)
        .collect())
}

/// Microfones reais: `(nome, descrição)`, sem monitores nem mic virtual.
/// Infallível (retorna `[]` sem servidor), como antes.
pub fn list_sources() -> Vec<(String, String)> {
    let Ok(all) = list_all_sources() else {
        return vec![];
    };
    all.into_iter()
        .filter(|(n, _)| !n.ends_with(".monitor") && n != VIRTUAL_MIC)
        .collect()
}

/// Source padrão do servidor.
pub fn default_source() -> anyhow::Result<String> {
    let conn = Connection::connect()?;
    let out: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let op = {
        let intro = conn.context.introspect();
        let out = out.clone();
        intro.get_server_info(move |info| {
            *out.lock().unwrap() = info
                .default_source_name
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_default();
        })
    };
    Connection::wait_done(&op)?;
    Ok(out.lock().unwrap().clone())
}

/// Sink padrão do servidor.
pub fn default_sink() -> anyhow::Result<String> {
    let conn = Connection::connect()?;
    let out: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let op = {
        let intro = conn.context.introspect();
        let out = out.clone();
        intro.get_server_info(move |info| {
            *out.lock().unwrap() = info
                .default_sink_name
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_default();
        })
    };
    Connection::wait_done(&op)?;
    Ok(out.lock().unwrap().clone())
}

/// Carrega um módulo e retorna seu índice.
pub fn load_module(module: &str, args: &str) -> anyhow::Result<u32> {
    let conn = Connection::connect()?;
    let out: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
    let op = {
        let mut intro = conn.context.introspect();
        let out = out.clone();
        intro.load_module(module, args, move |idx| {
            *out.lock().unwrap() = Some(idx);
        })
    };
    Connection::wait_done(&op)?;
    out.lock()
        .unwrap()
        .ok_or_else(|| anyhow::anyhow!("servidor não retornou o índice do módulo {module}"))
}

/// Descarrega um módulo pelo índice. Melhor esforço (como antes).
pub fn unload_module(idx: u32) {
    let Ok(conn) = Connection::connect() else {
        return;
    };
    let ok: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
    let op = {
        let mut intro = conn.context.introspect();
        let ok = ok.clone();
        intro.unload_module(idx, move |success| {
            *ok.lock().unwrap() = Some(success);
        })
    };
    let _ = Connection::wait_done(&op);
}

/// Índices dos módulos por trás dos NOSSOS nomes reservados
/// (`klipp-clips`, `klipp-mix`, `soundboard-mic`) mais loopbacks antigos com
/// nossas assinaturas.
///
/// Só o klipp cria esses nomes (instância única via IPC), então tudo aqui
/// é ou lixo de processo morto ou de um `init` anterior: o dono atual pode
/// derrubar e recriar. É o que impede "dispositivo fantasma" após crash —
/// adotar-e-preservar tornaria o lixo imortal.
pub fn owned_module_indices() -> anyhow::Result<Vec<u32>> {
    let conn = Connection::connect()?;
    let mut out: Vec<u32> = vec![];

    // Sinks nossos -> módulo dono (module-null-sink).
    {
        let found: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(vec![]));
        let op = {
            let intro = conn.context.introspect();
            let found = found.clone();
            intro.get_sink_info_list(move |r| {
                if let ListResult::Item(info) = r
                    && let Some(name) = info.name.as_ref()
                    && (name == CLIPS_SINK || name == MIX_SINK)
                    && let Some(owner) = info.owner_module
                {
                    found.lock().unwrap().push(owner);
                }
            })
        };
        Connection::wait_done(&op)?;
        out.extend(found.lock().unwrap().iter());
    }

    // Mic virtual -> módulo dono (module-remap-source).
    {
        let found: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(vec![]));
        let op = {
            let intro = conn.context.introspect();
            let found = found.clone();
            intro.get_source_info_list(move |r| {
                if let ListResult::Item(info) = r
                    && let Some(name) = info.name.as_ref()
                    && name == VIRTUAL_MIC
                    && let Some(owner) = info.owner_module
                {
                    found.lock().unwrap().push(owner);
                }
            })
        };
        Connection::wait_done(&op)?;
        out.extend(found.lock().unwrap().iter());
    }

    out.extend(stale_loopbacks()?);

    out.sort_unstable();
    out.dedup();
    Ok(out)
}

/// Índices de `module-loopback` antigos com nossos nomes nos argumentos
/// (lixo de crashes ou execuções anteriores).
fn stale_loopbacks() -> anyhow::Result<Vec<u32>> {
    let conn = Connection::connect()?;
    let out: Arc<Mutex<Vec<(u32, String, String)>>> = Arc::new(Mutex::new(vec![]));
    let op = {
        let intro = conn.context.introspect();
        let out = out.clone();
        intro.get_module_info_list(move |r| {
            if let ListResult::Item(info) = r {
                let name = info.name.as_ref().map(|s| s.to_string()).unwrap_or_default();
                let args = info
                    .argument
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                out.lock().unwrap().push((info.index, name, args));
            }
        })
    };
    Connection::wait_done(&op)?;
    Ok(out
        .lock()
        .unwrap()
        .iter()
        .filter(|(_, name, args)| {
            name == "module-loopback"
                && (args.contains("klipp-clips") || args.contains("klipp-mix"))
        })
        .map(|(idx, _, _)| *idx)
        .collect())
}

#[cfg(test)]
mod live_tests {
    //! Exigem servidor Pulse/PipeWire vivo. Ignorados no CI (`cargo test`
    //! pula); rode manual em série com
    //! `cargo test -p klipp pulse_live -- --ignored --nocapture --test-threads=1`
    //! (em paralelo o load/unload de um teste altera a contagem do outro).

    use super::{Arc, Connection, ListResult, Mutex};

    /// Nomes de todos os sinks (só para asserções dos testes live).
    fn list_sinks() -> anyhow::Result<Vec<String>> {
        let conn = Connection::connect()?;
        let out: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let op = {
            let intro = conn.context.introspect();
            let out = out.clone();
            intro.get_sink_info_list(move |r| {
                if let ListResult::Item(info) = r
                    && let Some(name) = info.name.as_ref()
                {
                    out.lock().unwrap().push(name.to_string());
                }
            })
        };
        Connection::wait_done(&op)?;
        Ok(out.lock().unwrap().clone())
    }

    #[test]
    #[ignore]
    fn pulse_live_paridade_com_pactl() {
        let pactl = |args: &[&str]| {
            let out = std::process::Command::new("pactl")
                .args(args)
                .output()
                .expect("pactl precisa estar instalado para o teste de paridade");
            assert!(out.status.success());
            String::from_utf8_lossy(&out.stdout).to_string()
        };

        let sinks = list_sinks().expect("list_sinks");
        let pactl_names: Vec<String> = pactl(&["list", "short", "sinks"])
            .lines()
            .map(|l| l.split('\t').nth(1).unwrap_or("").to_string())
            .collect();
        assert_eq!(sinks.len(), pactl_names.len(), "nº de sinks diverge");
        for s in &pactl_names {
            assert!(sinks.contains(s), "sink {s} faltando no nativo");
        }

        let ds = super::default_source().expect("default_source");
        assert_eq!(ds, pactl(&["get-default-source"]).trim());
        let dk = super::default_sink().expect("default_sink");
        assert_eq!(dk, pactl(&["get-default-sink"]).trim());

        let mics = super::list_sources();
        assert!(
            mics.iter().all(|(n, _)| !n.ends_with(".monitor")
                && n != super::super::audio_graph::VIRTUAL_MIC),
            "filtro de monitores/mic virtual quebrou: {mics:?}"
        );
    }

    #[test]
    #[ignore]
    fn pulse_live_ciclo_load_unload() {
        let idx = super::load_module("module-null-sink", "sink_name=klipp-test-tmp")
            .expect("load_module");
        assert!(
            list_sinks().expect("list").contains(&"klipp-test-tmp".to_string()),
            "sink de teste não apareceu"
        );
        super::unload_module(idx);
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(
            !list_sinks().expect("list").contains(&"klipp-test-tmp".to_string()),
            "sink de teste não sumiu após unload"
        );
    }

    #[test]
    #[ignore]
    fn pulse_live_grafo_init_cleanup() {
        use super::super::audio_graph::{AudioGraph, CLIPS_SINK, MIX_SINK, VIRTUAL_MIC};
        let mut g = AudioGraph::new();
        g.init("", true, false).expect("init");
        let sinks = list_sinks().expect("list");
        assert!(sinks.contains(&CLIPS_SINK.to_string()));
        assert!(sinks.contains(&MIX_SINK.to_string()));
        assert!(
            super::list_source_names()
                .expect("sources")
                .contains(&VIRTUAL_MIC.to_string())
        );
        g.cleanup();
        // Quit de verdade não deixa fantasma: nomes somem do servidor.
        std::thread::sleep(std::time::Duration::from_millis(300));
        let sinks = list_sinks().expect("list");
        assert!(!sinks.contains(&CLIPS_SINK.to_string()), "klipp-clips vazou");
        assert!(!sinks.contains(&MIX_SINK.to_string()), "klipp-mix vazou");
        assert!(
            !super::list_source_names()
                .expect("sources")
                .contains(&VIRTUAL_MIC.to_string()),
            "soundboard-mic vazou"
        );
    }

    #[test]
    #[ignore]
    fn pulse_live_reinit_nao_vaza() {
        // Regressão: reabrir a janela via tray rodava `init` de novo no mesmo
        // grafo; a geração anterior era marcada "pré-existente" e o cleanup
        // pulava — klipp-clips/mix/mic ficavam imortais.
        use super::super::audio_graph::AudioGraph;
        let mut g = AudioGraph::new();
        g.init("", true, false).expect("init 1");
        g.init("", true, false).expect("init 2");
        g.cleanup();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let sinks = list_sinks().expect("list");
        assert!(!sinks.iter().any(|s| s.starts_with("klipp-")), "vazou: {sinks:?}");
        assert!(
            !super::list_source_names()
                .expect("sources")
                .iter()
                .any(|s| s == super::super::audio_graph::VIRTUAL_MIC),
            "mic vazou"
        );
    }
}
