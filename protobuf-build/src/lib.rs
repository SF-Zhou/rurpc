use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

mod backend;

const DEFAULT_CONFIG_FILE: &str = "proto.yml";

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
struct Config {
    proto_path: PathBuf,
    output_filename: PathBuf,
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    includes: Vec<PathBuf>,
}

struct Builder {
    proto_path: PathBuf,
    output_filename: PathBuf,
    pilota_builder:
        pilota_build::Builder<backend::MakeBackend, pilota_build::parser::ProtobufParser>,
}

impl Builder {
    fn new(config: Config) -> Self {
        Builder {
            proto_path: config.proto_path,
            output_filename: config.output_filename,
            pilota_builder: pilota_build::Builder::protobuf()
                .with_backend(backend::MakeBackend)
                .include_dirs(config.includes),
        }
    }

    fn write(self) -> anyhow::Result<()> {
        let out_dir = PathBuf::from_str(&std::env::var("OUT_DIR")?)?;
        if !out_dir.exists() {
            std::fs::create_dir_all(&out_dir)?;
        }

        self.pilota_builder.compile_with_config(
            vec![pilota_build::IdlService::from_path(self.proto_path)],
            pilota_build::Output::File(out_dir.join(self.output_filename)),
        );

        Ok(())
    }
}

pub fn generate() -> anyhow::Result<()> {
    let config_file_path = PathBuf::from(DEFAULT_CONFIG_FILE);
    println!("cargo:rerun-if-changed={}", config_file_path.display());

    let config_file = std::fs::File::open(&config_file_path)?;
    let configs: Vec<Config> = serde_yaml::from_reader(&config_file)?;

    for config in configs {
        std::fs::File::open(&config.proto_path)
            .map_err(|e| anyhow!("open {} failed: {}", config.proto_path.display(), e))?;
        println!("cargo:rerun-if-changed={}", config.proto_path.display());
        Builder::new(config).write()?;
    }

    Ok(())
}
