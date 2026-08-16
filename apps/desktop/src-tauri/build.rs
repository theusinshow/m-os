fn main() {
    // O `tauri_build` gera um `resource.rc` que aponta para `icons/icon.ico`,
    // mas nao declara dependencia nenhuma sobre ele. Trocar o icone, sozinho,
    // nao invalida nada: o Cargo reaproveita o `.res` ja compilado e o
    // executavel sai com o icone antigo. O build passa, o instalador e
    // produzido, nenhum aviso aparece — so olhando os bytes do binario da para
    // notar.
    //
    // Localmente isso se resolvia apagando o diretorio de build a mao, o que
    // exigia lembrar. Na CI nao havia como lembrar: o `Swatinem/rust-cache`
    // restaura o `target/`, entao a release sairia com o icone da versao
    // anterior.
    println!("cargo:rerun-if-changed=icons/icon.ico");
    tauri_build::build()
}
