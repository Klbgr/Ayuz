use crate::services::commands::run_command_blocking;
use crate::services::config::AppConfig;
use rust_i18n::t;

pub(crate) const DISPLAY_NAME: &str = "eDP-1";

const SRGB_ICM: &[u8] = include_bytes!("../../../assets/icm/ASUS_sRGB.icm");
const DCIP3_ICM: &[u8] = include_bytes!("../../../assets/icm/ASUS_DCIP3.icm");
const DISPLAYP3_ICM: &[u8] = include_bytes!("../../../assets/icm/ASUS_DisplayP3.icm");

pub(crate) async fn setup_icm_profiles() -> Result<std::path::PathBuf, String> {
    let basis = AppConfig::config_dir()
        .ok_or_else(|| t!("error_config_dir").to_string())?
        .join("icm");

    let basis_clone = basis.clone();
    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(&basis_clone)
            .map_err(|e| t!("error_icm_dir_create", error = e.to_string()).to_string())?;

        for (name, data) in [
            ("ASUS_sRGB.icm", SRGB_ICM),
            ("ASUS_DCIP3.icm", DCIP3_ICM),
            ("ASUS_DisplayP3.icm", DISPLAYP3_ICM),
        ] {
            let pfad = basis_clone.join(name);
            if !pfad.exists() {
                std::fs::write(&pfad, data).map_err(|e| {
                    t!("error_icm_write", name = name, error = e.to_string()).to_string()
                })?;
            }
        }
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| t!("error_spawn_blocking", error = e.to_string()).to_string())??;

    Ok(basis)
}

pub(crate) async fn icm_profil_reset() -> Result<(), String> {
    let arg = format!("output.{}.colorProfileSource.EDID", DISPLAY_NAME);
    run_command_blocking("kscreen-doctor", &[&arg]).await
}

pub(crate) async fn icm_profil_anwenden(
    dateiname: &str,
    basis_pfad: &std::path::Path,
) -> Result<(), String> {
    let arg = format!(
        "output.{}.iccprofile.{}",
        DISPLAY_NAME,
        basis_pfad.join(dateiname).display()
    );
    run_command_blocking("kscreen-doctor", &[&arg]).await
}

/// Fallback: versucht qdbus-qt6, dann qdbus.
pub(crate) async fn qdbus_ausfuehren(args: Vec<String>) -> Result<(), String> {
    let result = tokio::task::spawn_blocking(move || {
        let status = std::process::Command::new("qdbus-qt6").args(&args).status();
        match status {
            Ok(s) => Ok(("qdbus-qt6", s)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                std::process::Command::new("qdbus")
                    .args(&args)
                    .status()
                    .map(|s| ("qdbus", s))
            }
            Err(e) => Err(e),
        }
    })
    .await;

    match result {
        Ok(Ok((_, status))) if status.success() => Ok(()),
        Ok(Ok((cmd, status))) => Err(t!(
            "error_cmd_exit_code",
            cmd = cmd,
            code = status.code().unwrap_or(-1).to_string()
        )
        .to_string()),
        Ok(Err(e)) => Err(t!("error_qdbus_start", error = e.to_string()).to_string()),
        Err(e) => Err(t!("error_spawn_blocking", error = e.to_string()).to_string()),
    }
}

