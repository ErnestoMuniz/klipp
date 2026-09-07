use std::sync::{Arc, Mutex, OnceLock};

use crate::core::i18n::t;
use crate::core::state::Shared;

/// Pedidos do tray icon para a UI thread (ver `main.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayEvent {
    /// Clique no ícone / "Show": mostra a janela (abre se fechada).
    Toggle,
    /// "Quit" no menu: encerra o app.
    Quit,
}

static QUEUE: OnceLock<Mutex<Vec<TrayEvent>>> = OnceLock::new();

fn queue() -> &'static Mutex<Vec<TrayEvent>> {
    QUEUE.get_or_init(|| Mutex::new(Vec::new()))
}

pub(crate) fn push_event(ev: TrayEvent) {
    if let Ok(mut q) = queue().lock() {
        q.push(ev);
    }
}

/// Drena os pedidos pendentes (chamado na UI thread, ~10Hz).
pub fn drain_events() -> Vec<TrayEvent> {
    queue()
        .lock()
        .map(|mut q| std::mem::take(&mut *q))
        .unwrap_or_default()
}

struct KlippTray {
    shared: Arc<Mutex<Shared>>,
}

impl ksni::Tray for KlippTray {
    fn id(&self) -> String {
        "io.github.ErnestoMuniz.Klipp".into()
    }

    fn title(&self) -> String {
        "Klipp".into()
    }

    /// Vazio de propósito: com `icon_name` resolvendo para a logo
    /// colorida do tema, os hosts a preferem ao pixmap. Sem nome, o
    /// host usa o pixmap monocromático embutido (o visual do tray).
    fn icon_name(&self) -> String {
        String::new()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        tray_icon_pixmap()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: "Klipp".into(),
            ..Default::default()
        }
    }

    /// Clique esquerdo: mostra a janela principal.
    fn activate(&mut self, _x: i32, _y: i32) {
        push_event(TrayEvent::Toggle);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;
        let lang = self
            .shared
            .lock()
            .map(|s| s.lang.clone())
            .unwrap_or_else(|_| "en".into());
        vec![
            ksni::MenuItem::Standard(StandardItem {
                label: t(&lang, "tray.show"),
                activate: Box::new(|_| push_event(TrayEvent::Toggle)),
                ..Default::default()
            }),
            ksni::MenuItem::Separator,
            ksni::MenuItem::Standard(StandardItem {
                label: t(&lang, "tray.quit"),
                activate: Box::new(|_| push_event(TrayEvent::Quit)),
                ..Default::default()
            }),
        ]
    }
}

/// Pixmaps monocromáticos herdados do `klipp-old` (branco + alpha),
/// um por tamanho — o host escolhe o mais próximo. `icon_name` continua
/// como fallback para hosts que preferem o tema.
fn tray_icon_pixmap() -> Vec<ksni::Icon> {
    static ICONS: OnceLock<Vec<ksni::Icon>> = OnceLock::new();
    ICONS
        .get_or_init(|| {
            [
                include_bytes!("../../assets/tray-16.png").as_slice(),
                include_bytes!("../../assets/tray-22.png").as_slice(),
                include_bytes!("../../assets/tray-32.png").as_slice(),
            ]
            .iter()
            .filter_map(|png| {
                let img = image::load_from_memory(png).ok()?.to_rgba8();
                let (width, height) = (img.width() as i32, img.height() as i32);
                let mut data = Vec::with_capacity((width * height * 4) as usize);
                for px in img.pixels() {
                    // RGBA → ARGB32 em ordem de rede (A, R, G, B).
                    data.extend_from_slice(&[px[3], px[0], px[1], px[2]]);
                }
                Some(ksni::Icon {
                    width,
                    height,
                    data,
                })
            })
            .collect()
        })
        .clone()
}

/// Sobe o StatusNotifierItem (D-Bus puro, sem deps de sistema).
/// Sem watcher (ex. GNOME sem extensão) só registra o aviso e segue:
/// o app funciona normalmente sem o ícone.
/// Roda no executor em background do GPUI (vive o processo inteiro).
pub async fn run(shared: Arc<Mutex<Shared>>) {
    use ksni::TrayMethods;
    // No sandbox o barramento filtrado impede registrar o nome
    // bem-conhecido no D-Bus — a spec prevê esse modo (ver ksni).
    let sandboxed = ashpd::is_sandboxed();
    let result = KlippTray { shared }
        .disable_dbus_name(sandboxed)
        .spawn()
        .await;
    match result {
        Ok(_) => log::info!("tray icon ativo (sandbox={sandboxed})"),
        Err(err) => log::warn!("tray icon indisponível: {err}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixmaps_monocromaticos_decodificam() {
        let icons = tray_icon_pixmap();
        assert_eq!(icons.len(), 3);
        let sizes: Vec<(i32, i32)> = icons.iter().map(|i| (i.width, i.height)).collect();
        assert!(sizes.contains(&(14, 16)));
        assert!(sizes.contains(&(20, 22)));
        assert!(sizes.contains(&(29, 32)));
        for icon in &icons {
            assert_eq!(icon.data.len(), (icon.width * icon.height * 4) as usize);
        }
    }

    #[test]
    fn fila_acumula_e_drena_em_ordem() {
        drain_events();
        push_event(TrayEvent::Toggle);
        push_event(TrayEvent::Quit);
        assert_eq!(
            drain_events(),
            vec![TrayEvent::Toggle, TrayEvent::Quit]
        );
        assert!(drain_events().is_empty());
    }
}
