use crate::plugin_engine::{LayoutConfig, PluginConfig};

/// Look up the layout for the given scene ID from loaded plugins.
pub fn get_layout(scene_id: &str, plugins: &[PluginConfig]) -> Option<LayoutConfig> {
    plugins
        .iter()
        .find(|p| p.id == scene_id)
        .map(|p| p.layout.clone())
}
