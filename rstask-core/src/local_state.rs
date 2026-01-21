// Local state management for context and ID mapping
use crate::Result;
use crate::error::RstaskError;
use crate::query::Query;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub type IdsMap = HashMap<String, i32>;

/// Local state including context
#[derive(Debug, Clone)]
pub struct LocalState {
    pub context: Query,
    state_file: PathBuf,
}

impl LocalState {
    /// Load state from file or create default
    pub fn load(state_file: &Path) -> Self {
        let context = if let Ok(data) = std::fs::read(state_file) {
            bincode::deserialize(&data).unwrap_or_default()
        } else {
            Query::default()
        };

        LocalState {
            context,
            state_file: state_file.to_path_buf(),
        }
    }

    /// Set the context
    pub fn set_context(&mut self, context: Query) -> Result<()> {
        if !context.ids.is_empty() {
            return Err(RstaskError::Parse("context cannot contain IDs".to_string()));
        }

        if !context.text.is_empty() {
            return Err(RstaskError::Parse(
                "context cannot contain text".to_string(),
            ));
        }

        self.context = context;
        Ok(())
    }

    /// Get the current context
    pub fn get_context(&self) -> &Query {
        &self.context
    }

    /// Save state to file
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.state_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = bincode::serialize(&self.context)?;
        std::fs::write(&self.state_file, data)?;
        Ok(())
    }
}

pub fn load_ids(ids_file: &Path) -> IdsMap {
    if let Ok(data) = std::fs::read(ids_file) {
        bincode::deserialize(&data).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

pub fn save_ids(ids_file: &Path, ids: &IdsMap) -> Result<()> {
    if let Some(parent) = ids_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = bincode::serialize(ids)?;
    std::fs::write(ids_file, data)?;
    Ok(())
}

pub fn load_state(state_file: &Path) -> Option<Query> {
    if let Ok(data) = std::fs::read(state_file) {
        bincode::deserialize(&data).ok()
    } else {
        None
    }
}

pub fn save_state(state_file: &Path, query: &Query) -> Result<()> {
    if let Some(parent) = state_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = bincode::serialize(query)?;
    std::fs::write(state_file, data)?;
    Ok(())
}
