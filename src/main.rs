use shazam_cli::cli;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let mode = cli_command_kit::Mode::from_env();
    let mut command = match cli::command(mode) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ERR: {}", cli::safe_error(&e));
            return 1.into();
        }
    };
    let matches = match command.try_get_matches_from_mut(std::env::args_os()) {
        Ok(m) => m,
        Err(e) => {
            let status = e.exit_code() as u8;
            let _ = e.print();
            return status.into();
        }
    };
    let cancel = CancellationToken::new();
    let signal = cancel.clone();
    let listener = tokio::spawn(async move {
        #[cfg(unix)]
        {
            let mut terminate =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
            tokio::select! {r=tokio::signal::ctrl_c()=>r?,_=terminate.recv()=>{}}
        }
        #[cfg(not(unix))]
        tokio::signal::ctrl_c().await?;
        signal.cancel();
        std::io::Result::Ok(())
    });
    let result = cli::execute(&matches, mode, &cancel).await;
    listener.abort();
    match result {
        Ok(()) => 0.into(),
        Err(e) => {
            eprintln!("ERR: {}", cli::safe_error(&e));
            cli::exit_code(&e).into()
        }
    }
}
