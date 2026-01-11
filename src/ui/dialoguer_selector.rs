//! dialoguer selector implementation (fallback)

use crate::types::MenuItem;
use dialoguer::{theme::ColorfulTheme, Select};

pub struct DialoguerSelector;

impl DialoguerSelector {
    pub fn new() -> Self {
        Self
    }

    pub fn select<T: Clone + Send + 'static>(
        &self,
        items: &[MenuItem<T>],
        prompt: &str,
    ) -> Option<T> {
        if items.is_empty() {
            return None;
        }

        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .items(&labels)
            .default(0)
            .interact_opt()
            .ok()
            .flatten()?;

        items.get(selection).map(|item| item.value.clone())
    }

    #[allow(dead_code)]
    pub fn is_available(&self) -> bool {
        true  // Always available as fallback
    }
}
