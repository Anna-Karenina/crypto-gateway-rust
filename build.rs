fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(false) // Клиенты не нужны, только сервер
        .compile(
            &[
                "proto/common.proto",
                "proto/wallet.proto",
                "proto/transfer.proto",
            ],
            &["proto"],
        )?;

    // Сообщаем cargo пересобирать при изменении .proto файлов
    println!("cargo:rerun-if-changed=proto/");

    Ok(())
}
