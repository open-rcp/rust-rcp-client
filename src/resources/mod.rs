use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Resource type
pub enum ResourceType {
    /// Image resource
    Image,

    /// Font resource
    Font,

    /// Shader resource
    Shader,

    /// Configuration resource
    Config,

    /// Other resource type
    Other,
}

/// Resource manager
pub struct ResourceManager {
    /// Base path for resources
    base_path: PathBuf,

    /// Cache of loaded resources
    cache: std::collections::HashMap<String, Vec<u8>>,
}

impl ResourceManager {
    /// Create a new resource manager with the given base path
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            cache: std::collections::HashMap::new(),
        }
    }

    /// Get the default resource path
    pub fn default_path() -> PathBuf {
        // Check for environment variable first
        if let Ok(path) = std::env::var("RCP_RESOURCE_PATH") {
            return PathBuf::from(path);
        }

        // Try to find the resources in common locations
        let candidates = vec![
            // Current directory
            PathBuf::from("resources"),
            // User configuration directory
            dirs::config_dir()
                .map(|p| p.join("rcp_client").join("resources"))
                .unwrap_or_else(|| PathBuf::from("resources")),
            // System-wide configuration directory
            #[cfg(target_os = "linux")]
            PathBuf::from("/etc/rcp_client/resources"),
            #[cfg(target_os = "macos")]
            PathBuf::from("/Library/Application Support/RCP Client/Resources"),
            #[cfg(target_os = "windows")]
            PathBuf::from(r"C:\Program Files\RCP Client\Resources"),
        ];

        // Return the first existing path, or the default
        for path in candidates {
            if path.exists() {
                return path;
            }
        }

        // Default to the current directory
        PathBuf::from("resources")
    }

    /// Load a resource from the given path
    pub async fn load<P: AsRef<Path>>(&mut self, path: P) -> Result<&[u8]> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        // Check if the resource is already in the cache
        if self.cache.contains_key(&path_str) {
            return Ok(&self.cache[&path_str]);
        }

        // Load the resource
        let full_path = self.base_path.join(path);
        let data = fs::read(&full_path)
            .await
            .with_context(|| format!("Failed to load resource: {:?}", full_path))?;

        // Add to the cache
        self.cache.insert(path_str.clone(), data);

        Ok(&self.cache[&path_str])
    }

    /// Load a resource of the given type
    pub async fn load_resource(
        &mut self,
        name: &str,
        resource_type: ResourceType,
    ) -> Result<&[u8]> {
        let path = match resource_type {
            ResourceType::Image => format!("images/{}", name),
            ResourceType::Font => format!("fonts/{}", name),
            ResourceType::Shader => format!("shaders/{}", name),
            ResourceType::Config => format!("config/{}", name),
            ResourceType::Other => name.to_string(),
        };

        self.load(path).await
    }
}
