//! Orquestra o atualizador interno sobre o `Shared`.
//!
//! Rede e disco rodam em threads (a UI repinta via `bump`, como o resto
//! do app). O restart é executado na UI thread: o download agenda o
//! caminho em `update_restart` e o tick consome (ver `MainWindow::on_tick`)
//! — sair no meio da thread worker com locks na mão seria pedir deadlock.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use crate::backend;
use crate::core::state::{Shared, UpdateStatus};

/// Checagem manual (botão no sobre): sempre consulta a API.
pub fn check_updates(shared: &Arc<Mutex<Shared>>) {
    {
        let mut s = shared.lock().unwrap();
        match s.update_status {
            UpdateStatus::Checking | UpdateStatus::Downloading | UpdateStatus::Applying => {
                return;
            }
            _ => {}
        }
        s.update_status = UpdateStatus::Checking;
        s.update_error = None;
        s.bump();
    }
    let shared = shared.clone();
    std::thread::Builder::new()
        .name("klipp-update-check".into())
        .spawn(move || {
            match backend::updater::check_for_update() {
                Ok(None) => {
                    let mut s = shared.lock().unwrap();
                    s.update_status = UpdateStatus::UpToDate;
                    s.update_version = None;
                    s.bump();
                }
                Ok(Some(rel)) => {
                    log::info!("update: {} disponível ({})", rel.version, rel.tag);
                    let mut s = shared.lock().unwrap();
                    s.update_status = UpdateStatus::Available;
                    s.update_version = Some(rel.version);
                    s.update_asset_url = Some(rel.asset_url);
                    s.update_asset_name = Some(rel.asset_name);
                    s.update_page_url = Some(rel.page_url);
                    s.update_error = None;
                    s.bump();
                }
                Err(err) => {
                    log::warn!("update: checagem falhou: {err:#}");
                    let mut s = shared.lock().unwrap();
                    // Auto-check silencioso: sem banner, sem drama — o
                    // usuário pode tentar à mão no sobre.
                    s.update_status = UpdateStatus::Error;
                    s.update_error = Some(format!("{err:#}"));
                    s.bump();
                }
            }
        })
        .ok();
}

/// Auto-check da largada: uma vez por processo, com atraso para não
/// disputar rede/CPU com a inicialização da janela e do áudio.
pub fn auto_check_updates(shared: &Arc<Mutex<Shared>>) {
    static ONCE: OnceLock<()> = OnceLock::new();
    let shared = shared.clone();
    ONCE.get_or_init(|| {
        std::thread::Builder::new()
            .name("klipp-update-auto".into())
            .spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(4));
                check_updates(&shared);
            })
            .ok();
    });
}

/// Botão "atualizar": no AppImage baixa, troca o arquivo e agenda o
/// restart; fora dele abre a página da release no navegador.
pub fn apply_update(shared: &Arc<Mutex<Shared>>) {
    let release = {
        let mut s = shared.lock().unwrap();
        match s.update_status {
            UpdateStatus::Downloading | UpdateStatus::Applying => return,
            _ => {}
        }
        let release = match (
            s.update_version.clone(),
            s.update_asset_url.clone(),
            s.update_asset_name.clone(),
        ) {
            (Some(version), Some(asset_url), Some(asset_name)) => {
                backend::updater::UpdateRelease {
                    version,
                    tag: String::new(),
                    asset_url,
                    asset_name,
                    page_url: s.update_page_url.clone().unwrap_or_default(),
                }
            }
            _ => {
                // Sem release detectada (ex. fora do AppImage): abre a
                // página de releases em vez de trocar binário.
                let page = s
                    .update_page_url
                    .clone()
                    .unwrap_or_else(|| "https://github.com/ErnestoMuniz/klipp/releases/latest".to_string());
                drop(s);
                backend::updater::open_in_browser(&page);
                return;
            }
        };
        if backend::updater::appimage_path().is_none() {
            let page = if release.page_url.is_empty() {
                "https://github.com/ErnestoMuniz/klipp/releases/latest".to_string()
            } else {
                release.page_url.clone()
            };
            drop(s);
            backend::updater::open_in_browser(&page);
            return;
        }
        s.update_status = UpdateStatus::Downloading;
        s.update_progress = None;
        s.update_error = None;
        s.bump();
        release
    };
    let shared = shared.clone();
    std::thread::Builder::new()
        .name("klipp-update-apply".into())
        .spawn(move || {
            let progress = {
                let shared = shared.clone();
                move |done: u64, total: Option<u64>| {
                    let mut s = shared.lock().unwrap();
                    s.update_progress = Some((done, total));
                    s.bump();
                }
            };
            {
                let mut s = shared.lock().unwrap();
                s.update_status = UpdateStatus::Applying;
                s.bump();
            }
            match backend::updater::apply_over_appimage(&release, &progress) {
                Ok(path) => {
                    log::info!("update: aplicado, reiniciando em {}", path.display());
                    let mut s = shared.lock().unwrap();
                    s.update_restart = Some(path);
                    s.bump();
                }
                Err(err) => {
                    log::warn!("update: aplicação falhou: {err:#}");
                    let mut s = shared.lock().unwrap();
                    s.update_status = UpdateStatus::Error;
                    s.update_error = Some(format!("{err:#}"));
                    s.bump();
                }
            }
        })
        .ok();
}

/// Lança o processo novo e encerra este (chamado na UI thread).
/// `KLIPP_UPDATED=1` faz o filho remover o socket IPC obsoleto antes do
/// bind — senão ele se enxergaria como "segunda instância" e sairia.
pub fn restart_into(path: PathBuf) -> anyhow::Result<()> {
    std::process::Command::new(&path)
        .env("KLIPP_UPDATED", "1")
        .spawn()
        .map_err(|err| anyhow::anyhow!("não consegui relançar: {err}"))?;
    // Respiro para o filho dar bind no socket antes deste soltar tudo
    // (tray some e volta em seguida).
    std::thread::sleep(std::time::Duration::from_millis(500));
    std::process::exit(0);
}
