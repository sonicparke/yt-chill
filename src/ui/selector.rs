//! Selector trait and factory

use super::dialoguer_selector::DialoguerSelector;
use super::fzf::FzfSelector;
use crate::types::{MenuItem, SelectorType};

/// Object-safe menu selector (index into labels).
pub trait MenuSelector: Send {
    fn select_index(&self, labels: &[String], prompt: &str) -> Option<usize>;
}

struct FzfMenuSelector(FzfSelector);

impl MenuSelector for FzfMenuSelector {
    fn select_index(&self, labels: &[String], prompt: &str) -> Option<usize> {
        let items: Vec<MenuItem<usize>> = labels
            .iter()
            .enumerate()
            .map(|(i, label)| MenuItem {
                label: label.clone(),
                value: i,
            })
            .collect();
        self.0.select(&items, prompt)
    }
}

struct DialoguerMenuSelector(DialoguerSelector);

impl MenuSelector for DialoguerMenuSelector {
    fn select_index(&self, labels: &[String], prompt: &str) -> Option<usize> {
        let items: Vec<MenuItem<usize>> = labels
            .iter()
            .enumerate()
            .map(|(i, label)| MenuItem {
                label: label.clone(),
                value: i,
            })
            .collect();
        self.0.select(&items, prompt)
    }
}

/// Interactive menu backed by `fzf` or dialoguer.
pub struct DynSelector(Box<dyn MenuSelector>);

impl DynSelector {
    pub fn select<T: Clone + Send + 'static>(
        &self,
        items: &[MenuItem<T>],
        prompt: &str,
    ) -> Option<T> {
        if items.is_empty() {
            return None;
        }
        let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
        let idx = self.0.select_index(&labels, prompt)?;
        items.get(idx).map(|m| m.value.clone())
    }
}

/// Create a selector based on type
pub fn create_selector(selector_type: SelectorType) -> DynSelector {
    match selector_type {
        SelectorType::Fzf => {
            let fzf = FzfSelector::new();
            if fzf.is_available() {
                return DynSelector(Box::new(FzfMenuSelector(fzf)));
            }
            DynSelector(Box::new(DialoguerMenuSelector(DialoguerSelector::new())))
        }
        SelectorType::Dialoguer => {
            DynSelector(Box::new(DialoguerMenuSelector(DialoguerSelector::new())))
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MenuItem;

    #[test]
    fn dialoguer_dyn_select_empty_menu_returns_none() {
        let s = create_selector(SelectorType::Dialoguer);
        let items: Vec<MenuItem<u32>> = vec![];
        assert_eq!(s.select(&items, "prompt"), None);
    }
}
