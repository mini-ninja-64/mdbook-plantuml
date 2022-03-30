use mdbook::preprocess::PreprocessorContext;
use serde::{Deserialize, Serialize};

/// The configuration options available with this backend.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "kebab-case")]
pub struct PlantUMLConfig {
    /// By default it is assumed plantuml.jar is on the path
    /// Use plantuml_cmd if it is not on the path, or if you
    /// have some additional parameters.
    pub plantuml_cmd: Option<String>,
    /// PlantUML images become clickable for zoom by setting this flag to `true`.
    /// This is convenient for large diagrams which are hard to see in the book.
    /// The default value is `false`.
    pub clickable_img: bool,
    /// Defines whether logging should be enabled
    pub logging_enabled: bool,
    /// This allows users to override the default logger by providing a
    /// log4rs yaml file path
    pub logging_config: Option<String>,
}

pub fn get_plantuml_config(ctx: &PreprocessorContext) -> PlantUMLConfig {
    ctx.config
        .get("preprocessor.plantuml")
        .and_then(|raw| {
            raw.clone()
                .try_into()
                .map_err(|e| {
                    log::warn!(
                        "Failed to get config from book.toml, using default configuration ({}).",
                        e
                    );
                    e
                })
                .ok()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn default() {
        let cfg = PlantUMLConfig::default();
        assert_eq!(cfg.plantuml_cmd, None);
    }
}
