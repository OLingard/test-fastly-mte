pub mod logger {
    use chrono::prelude::Utc;
    use log::{Log, Record};
    use serde_json::json;

    pub struct JsonLogger<L: Log> {
        inner: L,
        id: String,
    }

    impl<L: Log> JsonLogger<L> {
        pub fn new(inner: L, id: String) -> Self {
            return Self { inner, id };
        }
    }

    impl<L: Log> Log for JsonLogger<L> {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            self.inner.enabled(metadata)
        }

        fn log(&self, record: &Record) {
            if self.enabled(record.metadata()) {
                let msg = json!({
                    "id": &self.id,
                    "timestamp": Utc::now().timestamp(),
                    "level": record.metadata().level().to_string(),
                    "message": record.args().to_string(),
                });

                self.inner.log(
                    &Record::builder()
                        .args(format_args!("{}", msg))
                        .file(record.file())
                        .line(record.line())
                        .module_path(record.module_path())
                        .target(record.target())
                        .level(record.level())
                        .build(),
                );
            }
        }

        fn flush(&self) {
            self.inner.flush();
        }
    }
}
