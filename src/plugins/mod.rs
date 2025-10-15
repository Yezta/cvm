pub mod java;
pub mod nodejs;
pub mod python;

// Future plugins will be added here:
// pub mod compass;

use crate::config::Config;
use crate::core::plugin::PluginRegistry;
use crate::error::Result;
use std::sync::Arc;

/// Register the built-in plugins (Java, Node.js, Python)
pub fn load_builtin_plugins(config: &Config) -> Result<PluginRegistry> {
    let registry = PluginRegistry::new();

    registry.register(
        Arc::new(java::JavaPlugin::new()),
        java::JavaPlugin::metadata(),
    )?;
    registry.register(
        Arc::new(nodejs::NodeJsPlugin::new()),
        nodejs::NodeJsPlugin::metadata(),
    )?;

    let python_plugin = python::PythonPlugin::new(
        config.tool_versions_dir("python"),
        config.tool_cache_dir("python"),
    );
    registry.register(Arc::new(python_plugin), python::PythonPlugin::metadata())?;

    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn builtin_plugins_are_registered() {
        let temp = tempdir().unwrap();
        let mut config = Config::default();
        config.jcvm_dir = temp.path().to_path_buf();
        config.versions_dir = config.jcvm_dir.join("versions");
        config.alias_dir = config.jcvm_dir.join("alias");
        config.cache_dir = config.jcvm_dir.join("cache");
        config.config_file = config.jcvm_dir.join("config.toml");
        std::fs::create_dir_all(&config.versions_dir).unwrap();
        std::fs::create_dir_all(&config.alias_dir).unwrap();
        std::fs::create_dir_all(&config.cache_dir).unwrap();

        let registry = load_builtin_plugins(&config).expect("registry");
        let plugins = registry.list_plugins().expect("list");
        assert!(plugins.contains(&"java".to_string()));
        assert!(plugins.contains(&"node".to_string()));
        assert!(plugins.contains(&"python".to_string()));
    }
}
