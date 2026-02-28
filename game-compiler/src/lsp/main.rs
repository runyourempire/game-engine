use tower_lsp::{LspService, Server};

// The LSP modules live inside the library crate under `game_compiler::lsp`.
// This binary just boots the server over stdin/stdout.

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) =
        LspService::new(|client| game_compiler::lsp::backend::GameBackend::new(client));

    Server::new(stdin, stdout, socket).serve(service).await;
}
