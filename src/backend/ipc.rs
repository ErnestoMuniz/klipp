use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::core::state::Shared;

/// Comando de uma segunda invocação para a instância em execução.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Command {
    /// Abre o overlay (ou confirma a seleção, se já aberto).
    ToggleOverlay,
    /// Traz a janela principal ao frente.
    Show,
}

pub struct Cli {
    pub command: Option<Command>,
    pub help: bool,
    pub version: bool,
    pub invalid: Option<String>,
}

pub const HELP: &str = "Klipp — desktop soundboard\n\
    \n\
    Uso:\n  \
      klipp [OPÇÃO]\n\
    \n\
    Opções:\n  \
      --toggle-overlay   Abre o overlay no cursor (ou confirma, se aberto).\n                         \
    Para atalho global, amarre esta flag num atalho custom do DE\n                         \
    (KDE: Settings → Shortcuts → Add New → Command/URL).\n  \
      --show               Traz a janela principal ao frente.\n  \
      -h, --help           Mostra esta ajuda.\n  \
      -V, --version        Mostra a versão.\n\
    \n\
    Sem flag, uma segunda invocação só foca a janela existente\n\
    (instância única via socket em $XDG_RUNTIME_DIR/klipp.sock).\n";

/// Parse puro de argv (sem o nome do programa): trivialmente testável.
pub fn parse_cli(args: impl IntoIterator<Item = String>) -> Cli {
    let mut cli = Cli {
        command: None,
        help: false,
        version: false,
        invalid: None,
    };
    for arg in args {
        match arg.as_str() {
            "--toggle-overlay" => cli.command = Some(Command::ToggleOverlay),
            "--show" => cli.command = Some(Command::Show),
            "-h" | "--help" => cli.help = true,
            "-V" | "--version" => cli.version = true,
            _ if arg.starts_with('-') => {
                cli.invalid = Some(arg);
                break;
            }
            _ => {
                cli.invalid = Some(arg);
                break;
            }
        }
    }
    cli
}

/// Papel deste processo na instância única.
pub enum Instance {
    /// Primeiro processo: inicia o app (listener de IPC já rodando).
    Primary,
    /// Já havia outro: o comando foi entregue (ou era só focar).
    Secondary,
}

/// Garante instância única via socket Unix. Se já houver um Klipp rodando,
/// entrega `cmd` (padrão: focar a janela) e o chamador deve encerrar.
/// Senão, sobe o listener em thread e o chamador segue como primário.
pub fn ensure_single_instance(shared: &Arc<Mutex<Shared>>, cmd: Option<Command>) -> Instance {
    let path = socket_path();
    match UnixListener::bind(&path) {
        Ok(listener) => {
            spawn_listener(listener, shared.clone());
            Instance::Primary
        }
        Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => {
            let cmd = cmd.unwrap_or(Command::Show);
            match send_command(&path, cmd) {
                Ok(()) => Instance::Secondary,
                Err(_) => {
                    // Socket obsoleto (processo morto sem limpar): assume.
                    let _ = std::fs::remove_file(&path);
                    match UnixListener::bind(&path) {
                        Ok(listener) => {
                            spawn_listener(listener, shared.clone());
                            Instance::Primary
                        }
                        Err(err) => {
                            log::warn!("ipc indisponível ({err}); seguindo sem instância única");
                            Instance::Primary
                        }
                    }
                }
            }
        }
        Err(err) => {
            log::warn!("ipc indisponível ({err}); seguindo sem instância única");
            Instance::Primary
        }
    }
}

pub fn socket_path() -> PathBuf {
    if let Ok(sock) = std::env::var("KLIPP_IPC_SOCK") {
        if !sock.is_empty() {
            return PathBuf::from(sock);
        }
    }
    if let Ok(rt) = std::env::var("XDG_RUNTIME_DIR") {
        if !rt.is_empty() {
            return PathBuf::from(rt).join("klipp.sock");
        }
    }
    std::env::temp_dir().join("klipp.sock")
}

fn send_command(path: &std::path::Path, cmd: Command) -> std::io::Result<()> {
    let mut stream = UnixStream::connect(path)?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let line = match cmd {
        Command::ToggleOverlay => "toggle-overlay\n",
        Command::Show => "show\n",
    };
    stream.write_all(line.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn spawn_listener(listener: UnixListener, shared: Arc<Mutex<Shared>>) {
    std::thread::Builder::new()
        .name("klipp-ipc".into())
        .spawn(move || {
            for conn in listener.incoming() {
                let Ok(stream) = conn else { continue };
                if stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .is_err()
                {
                    continue;
                }
                let mut reader = BufReader::new(stream);
                let mut line = String::new();
                let cmd = match reader.read_line(&mut line) {
                    Ok(_) => line.trim().to_string(),
                    Err(_) => continue,
                };
                match cmd.as_str() {
                    "toggle-overlay" => {
                        log::info!("ipc: toggle-overlay");
                        crate::backend::overlay::toggle(&shared);
                    }
                    "show" => {
                        log::info!("ipc: show");
                        crate::backend::tray::push_event(
                            crate::backend::tray::TrayEvent::Toggle,
                        );
                    }
                    other => {
                        log::info!("ipc: comando desconhecido '{other}'");
                    }
                }
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::settings::Settings;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SOCK_SEQ: AtomicU64 = AtomicU64::new(0);

    fn test_sock() -> PathBuf {
        let id = SOCK_SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("klipp-test-{}-{id}.sock", std::process::id()))
    }

    #[test]
    fn parse_flags() {
        let cli = parse_cli(["--toggle-overlay".to_string()]);
        assert_eq!(cli.command, Some(Command::ToggleOverlay));
        assert!(!cli.help && !cli.version && cli.invalid.is_none());

        let cli = parse_cli(["--show".to_string()]);
        assert_eq!(cli.command, Some(Command::Show));

        let cli = parse_cli(["-h".to_string()]);
        assert!(cli.help);

        let cli = parse_cli(Vec::<String>::new());
        assert!(cli.command.is_none() && !cli.help && !cli.version);

        let cli = parse_cli(["--bogus".to_string()]);
        assert_eq!(cli.invalid.as_deref(), Some("--bogus"));
    }

    #[test]
    fn segunda_instancia_entrega_toggle() {
        // Via "toggle-overlay" sobre overlay ativo: o listener escreve
        // direto no Shared (sem a fila global do tray — sem corrida com
        // os testes do tray, que rodam em paralelo no mesmo processo).
        let sock = test_sock();
        unsafe { std::env::set_var("KLIPP_IPC_SOCK", &sock) };
        let _ = std::fs::remove_file(&sock);

        let shared = Arc::new(Mutex::new(Shared::new(&Settings::default())));
        {
            let mut g = shared.lock().unwrap();
            g.overlay_active = true;
        }
        assert!(matches!(
            ensure_single_instance(&shared, None),
            Instance::Primary
        ));
        assert!(matches!(
            ensure_single_instance(&shared, Some(Command::ToggleOverlay)),
            Instance::Secondary
        ));

        let mut confirmed = false;
        for _ in 0..100 {
            if shared.lock().unwrap().confirm_request {
                confirmed = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(confirmed, "listener devia confirmar a seleção");
        let _ = std::fs::remove_file(&sock);
        unsafe { std::env::remove_var("KLIPP_IPC_SOCK") };
    }
}
