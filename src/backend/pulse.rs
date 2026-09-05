/// Wrapper fino sobre `pactl` (PipeWire/PulseAudio).
pub fn run_pactl(args: &[&str]) -> anyhow::Result<String> {
    let out = std::process::Command::new("pactl")
        .args(args)
        .output()
        .map_err(|e| anyhow::anyhow!("pactl: {e}"))?;
    if !out.status.success() {
        return Err(anyhow::anyhow!(
            "pactl {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Microfones reais: `(nome, descrição)`, sem monitores nem mic virtual.
pub fn list_sources() -> Vec<(String, String)> {
    let Ok(out) = run_pactl(&["list", "sources"]) else {
        return vec![];
    };
    let mut sources = vec![];
    let (mut name, mut desc) = (None::<String>, None::<String>);
    let flush = |name: &mut Option<String>, desc: &mut Option<String>, out: &mut Vec<(String, String)>| {
        if let (Some(n), Some(d)) = (name.take(), desc.take()) {
            if !n.ends_with(".monitor") && n != super::audio_graph::VIRTUAL_MIC {
                out.push((n, d));
            }
        }
    };
    for line in out.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("Source #") {
            let _ = rest;
            flush(&mut name, &mut desc, &mut sources);
        } else if let Some(rest) = t.strip_prefix("Name: ") {
            name = Some(rest.to_string());
        } else if let Some(rest) = t.strip_prefix("Description: ") {
            desc = Some(rest.to_string());
        }
    }
    flush(&mut name, &mut desc, &mut sources);
    sources
}
