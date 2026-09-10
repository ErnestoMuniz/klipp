use std::path::PathBuf;
use std::process::Command;

use crate::core::state::CursorExt;

/// Cursor exato no GNOME Wayland (mouse-coords 0.3.0): status + instalação
/// 1-clique da extensão companion.
///
/// Sem a extensão o pie abre no centro e se corrige no primeiro movimento
/// do mouse (âncora pendente) — usável, mas não exato. Com ela
/// (`org.mousecoords.Bridge` no ar), `get_position()` devolve o cursor
/// real e o pie abre em cima dele. A Shell só carrega extensão nova no
/// login: instalada-sem-login aparece como pendente, não como falha.
///
/// No KDE (KWin) e no Hyprland (socket IPC `cursorpos`) o mouse-coords
/// resolve sozinho, sem extensão: nada a instalar nem a mostrar aqui.
pub const EXT_UUID: &str = "mousecoords@mouse-coords.github.io";
/// Bridges aceitas (a nossa ou a do wdotool, GNOME 45–48).
const BRIDGES: &[&str] = &["org.mousecoords.Bridge", "org.wdotool.GnomeShellBridge"];

/// Sonda síncrona (poucas chamadas rápidas): só chamada de background
/// (largada, pós-install, re-probe periódico) ou de teste — nunca no frame.
pub fn status() -> CursorExt {
    if !crate::backend::gnome_shortcuts::is_desktop_gnome() {
        return CursorExt::Unknown;
    }
    if BRIDGES.iter().any(|b| has_owner(b)) {
        return CursorExt::Active;
    }
    if installed_dir().is_some() {
        // A Shell só lista extensões que viu num login: listada porém
        // sem dono = instalada e desativada; nem listada = falta logout.
        if shell_lists_uuid() {
            return CursorExt::Disabled;
        }
        return CursorExt::NeedsLogin;
    }
    CursorExt::Missing
}

/// Habilita a extensão instalada (`gnome-extensions enable`). Pode valer
/// na hora (a Shell às vezes carrega sem login); o garantido é no próximo
/// login. Devolve o status pós-enable.
pub fn enable() -> anyhow::Result<CursorExt> {
    let out = Command::new("gnome-extensions")
        .arg("enable")
        .arg(EXT_UUID)
        .output()
        .map_err(|e| anyhow::anyhow!("gnome-extensions indisponível: {e}"))?;
    if !out.status.success() {
        return Err(anyhow::anyhow!(
            "gnome-extensions enable: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    log::info!("extensão do cursor habilitada");
    Ok(status())
}

/// Instala a extensão embarcada e habilita (`gnome-extensions enable`).
/// Vale após logout/login. Devolve o status pós-install.
pub fn install() -> anyhow::Result<CursorExt> {
    let src = bundled_source().ok_or_else(|| anyhow::anyhow!("extensão não embarcada"))?;
    let dest = extensions_dir().join(EXT_UUID);
    copy_dir(&src, &dest)?;
    let out = Command::new("gnome-extensions")
        .arg("enable")
        .arg(EXT_UUID)
        .output()
        .map_err(|e| anyhow::anyhow!("gnome-extensions indisponível: {e}"))?;
    if !out.status.success() {
        return Err(anyhow::anyhow!(
            "gnome-extensions enable: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    log::info!("extensão do cursor instalada (vale após logout/login)");
    Ok(status())
}

/// Pasta da extensão no sistema, se copiada (com metadata).
fn installed_dir() -> Option<PathBuf> {
    let dir = extensions_dir().join(EXT_UUID);
    if dir.join("metadata.json").is_file() {
        Some(dir)
    } else {
        None
    }
}

fn extensions_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gnome-shell")
        .join("extensions")
}

/// A Shell lista a extensão? Só depois de vê-la num login — é assim que
/// o app distingue "falta sair/entrar" de "instalada porém desativada".
fn shell_lists_uuid() -> bool {
    Command::new("gnome-extensions")
        .arg("list")
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|l| l.trim() == EXT_UUID)
        })
        .unwrap_or(false)
}

/// Origem embarcada: AppImage (`../share/klipp/...` ao lado do binário)
/// ou árvore de dev (`CARGO_MANIFEST_DIR`, só existe no fonte).
fn bundled_source() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let cand = dir
                .join("..")
                .join("share")
                .join("klipp")
                .join("gnome-extension")
                .join(EXT_UUID);
            if cand.join("metadata.json").is_file() {
                return Some(cand);
            }
        }
    }
    let cand = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("packaging")
        .join("gnome-extension")
        .join(EXT_UUID);
    if cand.join("metadata.json").is_file() {
        Some(cand)
    } else {
        None
    }
}

fn has_owner(bus_name: &str) -> bool {
    Command::new("gdbus")
        .arg("call")
        .arg("--session")
        .arg("--dest")
        .arg("org.freedesktop.DBus")
        .arg("--object-path")
        .arg("/org/freedesktop/DBus")
        .arg("--method")
        .arg("org.freedesktop.DBus.NameHasOwner")
        .arg(bus_name)
        .output()
        .map(|o| o.status.success() && parse_has_owner(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or(false)
}

/// `"(true,)"` → true. Pura para ser testável.
fn parse_has_owner(out: &str) -> bool {
    out.trim() == "(true,)"
}

fn copy_dir(src: &PathBuf, dest: &PathBuf) -> anyhow::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let to = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CursorExt, EXT_UUID, parse_has_owner};

    #[test]
    fn dono_do_bridge() {
        assert!(parse_has_owner("(true,)\n"));
        assert!(!parse_has_owner("(false,)\n"));
        assert!(!parse_has_owner(""));
        assert!(!parse_has_owner("Error: xyz"));
    }

    #[test]
    fn status_fora_do_gnome_e_desconhecido() {
        // Fora de sessão GNOME o cursor usa outro backend (KWin/X11):
        // nada a instalar, nada a mostrar.
        for key in [
            "XDG_CURRENT_DESKTOP",
            "XDG_SESSION_DESKTOP",
            "DESKTOP_SESSION",
            "GDMSESSION",
        ] {
            unsafe { std::env::remove_var(key) };
        }
        unsafe { std::env::set_var("XDG_CURRENT_DESKTOP", "KDE") };
        assert_eq!(super::status(), CursorExt::Unknown);
        unsafe { std::env::remove_var("XDG_CURRENT_DESKTOP") };
    }

    #[test]
    fn copia_recursiva() {
        let base = std::env::temp_dir().join(format!("klipp-cur-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("a.txt"), "a").unwrap();
        std::fs::write(src.join("sub").join("b.txt"), "b").unwrap();
        let dest = base.join("dest");
        super::copy_dir(&src, &dest).unwrap();
        assert_eq!(std::fs::read_to_string(dest.join("a.txt")).unwrap(), "a");
        assert_eq!(
            std::fs::read_to_string(dest.join("sub").join("b.txt")).unwrap(),
            "b"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn uuid_bate_com_o_empacotado() {
        // O dir embarcado tem que existir com o mesmo uuid (dev).
        let cand = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("packaging")
            .join("gnome-extension")
            .join(EXT_UUID);
        assert!(
            cand.join("metadata.json").is_file(),
            "sem extensão embarcada"
        );
        assert!(cand.join("extension.js").is_file(), "sem extension.js");
    }

    /// Ao vivo: instala de verdade, confirma o bridge e remove tudo,
    /// restaurando o estado anterior. Sem re-login o status pode ser
    /// pendente (NeedsLogin) — mas no GNOME 50 o `enable` costuma carregar
    /// na hora (Active). Precisa de GNOME com gnome-extensions:
    /// `cargo test cursor_instala -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn cursor_instala_e_remove() {
        use std::process::Command;

        assert!(crate::backend::gnome_shortcuts::is_desktop_gnome());
        let dest = super::extensions_dir().join(EXT_UUID);
        let had_dir = dest.is_dir();
        let enabled_before = Command::new("gnome-extensions")
            .arg("list")
            .arg("--enabled")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        let was_enabled = enabled_before.lines().any(|l| l.trim() == EXT_UUID);
        let before = super::status();

        let status = super::install().expect("instalar extensão");
        assert!(dest.join("metadata.json").is_file(), "metadata copiada");
        assert!(
            matches!(status, CursorExt::Active | CursorExt::NeedsLogin),
            "instalada: ativa ou pendente de login, foi {status:?}"
        );
        // Bridge no ar: o cursor real resolve (prova fim-a-fim do 0.3.0).
        if status == CursorExt::Active {
            let pos = mouse_coords::get_position().expect("bridge responde");
            assert!(
                (0..10000).contains(&pos.x) && (0..10000).contains(&pos.y),
                "coordenada sã: {pos:?}"
            );
        }

        // Limpeza: volta ao estado anterior.
        if !was_enabled {
            let _ = Command::new("gnome-extensions")
                .arg("disable")
                .arg(EXT_UUID)
                .status();
        }
        if !had_dir {
            let _ = std::fs::remove_dir_all(&dest);
        }
        assert_eq!(super::status(), before, "limpeza restaura o estado");
    }
}
