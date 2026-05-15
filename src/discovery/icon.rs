use std::path::PathBuf;

pub fn resolve_icon(icon_name: &Option<String>) -> Option<String> {
    let name = icon_name.as_ref()?;

    if name.starts_with('/') {
        if std::path::Path::new(name).exists() {
            return Some(name.clone());
        }
        return None;
    }

    let theme = current_icon_theme();

    let icon_dirs: Vec<PathBuf> = vec![
        PathBuf::from("/usr/share/icons"),
        dirs::home_dir()
            .map(|h| h.join(".local/share/icons"))
            .unwrap_or_default(),
    ];

    for icon_dir in &icon_dirs {
        if !icon_dir.exists() {
            continue;
        }

        for theme_name in theme_chain(icon_dir, &theme) {
            let theme_dir = icon_dir.join(&theme_name);
            if !theme_dir.exists() {
                continue;
            }

            // Prefer scalable SVG
            let scalable = theme_dir.join(format!("scalable/apps/{name}.svg"));
            if scalable.exists() {
                return Some(scalable.to_string_lossy().into_owned());
            }

            // Try PNG sizes (largest first)
            for size in ["512x512", "256x256", "192x192", "128x128", "96x96", "64x64", "48x48", "32x32"] {
                let png = theme_dir.join(format!("{size}/apps/{name}.png"));
                if png.exists() {
                    return Some(png.to_string_lossy().into_owned());
                }
                let svg = theme_dir.join(format!("{size}/apps/{name}.svg"));
                if svg.exists() {
                    return Some(svg.to_string_lossy().into_owned());
                }
            }
        }
    }

    // Fallback: /usr/share/pixmaps/
    for ext in ["svg", "png"] {
        let path = PathBuf::from(format!("/usr/share/pixmaps/{name}.{ext}"));
        if path.exists() {
            return Some(path.to_string_lossy().into_owned());
        }
    }

    None
}

fn current_icon_theme() -> String {
    if let Ok(output) = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "icon-theme"])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            let s = s.trim().trim_matches('\'').trim_matches('"');
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    "hicolor".to_string()
}

fn theme_chain(icon_dir: &std::path::Path, theme: &str) -> Vec<String> {
    let mut chain = vec![theme.to_string()];

    let index = icon_dir.join(theme).join("index.theme");
    if let Ok(content) = std::fs::read_to_string(&index) {
        for line in content.lines() {
            if let Some(inherits) = line.strip_prefix("Inherits=") {
                for parent in inherits.split(',') {
                    let parent = parent.trim();
                    if !parent.is_empty() && parent != theme {
                        chain.push(parent.to_string());
                    }
                }
            }
        }
    }

    if !chain.contains(&"hicolor".to_string()) {
        chain.push("hicolor".to_string());
    }

    chain
}
