//! Selector trait and factory

use crate::types::{MenuItem, SelectorType};
use super::fzf::FzfSelector;
use super::dialoguer_selector::DialoguerSelector;

/// Selector enum for interactive menus
pub enum Selector {
    Fzf(FzfSelector),
    Dialoguer(DialoguerSelector),
}

impl Selector {
    /// Select an item from the menu
    pub fn select<T: Clone + Send + 'static>(
        &self,
        items: &[MenuItem<T>],
        prompt: &str,
    ) -> Option<T> {
        match self {
            Selector::Fzf(s) => s.select(items, prompt),
            Selector::Dialoguer(s) => s.select(items, prompt),
        }
    }

    /// Check if selector is available
    #[allow(dead_code)]
    pub fn is_available(&self) -> bool {
        match self {
            Selector::Fzf(s) => s.is_available(),
            Selector::Dialoguer(s) => s.is_available(),
        }
    }
}

/// Create a selector based on type
pub fn create_selector(selector_type: SelectorType) -> Selector {
    match selector_type {
        SelectorType::Fzf => {
            let fzf = FzfSelector::new();
            if fzf.is_available() {
                return Selector::Fzf(fzf);
            }
            // Fall back to dialoguer
            Selector::Dialoguer(DialoguerSelector::new())
        }
        SelectorType::Rofi => {
            // TODO: Implement rofi selector
            Selector::Dialoguer(DialoguerSelector::new())
        }
        SelectorType::Dialoguer => Selector::Dialoguer(DialoguerSelector::new()),
    }
}

/// Detect the best available selector
pub fn detect_selector() -> SelectorType {
    let fzf = FzfSelector::new();
    if fzf.is_available() {
        return SelectorType::Fzf;
    }
    SelectorType::Dialoguer
}
