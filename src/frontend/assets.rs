use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use open_gpui::{AssetSource, SharedString};

/// Fonte de assets do GPUI: serve os SVGs do pie gerados em memória.
pub struct AppAssets {
    pub svgs: Arc<Mutex<HashMap<String, String>>>,
}

impl AppAssets {
    pub fn new() -> Arc<Self> {
        let svgs = Arc::new(Mutex::new(HashMap::new()));
        let assets = Self { svgs };
        assets.register_defaults();
        Arc::new(assets)
    }

    /// Registra os SVGs estáticos (Lucide). O pie do overlay continua
    /// sendo gerado dinamicamente em `frontend::overlay`.
    /// A logo raster vai direto por `icons::logo()` (sem asset pipeline).
    fn register_defaults(&self) {
        let mut svgs = self.svgs.lock().unwrap();
        for (name, svg) in super::icons::all() {
            svgs.insert(name, svg);
        }
    }
}

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>, anyhow::Error> {
        Ok(self
            .svgs
            .lock()
            .unwrap()
            .get(path)
            .map(|svg| Cow::Owned(svg.clone().into_bytes())))
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>, anyhow::Error> {
        Ok(vec![])
    }
}
