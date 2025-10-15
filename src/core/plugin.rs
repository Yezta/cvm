use crate::core::traits::{Architecture, Platform, PluginMetadata, ToolPlugin};
use crate::error::{JcvmError, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Registry for managing tool plugins
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, Arc<dyn ToolPlugin>>>>,
    metadata: Arc<RwLock<HashMap<String, PluginMetadata>>>,
}

impl Clone for PluginRegistry {
    fn clone(&self) -> Self {
        Self {
            plugins: Arc::clone(&self.plugins),
            metadata: Arc::clone(&self.metadata),
        }
    }
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a plugin
    pub fn register(&self, plugin: Arc<dyn ToolPlugin>, metadata: PluginMetadata) -> Result<()> {
        let plugin_id = plugin.info().id.clone();

        // Verify metadata ID matches plugin ID
        if metadata.id != plugin_id {
            return Err(JcvmError::PluginError {
                plugin: plugin_id.clone(),
                message: format!(
                    "Metadata ID '{}' doesn't match plugin ID '{}'",
                    metadata.id, plugin_id
                ),
            });
        }

        let mut plugins = self.plugins.write().map_err(|e| JcvmError::PluginError {
            plugin: plugin_id.clone(),
            message: format!("Failed to acquire write lock: {}", e),
        })?;

        let mut metadata_map = self.metadata.write().map_err(|e| JcvmError::PluginError {
            plugin: plugin_id.clone(),
            message: format!("Failed to acquire write lock: {}", e),
        })?;

        if plugins.contains_key(&plugin_id) {
            return Err(JcvmError::PluginError {
                plugin: plugin_id,
                message: "Plugin already registered".to_string(),
            });
        }

        plugins.insert(plugin_id.clone(), plugin);
        metadata_map.insert(plugin_id, metadata);

        Ok(())
    }

    /// Get a plugin by ID
    pub fn get(&self, id: &str) -> Result<Arc<dyn ToolPlugin>> {
        let plugins = self.plugins.read().map_err(|e| JcvmError::PluginError {
            plugin: id.to_string(),
            message: format!("Failed to acquire read lock: {}", e),
        })?;

        plugins
            .get(id)
            .cloned()
            .ok_or_else(|| JcvmError::PluginNotFound(id.to_string()))
    }

    /// List all registered plugins
    pub fn list_plugins(&self) -> Result<Vec<String>> {
        let plugins = self.plugins.read().map_err(|e| JcvmError::PluginError {
            plugin: "registry".to_string(),
            message: format!("Failed to acquire read lock: {}", e),
        })?;

        Ok(plugins.keys().cloned().collect())
    }

    /// Get metadata for a plugin
    pub fn get_metadata(&self, id: &str) -> Result<PluginMetadata> {
        let metadata = self.metadata.read().map_err(|e| JcvmError::PluginError {
            plugin: id.to_string(),
            message: format!("Failed to acquire read lock: {}", e),
        })?;

        metadata
            .get(id)
            .cloned()
            .ok_or_else(|| JcvmError::PluginNotFound(id.to_string()))
    }

    /// List all plugin metadata
    pub fn list_metadata(&self) -> Result<Vec<PluginMetadata>> {
        let metadata = self.metadata.read().map_err(|e| JcvmError::PluginError {
            plugin: "registry".to_string(),
            message: format!("Failed to acquire read lock: {}", e),
        })?;

        Ok(metadata.values().cloned().collect())
    }

    /// Check if a plugin is registered
    pub fn has_plugin(&self, id: &str) -> bool {
        if let Ok(plugins) = self.plugins.read() {
            plugins.contains_key(id)
        } else {
            false
        }
    }

    /// Unregister a plugin (useful for dynamic plugins or testing)
    pub fn unregister(&self, id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().map_err(|e| JcvmError::PluginError {
            plugin: id.to_string(),
            message: format!("Failed to acquire write lock: {}", e),
        })?;

        let mut metadata = self.metadata.write().map_err(|e| JcvmError::PluginError {
            plugin: id.to_string(),
            message: format!("Failed to acquire write lock: {}", e),
        })?;

        plugins.remove(id);
        metadata.remove(id);

        Ok(())
    }

    /// Get plugins that support a specific platform/architecture
    pub fn get_plugins_for_platform(
        &self,
        platform: Platform,
        arch: Architecture,
    ) -> Result<Vec<String>> {
        let metadata = self.metadata.read().map_err(|e| JcvmError::PluginError {
            plugin: "registry".to_string(),
            message: format!("Failed to acquire read lock: {}", e),
        })?;

        Ok(metadata
            .values()
            .filter(|m| m.platforms.contains(&platform) && m.architectures.contains(&arch))
            .map(|m| m.id.clone())
            .collect())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = PluginRegistry::new();
        assert_eq!(registry.list_plugins().unwrap().len(), 0);
    }
}
