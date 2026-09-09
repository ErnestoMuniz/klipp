//! Atualizador interno do AppImage.
//!
//! Fluxo: consulta `releases/latest` no GitHub, compara a versão com a
//! embutida (`CARGO_PKG_VERSION`) e, se houver release nova com asset
//! `.AppImage`, baixa e troca o arquivo em execução (`$APPIMAGE`), com
//! restart em seguida (ver `frontend::updater`).
//!
//! Fora do AppImage (dev/`cargo run`): só detecta — o "atualizar" abre a
//! página da release no navegador em vez de trocar binário.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::time::Duration;

const API_LATEST: &str = "https://api.github.com/repos/ErnestoMuniz/klipp/releases/latest";
const UA: &str = concat!("klipp/", env!("CARGO_PKG_VERSION"), " (appimage-updater)");

/// Release remota com asset `.AppImage` usável.
#[derive(Clone, Debug)]
pub struct UpdateRelease {
    /// Versão sem o `v` ("0.4.0").
    pub version: String,
    /// Tag original ("v0.4.0").
    pub tag: String,
    /// URL direta do `.AppImage` (`browser_download_url`).
    pub asset_url: String,
    /// Nome do arquivo do asset.
    pub asset_name: String,
    /// Página da release (fallback / "ver novidades").
    pub page_url: String,
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Caminho do AppImage em execução (`$APPIMAGE`, definido pelo runtime).
/// `None` fora do AppImage (dev, tarball, Flatpak...).
pub fn appimage_path() -> Option<PathBuf> {
    let p = std::env::var_os("APPIMAGE")?;
    if p.is_empty() {
        return None;
    }
    let path = PathBuf::from(p);
    if path.as_os_str().is_empty() {
        return None;
    }
    Some(path)
}

/// Compara versões `major.minor.patch` (componentes ausentes valem 0;
/// sufixos como `-rc1` são ignorados para a ordem numérica).
pub fn compare_versions(current: &str, latest: &str) -> Ordering {
    let mut a = numeric_parts(current);
    let mut b = numeric_parts(latest);
    let len = a.len().max(b.len());
    a.resize(len, 0);
    b.resize(len, 0);
    a.cmp(&b)
}

fn numeric_parts(v: &str) -> Vec<u64> {
    let core = v
        .trim()
        .trim_start_matches(['v', 'V'])
        .split(['-', '+'])
        .next()
        .unwrap_or("");
    core.split('.')
        .map(|p| {
            p.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<u64>()
                .unwrap_or(0)
        })
        .collect()
}

/// Escolhe o asset `.AppImage` da release: prefere o nome exato da
/// convenção (`Klipp-<ver>-<arch>.AppImage`), senão qualquer `Klipp-*`
/// terminado em `.AppImage` (nunca o `.zsync`).
fn pick_asset<'a>(version: &str, assets: &'a [(String, String)]) -> Option<&'a (String, String)> {
    let arch = std::env::consts::ARCH;
    let exact = format!("Klipp-{version}-{arch}.AppImage");
    if let Some(a) = assets.iter().find(|(name, _)| name == &exact) {
        return Some(a);
    }
    assets.iter().find(|(name, _)| {
        name.starts_with("Klipp-") && name.ends_with(".AppImage") && !name.ends_with(".zsync")
    })
}

/// Consulta a latest release. `Ok(None)` = já está na última versão.
pub fn check_for_update() -> anyhow::Result<Option<UpdateRelease>> {
    let resp = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .build()
        .get(API_LATEST)
        .set("User-Agent", UA)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|err| anyhow::anyhow!("GitHub: {err}"))?;
    let json: serde_json::Value = resp
        .into_json()
        .map_err(|err| anyhow::anyhow!("resposta inválida: {err}"))?;
    parse_latest(&json)
}

fn parse_latest(json: &serde_json::Value) -> anyhow::Result<Option<UpdateRelease>> {
    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("release sem tag_name"))?;
    let version = tag.trim_start_matches(['v', 'V']).to_string();
    if compare_versions(current_version(), &version) != Ordering::Less {
        return Ok(None);
    }
    let assets: Vec<(String, String)> = json
        .get("assets")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|a| {
            Some((
                a.get("name")?.as_str()?.to_string(),
                a.get("browser_download_url")?.as_str()?.to_string(),
            ))
        })
        .collect();
    let Some((name, url)) = pick_asset(&version, &assets) else {
        anyhow::bail!("release {tag} sem asset .AppImage para esta arquitetura");
    };
    Ok(Some(UpdateRelease {
        version,
        tag: tag.to_string(),
        asset_url: url.clone(),
        asset_name: name.clone(),
        page_url: json
            .get("html_url")
            .and_then(|v| v.as_str())
            .unwrap_or("https://github.com/ErnestoMuniz/klipp/releases/latest")
            .to_string(),
    }))
}

/// Baixa o asset para `dest` (com progresso em bytes: baixados, total?).
pub fn download_asset(
    url: &str,
    dest: &Path,
    on_progress: &dyn Fn(u64, Option<u64>),
) -> anyhow::Result<()> {
    if let Some(parent) = dest.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let resp = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(600))
        .build()
        .get(url)
        .set("User-Agent", UA)
        .call()
        .map_err(|err| anyhow::anyhow!("download: {err}"))?;
    let total: Option<u64> = resp.header("Content-Length").and_then(|v| v.parse().ok());
    let mut reader = resp.into_reader();
    let mut file = std::fs::File::create(dest)?;
    let mut buf = [0u8; 65536];
    let mut done: u64 = 0;
    loop {
        use std::io::Read as _;
        let n = reader
            .read(&mut buf)
            .map_err(|err| anyhow::anyhow!("leitura: {err}"))?;
        if n == 0 {
            break;
        }
        use std::io::Write as _;
        file.write_all(&buf[..n])?;
        done += n as u64;
        on_progress(done, total);
    }
    use std::io::Write as _;
    file.flush()?;
    if let Some(total) = total
        && done < total
    {
        anyhow::bail!("download truncado ({done} de {total} bytes)");
    }
    if done == 0 {
        anyhow::bail!("download vazio");
    }
    Ok(())
}

/// Troca o AppImage em execução pelo baixado e devolve o caminho final.
/// Baixa para um temporário no mesmo diretório (mesmo filesystem) e
/// renomeia por cima: atômico, e o mount FUSE atual segue no inode
/// antigo até o restart.
pub fn apply_over_appimage(
    release: &UpdateRelease,
    on_progress: &dyn Fn(u64, Option<u64>),
) -> anyhow::Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt as _;
    let current =
        appimage_path().ok_or_else(|| anyhow::anyhow!("não está rodando como AppImage"))?;
    let dir = current
        .parent()
        .ok_or_else(|| anyhow::anyhow!("AppImage sem diretório"))?;
    let tmp = dir.join(format!(".{}.update", release.asset_name));
    let _ = std::fs::remove_file(&tmp);
    download_asset(&release.asset_url, &tmp, on_progress)?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    std::fs::rename(&tmp, &current)?;
    Ok(current)
}

/// Abre uma URL no navegador padrão (fallback fora do AppImage).
pub fn open_in_browser(url: &str) {
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compara_semver_com_v_e_componentes() {
        assert_eq!(compare_versions("0.3.0", "0.4.0"), Ordering::Less);
        assert_eq!(compare_versions("0.3.0", "v0.3.0"), Ordering::Equal);
        assert_eq!(compare_versions("v0.3.0", "0.3"), Ordering::Equal);
        assert_eq!(compare_versions("0.10.0", "0.9.9"), Ordering::Greater);
        assert_eq!(compare_versions("1.0", "1.0.1"), Ordering::Less);
        assert_eq!(compare_versions("0.4.0", "0.4.0"), Ordering::Equal);
    }

    #[test]
    fn escolhe_asset_exato_ou_generico_nunca_zsync() {
        let assets = vec![
            ("Klipp-0.4.0-x86_64.AppImage.zsync".to_string(), "z".to_string()),
            ("Klipp-0.4.0-x86_64.AppImage".to_string(), "a".to_string()),
        ];
        assert_eq!(pick_asset("0.4.0", &assets).unwrap().1, "a");
        // Sem o exato da arch: cai no genérico Klipp-*.
        let assets = vec![("Klipp-foo.AppImage".to_string(), "g".to_string())];
        assert_eq!(pick_asset("0.4.0", &assets).unwrap().1, "g");
        // Só zsync: nada.
        let assets = vec![("Klipp-0.4.0-x86_64.AppImage.zsync".to_string(), "z".to_string())];
        assert!(pick_asset("0.4.0", &assets).is_none());
    }

    #[test]
    fn latest_mais_nova_com_asset_vira_release() {
        let json = serde_json::json!({
            "tag_name": "v9.9.9",
            "html_url": "https://example.invalid/r",
            "assets": [
                {"name": "Klipp-9.9.9-x86_64.AppImage.zsync", "browser_download_url": "https://example.invalid/z"},
                {"name": "Klipp-9.9.9-x86_64.AppImage", "browser_download_url": "https://example.invalid/a"},
            ],
        });
        let rel = parse_latest(&json).unwrap().expect("devia detectar");
        assert_eq!(rel.version, "9.9.9");
        assert_eq!(rel.asset_url, "https://example.invalid/a");
    }

    #[test]
    fn mesma_versao_ou_mais_antiga_nao_e_update() {
        for tag in [
            current_version().to_string(),
            format!("v{}", current_version()),
            "v0.0.1".to_string(),
        ] {
            let json = serde_json::json!({"tag_name": tag, "assets": []});
            assert!(
                parse_latest(&json).unwrap().is_none(),
                "tag {tag} não devia ser update"
            );
        }
    }

    #[test]
    fn release_nova_sem_appimage_da_erro_explicito() {
        let json = serde_json::json!({"tag_name": "v9.9.9", "assets": []});
        assert!(parse_latest(&json).is_err());
    }
}
