/// Führt ein Programm in einem Blocking-Thread aus und gibt `Result<(), String>` zurück.
pub(crate) async fn run_command_blocking(program: &str, args: &[&str]) -> Result<(), String> {
    let program_name = program.to_string();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    let result = tokio::task::spawn_blocking(move || {
        std::process::Command::new(&program_name)
            .args(&args)
            .status()
    })
    .await;

    match result {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(Ok(status)) => Err(format!(
            "{program} fehlgeschlagen mit Exit-Code: {}",
            status.code().unwrap_or(-1)
        )),
        Ok(Err(e)) => Err(format!("{program} starten fehlgeschlagen: {e}")),
        Err(e) => Err(format!("spawn_blocking fehlgeschlagen: {e}")),
    }
}

/// Führt einen Shell-Befehl via `pkexec sh -c` aus.
pub(crate) async fn pkexec_shell(command: &str) -> Result<(), String> {
    run_command_blocking("pkexec", &["sh", "-c", command]).await
}
