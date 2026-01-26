pub fn setup_logger(log_file: &std::path::Path) {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(
            fern::log_file(log_file)
                .unwrap_or_else(|e| panic!("Failed to create log file at {:?}: {}", log_file, e)),
        )
        .apply()
        .unwrap_or_else(|e| panic!("Failed to apply logger configuration: {}", e));
}
