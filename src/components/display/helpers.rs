/// Führt qdbus-qt6 mit Fallback auf qdbus aus.
/// Gibt Ok(()) oder Err(String) zurück.
pub(crate) async fn qdbus_ausfuehren(args: Vec<String>) -> Result<(), String> {
    let args_clone = args.clone();
    let result = tokio::task::spawn_blocking(move || {
        let status = std::process::Command::new("qdbus-qt6")
            .args(&args_clone)
            .status();
        match status {
            Ok(s) => Ok(("qdbus-qt6", s)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Fallback auf qdbus
                std::process::Command::new("qdbus")
                    .args(&args_clone)
                    .status()
                    .map(|s| ("qdbus", s))
            }
            Err(e) => Err(e),
        }
    })
    .await;

    match result {
        Ok(Ok((_, status))) if status.success() => Ok(()),
        Ok(Ok((cmd, status))) => Err(format!(
            "{cmd} fehlgeschlagen mit Exit-Code: {}",
            status.code().unwrap_or(-1)
        )),
        Ok(Err(e)) => Err(format!("qdbus starten fehlgeschlagen: {e}")),
        Err(e) => Err(format!("spawn_blocking fehlgeschlagen: {e}")),
    }
}
