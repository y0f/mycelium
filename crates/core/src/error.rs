use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyceliumError {
    #[error("GPU initialization failed: {0}")]
    GpuInit(String),

    #[error("Shader compilation failed: {0}")]
    ShaderCompilation(String),

    #[error("Audio device error: {0}")]
    AudioDevice(String),

    #[error("Config parse error: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("Config serialize error: {0}")]
    ConfigSerialize(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Script error: {0}")]
    Script(String),

    #[error("Plugin load error: {0}")]
    PluginLoad(String),
}
